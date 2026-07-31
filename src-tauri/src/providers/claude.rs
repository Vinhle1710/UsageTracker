use crate::model::{SnapshotState, UsageSnapshot, UsageWindow};

fn reset_timestamp(value: Option<&serde_json::Value>) -> i64 {
    value
        .and_then(|value| {
            value.as_i64().or_else(|| {
                value.as_str().and_then(|text| {
                    chrono::DateTime::parse_from_rfc3339(text)
                        .ok()
                        .map(|date| date.timestamp())
                })
            })
        })
        .unwrap_or(0)
}

pub fn parse_usage(
    value: &serde_json::Value,
    fetched_at: i64,
    state: SnapshotState,
) -> UsageSnapshot {
    let mut windows = Vec::new();
    for (key, label) in [("five_hour", "5 hour"), ("seven_day", "Weekly")] {
        let Some(window) = value.get(key).filter(|v| !v.is_null()) else {
            continue;
        };
        let Some(utilization) = window.get("utilization").and_then(|v| v.as_f64()) else {
            continue;
        };
        windows.push(UsageWindow {
            label: label.into(),
            used_percent: utilization as f32,
            resets_at: reset_timestamp(window.get("resets_at")),
        });
    }
    UsageSnapshot {
        windows,
        fetched_at,
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_both_windows() {
        let v: serde_json::Value = serde_json::from_str(r#"{"five_hour":{"utilization":12.5,"resets_at":100},"seven_day":{"utilization":48.0,"resets_at":200}}"#).unwrap();
        let s = parse_usage(&v, 0, SnapshotState::Fresh);
        assert_eq!(s.windows.len(), 2);
        assert_eq!(s.windows[0].label, "5 hour");
    }
    #[test]
    fn absent_five_hour_hides_row() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"seven_day":{"utilization":48.0}}"#).unwrap();
        assert_eq!(parse_usage(&v, 0, SnapshotState::Fresh).windows.len(), 1);
    }
    #[test]
    fn zero_utilization_still_renders() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"five_hour":{"utilization":0.0}}"#).unwrap();
        assert_eq!(parse_usage(&v, 0, SnapshotState::Fresh).windows.len(), 1);
    }
    #[test]
    fn empty_payload_yields_no_windows() {
        let v: serde_json::Value = serde_json::from_str("{}").unwrap();
        assert!(parse_usage(&v, 0, SnapshotState::Fresh).windows.is_empty());
    }
    #[test]
    fn parses_rfc3339_reset_time() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"five_hour":{"utilization":1.0,"resets_at":"2026-07-31T18:49:59.785955+00:00"}}"#,
        )
        .unwrap();
        assert_eq!(
            parse_usage(&v, 0, SnapshotState::Fresh).windows[0].resets_at,
            1785523799
        );
    }
}
