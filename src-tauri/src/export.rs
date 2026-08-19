use crate::history::{HistoryQuery, HistoryResult};
use serde::Serialize;
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};
#[derive(Serialize)]
struct Envelope<'a> {
    #[serde(rename = "schemaVersion")]
    schema_version: u8,
    exported_at: i64,
    query: &'a HistoryQuery,
    points: &'a Vec<crate::history::HistoryPoint>,
    billing: &'a Vec<crate::history::BillingEntry>,
}
pub fn history_json(
    r: &HistoryResult,
    q: &HistoryQuery,
    at: i64,
) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&Envelope {
        schema_version: 1,
        exported_at: at,
        query: q,
        points: &r.points,
        billing: &r.billing,
    })
}
pub fn history_csv(r: &HistoryResult) -> Result<Vec<u8>, csv::Error> {
    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record([
        "provider",
        "window_kind",
        "sampled_at",
        "used_percent",
        "model",
        "api_calls",
        "estimated_cost_micros",
        "overage_cost_micros",
    ])?;
    for p in &r.points {
        w.write_record([
            p.provider.clone(),
            p.window_kind.clone(),
            p.sampled_at.to_string(),
            p.used_percent.to_string(),
            p.model.clone().unwrap_or_default(),
            p.api_calls.map(|v| v.to_string()).unwrap_or_default(),
            p.estimated_cost_micros
                .map(|v| v.to_string())
                .unwrap_or_default(),
            p.overage_cost_micros
                .map(|v| v.to_string())
                .unwrap_or_default(),
        ])?;
    }
    Ok(w.into_inner().map_err(|e| e.into_error())?)
}
pub fn history_billing_csv(r: &HistoryResult) -> Result<Vec<u8>, csv::Error> {
    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record([
        "provider",
        "period_start",
        "period_end",
        "amount_micros",
        "currency",
        "source",
    ])?;
    for b in &r.billing {
        w.write_record([
            b.provider.as_str(),
            &b.period_start.to_string(),
            &b.period_end.to_string(),
            &b.amount_micros.to_string(),
            b.currency.as_str(),
            b.source.as_str(),
        ])?;
    }
    Ok(w.into_inner().map_err(|e| e.into_error())?)
}
fn sibling_temp(dest: &Path) -> PathBuf {
    let name = dest
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or("history");
    dest.with_file_name(format!(".{name}.tmp-{}", std::process::id()))
}
pub fn write_export(
    dest: &Path,
    format: &str,
    r: &HistoryResult,
    q: &HistoryQuery,
    at: i64,
) -> Result<(), String> {
    if dest.is_dir()
        || dest.parent().is_some_and(|p| !p.exists())
        || !matches!(format, "json" | "csv")
        || dest.extension().and_then(|e| e.to_str()) != Some(format)
    {
        return Err("invalid destination".into());
    }
    if format == "csv" {
        return write_csv_pair(dest, r);
    }
    let tmp = sibling_temp(dest);
    let result = (|| {
        let mut f = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        write_json_stream(&mut f, r, q, at)?;
        f.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn write_csv_pair(dest: &Path, r: &HistoryResult) -> Result<(), String> {
    let billing_dest = dest.with_file_name(format!(
        "{}-billing.csv",
        dest.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("history")
    ));
    let usage_tmp = sibling_temp(dest);
    let billing_tmp = sibling_temp(&billing_dest);
    let prepared = (|| {
        let mut usage = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&usage_tmp)
            .map_err(|e| e.to_string())?;
        write_usage_csv_stream(&mut usage, r)?;
        usage.sync_all().map_err(|e| e.to_string())?;
        let mut billing = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&billing_tmp)
            .map_err(|e| e.to_string())?;
        write_billing_csv_stream(&mut billing, r)?;
        billing.sync_all().map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    })();
    if prepared.is_err() {
        let _ = std::fs::remove_file(&usage_tmp);
        let _ = std::fs::remove_file(&billing_tmp);
        return prepared;
    }
    let published = publish_pair(&usage_tmp, &billing_tmp, dest, &billing_dest);
    if published.is_err() {
        let _ = std::fs::remove_file(&usage_tmp);
        let _ = std::fs::remove_file(&billing_tmp);
    }
    published
}

fn publish_pair(
    usage_tmp: &Path,
    billing_tmp: &Path,
    usage_dest: &Path,
    billing_dest: &Path,
) -> Result<(), String> {
    publish_pair_with(usage_tmp, billing_tmp, usage_dest, billing_dest, |_| Ok(()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublicationStep {
    Usage,
    Billing,
}

fn publish_pair_with<F: FnMut(PublicationStep) -> Result<(), String>>(
    usage_tmp: &Path,
    billing_tmp: &Path,
    usage_dest: &Path,
    billing_dest: &Path,
    mut before_publish: F,
) -> Result<(), String> {
    let usage_backup = usage_dest.with_file_name(format!(
        ".{}-bak-{}",
        usage_dest
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or("usage"),
        std::process::id()
    ));
    let billing_backup = billing_dest.with_file_name(format!(
        ".{}-bak-{}",
        billing_dest
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or("billing"),
        std::process::id()
    ));
    let had_usage = usage_dest.exists();
    let had_billing = billing_dest.exists();
    if had_usage {
        std::fs::rename(usage_dest, &usage_backup).map_err(|e| e.to_string())?;
    }
    if had_billing {
        if let Err(e) = std::fs::rename(billing_dest, &billing_backup) {
            if had_usage {
                let _ = std::fs::rename(&usage_backup, usage_dest);
            }
            return Err(e.to_string());
        }
    }
    let result = (|| {
        before_publish(PublicationStep::Usage)?;
        std::fs::rename(usage_tmp, usage_dest).map_err(|e| e.to_string())?;
        before_publish(PublicationStep::Billing)?;
        std::fs::rename(billing_tmp, billing_dest).map_err(|e| e.to_string())
    })();
    if result.is_ok() {
        let _ = std::fs::remove_file(&usage_backup);
        let _ = std::fs::remove_file(&billing_backup);
        return Ok(());
    }
    let _ = std::fs::remove_file(usage_dest);
    let _ = std::fs::remove_file(billing_dest);
    if had_usage {
        let _ = std::fs::rename(&usage_backup, usage_dest);
    }
    if had_billing {
        let _ = std::fs::rename(&billing_backup, billing_dest);
    }
    let _ = std::fs::remove_file(usage_tmp);
    let _ = std::fs::remove_file(billing_tmp);
    result
}

fn write_json_stream(
    mut out: impl Write,
    r: &HistoryResult,
    q: &HistoryQuery,
    at: i64,
) -> Result<(), String> {
    out.write_all(b"{\"schemaVersion\":1,\"exportedAt\":")
        .map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut out, &at).map_err(|e| e.to_string())?;
    out.write_all(b",\"query\":").map_err(|e| e.to_string())?;
    serde_json::to_writer(&mut out, q).map_err(|e| e.to_string())?;
    out.write_all(b",\"points\":[").map_err(|e| e.to_string())?;
    for (i, p) in r.points.iter().enumerate() {
        if i > 0 {
            out.write_all(b",").map_err(|e| e.to_string())?;
        }
        serde_json::to_writer(&mut out, p).map_err(|e| e.to_string())?;
    }
    out.write_all(b"],\"billing\":[")
        .map_err(|e| e.to_string())?;
    for (i, b) in r.billing.iter().enumerate() {
        if i > 0 {
            out.write_all(b",").map_err(|e| e.to_string())?;
        }
        serde_json::to_writer(&mut out, b).map_err(|e| e.to_string())?;
    }
    out.write_all(b"]}").map_err(|e| e.to_string())
}

fn write_usage_csv_stream(mut out: impl Write, r: &HistoryResult) -> Result<(), String> {
    let mut w = csv::Writer::from_writer(&mut out);
    w.write_record([
        "provider",
        "window_kind",
        "sampled_at",
        "used_percent",
        "model",
        "api_calls",
        "estimated_cost_micros",
        "overage_cost_micros",
    ])
    .map_err(|e| e.to_string())?;
    for p in &r.points {
        w.write_record([
            p.provider.as_str(),
            p.window_kind.as_str(),
            &p.sampled_at.to_string(),
            &p.used_percent.to_string(),
            p.model.as_deref().unwrap_or(""),
            &p.api_calls.map(|x| x.to_string()).unwrap_or_default(),
            &p.estimated_cost_micros
                .map(|x| x.to_string())
                .unwrap_or_default(),
            &p.overage_cost_micros
                .map(|x| x.to_string())
                .unwrap_or_default(),
        ])
        .map_err(|e| e.to_string())?;
    }
    w.flush().map_err(|e| e.to_string())
}

fn write_billing_csv_stream(mut out: impl Write, r: &HistoryResult) -> Result<(), String> {
    let mut w = csv::Writer::from_writer(&mut out);
    w.write_record([
        "provider",
        "period_start",
        "period_end",
        "amount_micros",
        "currency",
        "source",
    ])
    .map_err(|e| e.to_string())?;
    for b in &r.billing {
        w.write_record([
            b.provider.as_str(),
            &b.period_start.to_string(),
            &b.period_end.to_string(),
            &b.amount_micros.to_string(),
            b.currency.as_str(),
            b.source.as_str(),
        ])
        .map_err(|e| e.to_string())?;
    }
    w.flush().map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    fn fixture() -> HistoryResult {
        HistoryResult {
            points: vec![crate::history::HistoryPoint {
                provider: "claude".into(),
                window_kind: "session_5h".into(),
                sampled_at: 1000,
                used_percent: 25.,
                model: Some("model,one".into()),
                api_calls: None,
                estimated_cost_micros: None,
                overage_cost_micros: None,
            }],
            billing: vec![crate::history::BillingEntry {
                provider: "claude".into(),
                period_start: 1,
                period_end: 2,
                amount_micros: 3,
                currency: "USD".into(),
                source: "provider".into(),
            }],
        }
    }
    #[test]
    fn csv_has_stable_columns_and_blank_nulls() {
        let text = String::from_utf8(history_csv(&fixture()).unwrap()).unwrap();
        assert_eq!(text.lines().next().unwrap(), "provider,window_kind,sampled_at,used_percent,model,api_calls,estimated_cost_micros,overage_cost_micros");
        assert!(text.contains("\"model,one\""));
    }
    #[test]
    fn json_is_versioned_and_billing_csv_is_separate_representation() {
        let value: serde_json::Value = serde_json::from_slice(
            &history_json(
                &fixture(),
                &HistoryQuery {
                    from: 0,
                    to: 10,
                    provider: None,
                    window_kind: None,
                },
                99,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(value["schemaVersion"], 1);
        let csv = String::from_utf8(history_billing_csv(&fixture()).unwrap()).unwrap();
        assert!(csv.contains("provider,period_start,period_end,amount_micros,currency,source"));
        assert!(csv.contains("claude,1,2,3,USD,provider"));
    }

    #[test]
    fn filesystem_export_is_atomic_streamed_and_overwrites() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("history.csv");
        let q = HistoryQuery {
            from: 0,
            to: 10,
            provider: None,
            window_kind: None,
        };
        write_export(&dest, "csv", &fixture(), &q, 99).unwrap();
        let text = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(text.lines().next().unwrap(), "provider,window_kind,sampled_at,used_percent,model,api_calls,estimated_cost_micros,overage_cost_micros");
        let billing = std::fs::read_to_string(dir.path().join("history-billing.csv")).unwrap();
        assert!(
            text.contains("claude,session_5h") && billing.contains("claude,1,2,3,USD,provider")
        );
        write_export(
            &dest,
            "csv",
            &HistoryResult {
                points: vec![],
                billing: vec![],
            },
            &q,
            100,
        )
        .unwrap();
        assert!(!std::fs::read_to_string(&dest)
            .unwrap()
            .contains("model,one"));
        assert!(write_export(
            &dir.path().join("missing\u{005c}history.csv"),
            "csv",
            &fixture(),
            &q,
            1
        )
        .is_err());
        assert!(!std::fs::read_dir(dir.path()).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp-")));
    }

    #[test]
    fn injected_stream_write_failure_is_returned() {
        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("injected"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert!(write_usage_csv_stream(FailingWriter, &fixture()).is_err());
        assert!(write_billing_csv_stream(FailingWriter, &fixture()).is_err());
    }

    #[test]
    fn paired_publish_failure_restores_existing_usage_file() {
        let dir = tempdir().unwrap();
        let usage_dest = dir.path().join("history.csv");
        std::fs::write(&usage_dest, "good usage").unwrap();
        let usage_tmp = dir.path().join(".usage.tmp");
        let billing_tmp = dir.path().join(".billing.tmp");
        std::fs::write(&usage_tmp, "new usage").unwrap();
        std::fs::write(&billing_tmp, "new billing").unwrap();
        let result = publish_pair(&usage_tmp, &billing_tmp, &usage_dest, &usage_dest);
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(&usage_dest).unwrap(), "good usage");
    }

    #[test]
    fn billing_publish_failpoint_restores_distinct_existing_pair_and_cleans_artifacts() {
        let dir = tempdir().unwrap();
        let usage_dest = dir.path().join("history.csv");
        let billing_dest = dir.path().join("history-billing.csv");
        std::fs::write(&usage_dest, "old usage").unwrap();
        std::fs::write(&billing_dest, "old billing").unwrap();
        let usage_tmp = dir.path().join(".usage.tmp");
        let billing_tmp = dir.path().join(".billing.tmp");
        std::fs::write(&usage_tmp, "new usage").unwrap();
        std::fs::write(&billing_tmp, "new billing").unwrap();
        let result = publish_pair_with(
            &usage_tmp,
            &billing_tmp,
            &usage_dest,
            &billing_dest,
            |step| {
                if step == PublicationStep::Billing {
                    Err("injected billing rename failure".into())
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(&usage_dest).unwrap(), "old usage");
        assert_eq!(
            std::fs::read_to_string(&billing_dest).unwrap(),
            "old billing"
        );
        assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| {
            let name = entry.unwrap().file_name();
            let name = name.to_string_lossy();
            name.contains("tmp") || name.contains("bak")
        }));
    }
}
