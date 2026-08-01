use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotState {
    Fresh,
    Stale,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageWindow {
    pub label: String,
    pub used_percent: f32,
    pub resets_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub windows: Vec<UsageWindow>,
    pub fetched_at: i64,
    pub state: SnapshotState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Openai,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderUsageEvent {
    pub provider: Provider,
    pub snapshot: UsageSnapshot,
}

pub fn label_for_minutes(minutes: u32) -> String {
    match minutes {
        m if m % 10080 == 0 => {
            let weeks = m / 10080;
            if weeks == 1 {
                "Weekly".into()
            } else {
                format!("{weeks} weeks")
            }
        }
        m if m % 1440 == 0 => {
            let days = m / 1440;
            if days == 1 {
                "Daily".into()
            } else {
                format!("{days} days")
            }
        }
        m if m % 60 == 0 => format!("{} hour", m / 60),
        m => format!("{m} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels_weekly_window() {
        assert_eq!(label_for_minutes(10080), "Weekly");
    }
    #[test]
    fn labels_five_hour_window() {
        assert_eq!(label_for_minutes(300), "5 hour");
    }
    #[test]
    fn labels_daily_window() {
        assert_eq!(label_for_minutes(1440), "Daily");
    }
    #[test]
    fn labels_odd_window_in_minutes() {
        assert_eq!(label_for_minutes(45), "45 min");
    }
    #[test]
    fn provider_event_serialization_keeps_ownership_explicit() {
        let snapshot = UsageSnapshot {
            windows: vec![],
            fetched_at: 42,
            state: SnapshotState::Fresh,
        };
        let claude = serde_json::to_value(ProviderUsageEvent {
            provider: Provider::Claude,
            snapshot: snapshot.clone(),
        })
        .unwrap();
        let openai = serde_json::to_value(ProviderUsageEvent {
            provider: Provider::Openai,
            snapshot,
        })
        .unwrap();
        assert_eq!(claude["provider"], "claude");
        assert_eq!(openai["provider"], "openai");
    }
}
