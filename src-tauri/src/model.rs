use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotState {
    Fresh,
    Stale,
    Error,
    /// No usage fetched yet and the refresh is still being retried. Distinct from `Stale`, which
    /// claims the data is known-unavailable rather than merely not in hand.
    Pending,
    /// The provider's credential file does not exist, so this machine has never been signed in.
    /// Distinct from `Error`, which means credentials exist but were rejected — the two need
    /// different instructions ("sign in" versus "re-authenticate").
    #[serde(rename = "signed-out")]
    SignedOut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageWindow {
    pub label: String,
    pub used_percent: f32,
    pub resets_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pace: Option<crate::pace::Pace>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub windows: Vec<UsageWindow>,
    pub fetched_at: i64,
    pub state: SnapshotState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<ProviderDetails>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderDetails {
    Claude(ClaudeUsageDetails),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsageDetails {
    pub limits: DataSection<Vec<ClaudeModelLimit>>,
    pub extra: DataSection<ClaudeExtra>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ClaudeServiceStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeModelLimit {
    pub model_key: String,
    pub display_name: String,
    pub utilization_percent: f32,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Money {
    pub minor_units: i64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeExtra {
    pub spend: Option<Money>,
    pub budget: Option<Money>,
    pub balance: Option<Money>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataSectionState {
    Fresh,
    Stale,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSection<T> {
    pub value: Option<T>,
    pub fetched_at: i64,
    pub state: DataSectionState,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnavailableReason {
    NoCredential,
    InsufficientRole,
    UnsupportedBySource,
    ProviderUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleMoney {
    pub minor_units: String,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostPeriod {
    pub starts_at: String,
    pub ends_at: String,
    pub timezone: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostPoint {
    pub key: String,
    pub label: String,
    pub amount: ConsoleMoney,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleCostsDashboard {
    pub period: CostPeriod,
    pub spend: DataSection<ConsoleMoney>,
    pub prepaid_balance: DataSection<ConsoleMoney>,
    pub daily: DataSection<Vec<CostPoint>>,
    pub by_api_key: DataSection<Vec<CostPoint>>,
    pub by_model: DataSection<Vec<CostPoint>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeServiceStatus {
    pub indicator: String,
    pub description: String,
    pub incidents: Vec<ClaudeIncident>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeIncident {
    pub name: String,
    pub status: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    fn signed_out_reaches_the_ui_as_its_own_state_not_as_an_error() {
        // The UI has to tell "you have never signed in" apart from "your token was rejected";
        // they need different copy, so the wire value must be distinct from "error".
        assert_eq!(
            serde_json::to_value(SnapshotState::SignedOut).unwrap(),
            "signed-out"
        );
        assert_eq!(serde_json::to_value(SnapshotState::Error).unwrap(), "error");
    }

    #[test]
    fn provider_event_serialization_keeps_ownership_explicit() {
        let snapshot = UsageSnapshot {
            windows: vec![],
            fetched_at: 42,
            state: SnapshotState::Fresh,
            details: None,
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

    #[test]
    fn console_money_serializes_losslessly() {
        let dashboard = ConsoleCostsDashboard {
            period: CostPeriod {
                starts_at: "2026-08-01T00:00:00Z".into(),
                ends_at: "2026-09-01T00:00:00Z".into(),
                timezone: "UTC".into(),
            },
            spend: DataSection {
                value: Some(ConsoleMoney {
                    minor_units: "900719925474099312345".into(),
                    currency: "USD".into(),
                }),
                fetched_at: 1,
                state: DataSectionState::Fresh,
                error_code: None,
            },
            prepaid_balance: DataSection {
                value: None,
                fetched_at: 1,
                state: DataSectionState::Unavailable,
                error_code: Some("unsupported-by-source".into()),
            },
            daily: DataSection {
                value: None,
                fetched_at: 1,
                state: DataSectionState::Unavailable,
                error_code: None,
            },
            by_api_key: DataSection {
                value: None,
                fetched_at: 1,
                state: DataSectionState::Unavailable,
                error_code: None,
            },
            by_model: DataSection {
                value: None,
                fetched_at: 1,
                state: DataSectionState::Unavailable,
                error_code: None,
            },
        };
        let json = serde_json::to_value(dashboard).unwrap();
        assert_eq!(
            json["spend"]["value"]["minorUnits"],
            "900719925474099312345"
        );
    }

    #[test]
    fn claude_details_preserve_unknown_models() {
        let details = ProviderDetails::Claude(ClaudeUsageDetails {
            limits: DataSection {
                value: Some(vec![ClaudeModelLimit {
                    model_key: "claude-next-x".into(),
                    display_name: "Future model".into(),
                    utilization_percent: 12.5,
                    resets_at: None,
                }]),
                fetched_at: 1,
                state: DataSectionState::Fresh,
                error_code: None,
            },
            extra: DataSection {
                value: Some(ClaudeExtra {
                    spend: Some(Money {
                        minor_units: 125,
                        currency: "USD".into(),
                    }),
                    ..Default::default()
                }),
                fetched_at: 1,
                state: DataSectionState::Fresh,
                error_code: None,
            },
            status: None,
        });
        let roundtrip: ProviderDetails =
            serde_json::from_value(serde_json::to_value(&details).unwrap()).unwrap();
        assert_eq!(roundtrip, details);
    }
}
