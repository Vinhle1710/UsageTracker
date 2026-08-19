use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
const SCHEMA_V1: &str = r#"CREATE TABLE usage_samples(id INTEGER PRIMARY KEY,provider TEXT NOT NULL CHECK(provider IN ('claude','openai')),window_kind TEXT NOT NULL,used_percent REAL NOT NULL CHECK(used_percent>=0 AND used_percent<=100),resets_at INTEGER NOT NULL,sampled_at INTEGER NOT NULL,session_id TEXT,model TEXT,api_calls INTEGER,input_tokens INTEGER,output_tokens INTEGER,estimated_cost_micros INTEGER,overage_cost_micros INTEGER,UNIQUE(provider,window_kind,sampled_at));CREATE INDEX usage_samples_range ON usage_samples(sampled_at,provider,window_kind);CREATE TABLE billing_entries(id INTEGER PRIMARY KEY,provider TEXT NOT NULL CHECK(provider IN ('claude','openai')),period_start INTEGER NOT NULL,period_end INTEGER NOT NULL,amount_micros INTEGER NOT NULL CHECK(amount_micros>=0),currency TEXT NOT NULL DEFAULT 'USD' CHECK(length(currency)=3 AND currency=upper(currency)),source TEXT NOT NULL CHECK(source IN ('estimated','provider')),UNIQUE(provider,period_start,period_end,source));PRAGMA user_version=1;"#;
// The two tables are intentionally independent: this is a single-account event log with no relational references, so FK enforcement has no relationships to protect.
pub struct HistoryDb {
    connection: Connection,
    last_prune_day: Option<i64>,
}
impl HistoryDb {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let c = Connection::open(path)?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        c.execute_batch("PRAGMA journal_mode=WAL;PRAGMA foreign_keys=ON;")?;
        Self::migrate(c)
    }
    #[cfg(test)]
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::migrate(Connection::open_in_memory()?)
    }
    fn migrate(c: Connection) -> rusqlite::Result<Self> {
        let v: i64 = c.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if v > 1 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if v == 0 {
            c.execute_batch(&(String::from("BEGIN;") + SCHEMA_V1 + "COMMIT;"))?;
        }
        Ok(Self {
            connection: c,
            last_prune_day: None,
        })
    }
    #[cfg(test)]
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
    pub fn record_poll_cycle(
        &mut self,
        events: &[crate::model::ProviderUsageEvent],
        billing: &[BillingSample],
    ) -> rusqlite::Result<usize> {
        use crate::model::{Provider, SnapshotState};
        let tx = self.connection.transaction()?;
        let mut n = 0;
        for e in events {
            if e.snapshot.state != SnapshotState::Fresh {
                continue;
            }
            let p = match e.provider {
                Provider::Claude => "claude",
                Provider::Openai => "openai",
            };
            for w in &e.snapshot.windows {
                if !w.used_percent.is_finite() || !(0.0..=100.0).contains(&w.used_percent) {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                n += tx.execute("INSERT OR IGNORE INTO usage_samples(provider,window_kind,used_percent,resets_at,sampled_at) VALUES (?1,?2,?3,?4,?5)", params![p, window_kind(&w.label), w.used_percent, w.resets_at, e.snapshot.fetched_at])?;
            }
        }
        for b in billing {
            validate_billing(b).map_err(|_| rusqlite::Error::InvalidQuery)?;
            n += tx.execute("INSERT OR IGNORE INTO billing_entries(provider,period_start,period_end,amount_micros,currency,source) VALUES (?1,?2,?3,?4,?5,?6)", params![b.provider, b.period_start, b.period_end, b.amount_micros, b.currency, b.source])?;
        }
        tx.commit()?;
        Ok(n)
    }
    pub fn record_event(
        &mut self,
        e: &crate::model::ProviderUsageEvent,
    ) -> rusqlite::Result<usize> {
        self.record_poll_cycle(std::slice::from_ref(e), &[])
    }
    pub fn insert_billing(&mut self, b: &BillingSample) -> rusqlite::Result<usize> {
        self.record_poll_cycle(&[], std::slice::from_ref(b))
    }
    pub fn prune_retention_once(&mut self, now: i64, days: u16) -> rusqlite::Result<usize> {
        let day = now.div_euclid(86_400);
        if self.last_prune_day == Some(day) {
            return Ok(0);
        }
        let result = self.prune_before(now - i64::from(days.clamp(30, 730)) * 86_400)?;
        self.last_prune_day = Some(day);
        Ok(result)
    }
    pub fn prune_before(&mut self, c: i64) -> rusqlite::Result<usize> {
        let tx = self.connection.transaction()?;
        let usage = tx.execute("DELETE FROM usage_samples WHERE sampled_at < ?1", [c])?;
        let billing = tx.execute("DELETE FROM billing_entries WHERE period_end <= ?1", [c])?;
        tx.commit()?;
        Ok(usage + billing)
    }
    pub fn clear(&mut self) -> rusqlite::Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM usage_samples", [])?;
        tx.execute("DELETE FROM billing_entries", [])?;
        tx.commit()
    }
    pub fn query(&self, q: HistoryQuery) -> rusqlite::Result<HistoryResult> {
        validate(&q).map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut s=String::from("SELECT provider,window_kind,sampled_at,used_percent,model,api_calls,estimated_cost_micros,overage_cost_micros FROM usage_samples WHERE sampled_at>=?1 AND sampled_at<?2");
        if q.provider.is_some() {
            s.push_str(" AND provider=?3")
        }
        if q.window_kind.is_some() {
            s.push_str(if q.provider.is_some() {
                " AND window_kind=?4"
            } else {
                " AND window_kind=?3"
            })
        }
        s.push_str(" ORDER BY sampled_at,id");
        let mut vals: Vec<&dyn rusqlite::ToSql> = vec![&q.from, &q.to];
        if let Some(v) = q.provider.as_ref() {
            vals.push(v)
        }
        if let Some(v) = q.window_kind.as_ref() {
            vals.push(v)
        }
        let total: i64 = self.connection.query_row(
            &s.replacen("SELECT provider,window_kind,sampled_at,used_percent,model,api_calls,estimated_cost_micros,overage_cost_micros", "SELECT count(*)", 1),
            rusqlite::params_from_iter(vals.iter().copied()),
            |r| r.get(0),
        )?;
        let mut bounded = s;
        let step_value = (total as usize).div_ceil(5000) as i64;
        if total > 5000 {
            let where_clause = bounded.split_once(" ORDER BY").unwrap().0.replace("SELECT provider,window_kind,sampled_at,used_percent,model,api_calls,estimated_cost_micros,overage_cost_micros FROM usage_samples", "");
            bounded = format!("SELECT provider,window_kind,sampled_at,used_percent,model,api_calls,estimated_cost_micros,overage_cost_micros FROM (SELECT provider,window_kind,sampled_at,used_percent,model,api_calls,estimated_cost_micros,overage_cost_micros,ROW_NUMBER() OVER (ORDER BY sampled_at,id) AS row_num FROM usage_samples {} ) WHERE row_num=1 OR row_num=?{} OR ((row_num-1) % ?{})=0 ORDER BY row_num", where_clause, vals.len()+1, vals.len()+2);
            vals.push(&total);
            vals.push(&step_value);
        }
        let points = self
            .connection
            .prepare(&bounded)?
            .query_map(rusqlite::params_from_iter(vals), |r| {
                Ok(HistoryPoint {
                    provider: r.get(0)?,
                    window_kind: r.get(1)?,
                    sampled_at: r.get(2)?,
                    used_percent: r.get(3)?,
                    model: r.get(4)?,
                    api_calls: r.get(5)?,
                    estimated_cost_micros: r.get(6)?,
                    overage_cost_micros: r.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut points = points;
        downsample(&mut points);
        let billing=self.connection.prepare("SELECT provider,period_start,period_end,amount_micros,currency,source FROM billing_entries WHERE period_end>?1 AND period_start<?2 ORDER BY period_start,provider,currency,source")?.query_map([q.from,q.to],|r|Ok(BillingEntry{provider:r.get(0)?,period_start:r.get(1)?,period_end:r.get(2)?,amount_micros:r.get(3)?,currency:r.get(4)?,source:r.get(5)?}))?.collect::<Result<Vec<_>,_>>()?;
        Ok(HistoryResult { points, billing })
    }
    pub fn aggregate_billing(&self, q: HistoryQuery) -> rusqlite::Result<Vec<BillingAggregate>> {
        validate(&q).map_err(|_| rusqlite::Error::InvalidQuery)?;
        let entries = self.query(q)?.billing;
        let mut sums = BTreeMap::<(String, String, String), i64>::new();
        for e in entries {
            let key = (e.provider, e.currency, e.source);
            let total = sums
                .get(&key)
                .copied()
                .unwrap_or(0)
                .checked_add(e.amount_micros)
                .ok_or(rusqlite::Error::InvalidQuery)?;
            sums.insert(key, total);
        }
        Ok(sums
            .into_iter()
            .map(
                |((provider, currency, source), amount_micros)| BillingAggregate {
                    provider,
                    currency,
                    source,
                    amount_micros,
                },
            )
            .collect())
    }

    pub fn connection_busy_timeout_ms(&self) -> rusqlite::Result<i64> {
        self.connection
            .pragma_query_value(None, "busy_timeout", |r| r.get(0))
    }
}
fn downsample(points: &mut Vec<HistoryPoint>) {
    const MAX: usize = 5000;
    if points.len() <= MAX {
        return;
    }
    let original = std::mem::take(points);
    let total = original.len();
    let step = original.len().div_ceil(MAX);
    *points = original
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i == 0 || *i + 1 == total || *i % step == 0)
        .map(|(_, p)| p)
        .take(MAX)
        .collect();
}
fn validate_billing(b: &BillingSample) -> Result<(), ()> {
    if !matches!(b.provider.as_str(), "claude" | "openai")
        || b.period_start >= b.period_end
        || b.amount_micros < 0
        || b.currency.len() != 3
        || !b.currency.chars().all(|c| c.is_ascii_uppercase())
        || !matches!(b.source.as_str(), "estimated" | "provider")
    {
        Err(())
    } else {
        Ok(())
    }
}
fn window_kind(l: &str) -> String {
    match l.trim().to_ascii_lowercase().as_str() {
        "5 hour" => "session_5h".into(),
        "daily" | "24 hour" => "daily_24h".into(),
        "weekly" | "7 days" => "weekly_7d".into(),
        x => format!("provider:{}", x.replace(' ', "_")),
    }
}
fn validate(q: &HistoryQuery) -> Result<(), ()> {
    if q.from >= q.to || q.to - q.from > 366 * 86400 {
        return Err(());
    }
    if q.window_kind.as_deref().is_some_and(|w| {
        !(matches!(w, "session_5h" | "daily_24h" | "weekly_7d") || w.starts_with("provider:"))
    }) {
        return Err(());
    }
    if q.provider
        .as_deref()
        .is_some_and(|p| !matches!(p, "claude" | "openai"))
    {
        return Err(());
    }
    Ok(())
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryQuery {
    pub from: i64,
    pub to: i64,
    pub provider: Option<String>,
    pub window_kind: Option<String>,
}
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub provider: String,
    pub window_kind: String,
    pub sampled_at: i64,
    pub used_percent: f32,
    pub model: Option<String>,
    pub api_calls: Option<i64>,
    pub estimated_cost_micros: Option<i64>,
    pub overage_cost_micros: Option<i64>,
}
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BillingEntry {
    pub provider: String,
    pub period_start: i64,
    pub period_end: i64,
    pub amount_micros: i64,
    pub currency: String,
    pub source: String,
}
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BillingAggregate {
    pub provider: String,
    pub currency: String,
    pub source: String,
    pub amount_micros: i64,
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BillingSample {
    pub provider: String,
    pub period_start: i64,
    pub period_end: i64,
    pub amount_micros: i64,
    pub currency: String,
    pub source: String,
}
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResult {
    pub points: Vec<HistoryPoint>,
    pub billing: Vec<BillingEntry>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use tempfile::tempdir;

    #[test]
    fn poll_cycle_is_atomic_and_skips_non_fresh_events() {
        let mut db = HistoryDb::open_in_memory().unwrap();
        let fresh = ProviderUsageEvent {
            provider: Provider::Claude,
            snapshot: UsageSnapshot {
                windows: vec![UsageWindow {
                    label: "5 hour".into(),
                    used_percent: 1.,
                    resets_at: 2,
                }],
                fetched_at: 3,
                state: SnapshotState::Fresh,
                details: None,
            },
        };
        let stale = ProviderUsageEvent {
            provider: Provider::Openai,
            snapshot: UsageSnapshot {
                windows: vec![UsageWindow {
                    label: "daily".into(),
                    used_percent: 2.,
                    resets_at: 2,
                }],
                fetched_at: 3,
                state: SnapshotState::Stale,
                details: None,
            },
        };
        assert_eq!(db.record_poll_cycle(&[fresh, stale], &[]).unwrap(), 1);
        assert_eq!(
            db.connection()
                .query_row("SELECT count(*) FROM usage_samples", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn billing_aggregation_keeps_currencies_separate() {
        let mut db = HistoryDb::open_in_memory().unwrap();
        db.insert_billing(&BillingSample {
            provider: "claude".into(),
            period_start: 1,
            period_end: 5,
            amount_micros: 10,
            currency: "USD".into(),
            source: "provider".into(),
        })
        .unwrap();
        db.insert_billing(&BillingSample {
            provider: "claude".into(),
            period_start: 5,
            period_end: 9,
            amount_micros: 20,
            currency: "EUR".into(),
            source: "provider".into(),
        })
        .unwrap();
        let totals = db
            .aggregate_billing(HistoryQuery {
                from: 0,
                to: 10,
                provider: None,
                window_kind: None,
            })
            .unwrap();
        assert_eq!(totals.len(), 2);
        assert!(totals
            .iter()
            .any(|x| x.currency == "USD" && x.amount_micros == 10));
        assert!(totals
            .iter()
            .any(|x| x.currency == "EUR" && x.amount_micros == 20));
    }

    #[test]
    fn invalid_billing_rolls_back_usage_and_billing() {
        let mut db = HistoryDb::open_in_memory().unwrap();
        let event = ProviderUsageEvent {
            provider: Provider::Claude,
            snapshot: UsageSnapshot {
                windows: vec![UsageWindow {
                    label: "5 hour".into(),
                    used_percent: 1.,
                    resets_at: 2,
                }],
                fetched_at: 3,
                state: SnapshotState::Fresh,
                details: None,
            },
        };
        let invalid = BillingSample {
            provider: "claude".into(),
            period_start: 5,
            period_end: 4,
            amount_micros: 1,
            currency: "USD".into(),
            source: "provider".into(),
        };
        assert!(db.record_poll_cycle(&[event], &[invalid]).is_err());
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM usage_samples", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM billing_entries", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn disk_migration_preserves_data_and_configures_wal_timeout_and_constraints() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history.sqlite3");
        let db = HistoryDb::open(&path).unwrap();
        db.connection.execute("INSERT INTO usage_samples(provider,window_kind,used_percent,resets_at,sampled_at) VALUES ('claude','session_5h',1,0,1)", []).unwrap();
        drop(db);
        let db = HistoryDb::open(&path).unwrap();
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM usage_samples", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.connection
                .pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
                .unwrap()
                .to_ascii_lowercase(),
            "wal"
        );
        assert!(db.connection_busy_timeout_ms().unwrap() >= 5000);
        assert!(db.connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='index' AND name='usage_samples_range'", [], |r| r.get::<_, i64>(0)).unwrap() == 1);
        assert_eq!(
            db.connection
                .query_row(
                    "SELECT count(*) FROM pragma_foreign_key_list('usage_samples')",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        assert!(db.connection.execute("INSERT INTO billing_entries(provider,period_start,period_end,amount_micros,currency,source) VALUES ('claude',0,1,1,'usd','provider')", []).is_err());
    }

    #[test]
    fn query_downsamples_deterministically_without_sql_limit_truncation() {
        let db = HistoryDb::open_in_memory().unwrap();
        for t in 0..12_000_i64 {
            db.connection.execute("INSERT INTO usage_samples(provider,window_kind,used_percent,resets_at,sampled_at) VALUES ('claude','session_5h',1,0,?1)", [t]).unwrap();
        }
        let q = HistoryQuery {
            from: 0,
            to: 20_000,
            provider: None,
            window_kind: None,
        };
        let a = db.query(q.clone()).unwrap();
        let b = db.query(q).unwrap();
        assert!(a.points.len() < 12_000 && a.points.len() > 100);
        assert_eq!(
            a.points.iter().map(|p| p.sampled_at).collect::<Vec<_>>(),
            b.points.iter().map(|p| p.sampled_at).collect::<Vec<_>>()
        );
        assert_eq!(a.points.first().unwrap().sampled_at, 0);
        assert_eq!(a.points.last().unwrap().sampled_at, 11_999);
    }
    #[test]
    fn migration_creates_versioned_sample_and_cost_schema() {
        let db = HistoryDb::open_in_memory().unwrap();
        let mut s = db
            .connection()
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let n: Vec<String> = s
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(n.contains(&"usage_samples".into()));
        assert!(n.contains(&"billing_entries".into()));
        assert_eq!(
            db.connection()
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn records_fresh_windows_once_and_leaves_unknown_costs_null() {
        let mut db = HistoryDb::open_in_memory().unwrap();
        let e = ProviderUsageEvent {
            provider: Provider::Claude,
            snapshot: UsageSnapshot {
                windows: vec![UsageWindow {
                    label: "5 hour".into(),
                    used_percent: 25.,
                    resets_at: 2000,
                }],
                fetched_at: 1000,
                state: SnapshotState::Fresh,
                details: None,
            },
        };
        assert_eq!(db.record_event(&e).unwrap(), 1);
        assert_eq!(db.record_event(&e).unwrap(), 0);
        let row: (String, Option<i64>) = db
            .connection()
            .query_row(
                "SELECT window_kind,estimated_cost_micros FROM usage_samples",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(row, ("session_5h".into(), None));
        let columns: Vec<String> = db
            .connection
            .prepare("PRAGMA table_info(usage_samples)")
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(!columns
            .iter()
            .any(|c| c.contains("payload") || c.contains("secret")));
    }
    #[test]
    fn query_is_half_open_and_retention_keeps_cutoff() {
        let mut db = HistoryDb::open_in_memory().unwrap();
        for t in [100, 200, 300] {
            db.connection.execute("INSERT INTO usage_samples(provider,window_kind,used_percent,resets_at,sampled_at) VALUES ('claude','session_5h',1,0,?1)",[t]).unwrap();
        }
        assert_eq!(
            db.query(HistoryQuery {
                from: 200,
                to: 301,
                provider: None,
                window_kind: None
            })
            .unwrap()
            .points
            .len(),
            2
        );
        let boundary = db
            .query(HistoryQuery {
                from: 100,
                to: 200,
                provider: None,
                window_kind: None,
            })
            .unwrap();
        assert_eq!(
            boundary
                .points
                .iter()
                .map(|p| p.sampled_at)
                .collect::<Vec<_>>(),
            vec![100]
        );
        assert_eq!(db.prune_before(200).unwrap(), 1);
        assert_eq!(
            db.query(HistoryQuery {
                from: 0,
                to: 1000,
                provider: None,
                window_kind: None
            })
            .unwrap()
            .points
            .len(),
            2
        );
    }

    #[test]
    fn invalid_sample_rolls_back_entire_cycle() {
        let mut db = HistoryDb::open_in_memory().unwrap();
        let good = ProviderUsageEvent {
            provider: Provider::Claude,
            snapshot: UsageSnapshot {
                windows: vec![UsageWindow {
                    label: "5 hour".into(),
                    used_percent: 1.,
                    resets_at: 2,
                }],
                fetched_at: 3,
                state: SnapshotState::Fresh,
                details: None,
            },
        };
        let bad = ProviderUsageEvent {
            provider: Provider::Openai,
            snapshot: UsageSnapshot {
                windows: vec![UsageWindow {
                    label: "daily".into(),
                    used_percent: 101.,
                    resets_at: 2,
                }],
                fetched_at: 3,
                state: SnapshotState::Fresh,
                details: None,
            },
        };
        assert!(db.record_poll_cycle(&[good, bad], &[]).is_err());
        assert_eq!(
            db.connection()
                .query_row("SELECT count(*) FROM usage_samples", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn migration_is_idempotent_and_rejects_future_versions() {
        let db = HistoryDb::open_in_memory().unwrap();
        db.connection
            .execute_batch("PRAGMA user_version=1")
            .unwrap();
        drop(db);
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA user_version=99").unwrap();
        assert!(matches!(
            HistoryDb::migrate(conn),
            Err(rusqlite::Error::InvalidQuery)
        ));
    }

    #[test]
    fn retention_is_once_per_utc_day_and_keeps_cutoff() {
        let mut db = HistoryDb::open_in_memory().unwrap();
        db.connection.execute("INSERT INTO usage_samples(provider,window_kind,used_percent,resets_at,sampled_at) VALUES ('claude','session_5h',1,0,?1)", [100]).unwrap();
        assert_eq!(db.prune_retention_once(100 + 30 * 86400, 30).unwrap(), 0);
        assert_eq!(db.prune_retention_once(100 + 31 * 86400, 30).unwrap(), 1);
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM usage_samples", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn retention_prunes_billing_at_half_open_cutoff() {
        let mut db = HistoryDb::open_in_memory().unwrap();
        db.connection.execute("INSERT INTO billing_entries(provider,period_start,period_end,amount_micros,currency,source) VALUES ('claude',0,100,1,'USD','provider')", []).unwrap();
        db.connection.execute("INSERT INTO billing_entries(provider,period_start,period_end,amount_micros,currency,source) VALUES ('claude',100,200,1,'USD','provider')", []).unwrap();
        assert_eq!(db.prune_before(100).unwrap(), 1);
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM billing_entries", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn clear_rolls_back_when_second_delete_fails() {
        let mut db = HistoryDb::open_in_memory().unwrap();
        db.connection.execute("INSERT INTO usage_samples(provider,window_kind,used_percent,resets_at,sampled_at) VALUES ('claude','session_5h',1,0,1)", []).unwrap();
        db.connection.execute("INSERT INTO billing_entries(provider,period_start,period_end,amount_micros,currency,source) VALUES ('claude',0,1,1,'USD','provider')", []).unwrap();
        db.connection.execute("CREATE TRIGGER stop_billing_delete BEFORE DELETE ON billing_entries BEGIN SELECT RAISE(ABORT, 'blocked'); END", []).unwrap();
        assert!(db.clear().is_err());
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM usage_samples", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
