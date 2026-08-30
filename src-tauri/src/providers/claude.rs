use crate::model::{SnapshotState, UsageSnapshot, UsageWindow};

/// Reads the Claude Desktop app's local usage-history cache. The desktop app writes this file
/// for itself every few minutes regardless of whether Claude Code CLI has ever run on this
/// machine, which makes it the only usage source available to a user who has only ever signed
/// into the desktop app: `~/.claude/.credentials.json` (an OAuth session for the Code CLI) does
/// not exist for them at all.
pub fn read_desktop_usage_history(path: &std::path::Path) -> Option<serde_json::Value> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Parses the newest sample out of that cache. Reverse-engineered from the file, not a
/// documented format: `u.fh`/`u.sd` line up with this app's own "5 hour"/"Weekly" windows, and
/// there is no reset timestamp in it, so `resets_at` is left at 0 — the UI already renders that
/// as "reset time unavailable" for any window whose reset time isn't known.
pub fn parse_desktop_usage_history(
    value: &serde_json::Value,
    fetched_at: i64,
    state: SnapshotState,
) -> Option<UsageSnapshot> {
    let latest = value
        .get("samples")?
        .as_array()?
        .iter()
        .max_by_key(|sample| sample.get("t").and_then(|t| t.as_i64()).unwrap_or(i64::MIN))?;
    let usage = latest.get("u")?;
    let mut windows = Vec::new();
    for (key, label) in [("fh", "5 hour"), ("sd", "Weekly")] {
        let Some(percent) = usage.get(key).and_then(|v| v.as_f64()) else {
            continue;
        };
        windows.push(UsageWindow {
            label: label.into(),
            used_percent: percent as f32,
            resets_at: 0,
            pace: None,
        });
    }
    if windows.is_empty() {
        return None;
    }
    Some(UsageSnapshot {
        windows,
        fetched_at,
        state,
        details: None,
    })
}

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

fn is_valid_reset_timestamp(value: Option<&serde_json::Value>) -> bool {
    match value {
        None | Some(serde_json::Value::Null) => true,
        Some(value) => {
            value.as_i64().is_some()
                || value
                    .as_str()
                    .is_some_and(|text| chrono::DateTime::parse_from_rfc3339(text).is_ok())
        }
    }
}

/// Unified rate-limit headers, returned on every `/v1/messages` response (including 429s).
/// These are the replacement for `GET /api/oauth/usage`, which now answers 429 unconditionally.
pub const UNIFIED_UTILIZATION_5H: &str = "anthropic-ratelimit-unified-5h-utilization";
pub const UNIFIED_RESET_5H: &str = "anthropic-ratelimit-unified-5h-reset";
pub const UNIFIED_UTILIZATION_7D: &str = "anthropic-ratelimit-unified-7d-utilization";
pub const UNIFIED_RESET_7D: &str = "anthropic-ratelimit-unified-7d-reset";

/// Every header this app is willing to read off a provider response. Anything not named here is
/// dropped before the response leaves `fetch_response`, so an unexpected header can never reach
/// the UI or a diagnostic log.
pub const ALLOWED_USAGE_HEADERS: [&str; 4] = [
    UNIFIED_UTILIZATION_5H,
    UNIFIED_RESET_5H,
    UNIFIED_UTILIZATION_7D,
    UNIFIED_RESET_7D,
];

/// Documented as a 0.0-1.0 fraction. A value above 1 is taken as already being a percent
/// rather than multiplied again: if the scale ever ships as 0-100, multiplying would pin every
/// card at 100%, and no value in the documented range is ambiguous under this rule.
fn utilization_percent(raw: &str) -> Option<f32> {
    let value: f64 = raw.trim().parse().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let percent = if value > 1.0 { value } else { value * 100.0 };
    Some(percent.min(100.0) as f32)
}

/// Builds a usage snapshot from the unified rate-limit headers, or `None` when the response
/// carried none — which is what tells the caller to fall back rather than render zeroes.
pub fn parse_unified_rate_limit_headers(
    headers: &std::collections::BTreeMap<String, String>,
    fetched_at: i64,
    state: SnapshotState,
) -> Option<UsageSnapshot> {
    let header = |name: &str| headers.get(name).map(String::as_str);
    let mut windows = Vec::new();
    for (label, utilization_key, reset_key) in [
        ("5 hour", UNIFIED_UTILIZATION_5H, UNIFIED_RESET_5H),
        ("Weekly", UNIFIED_UTILIZATION_7D, UNIFIED_RESET_7D),
    ] {
        let Some(used_percent) = header(utilization_key).and_then(utilization_percent) else {
            continue;
        };
        windows.push(UsageWindow {
            label: label.into(),
            used_percent,
            // 0 is what the UI already renders as "reset time unavailable", so a missing or
            // unreadable reset degrades to that instead of claiming the epoch.
            resets_at: header(reset_key)
                .and_then(|value| value.trim().parse::<f64>().ok())
                .filter(|value| value.is_finite() && *value > 0.0)
                .map(|value| value as i64)
                .unwrap_or(0),
            pace: None,
        });
    }
    if windows.is_empty() {
        return None;
    }
    Some(UsageSnapshot {
        windows,
        fetched_at,
        state,
        details: None,
    })
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
            pace: None,
        });
    }
    UsageSnapshot {
        windows,
        fetched_at,
        state,
        details: None,
    }
}

pub fn parse_usage_checked(
    value: &serde_json::Value,
    fetched_at: i64,
    state: SnapshotState,
) -> Result<UsageSnapshot, super::FetchError> {
    let has_non_null_expected_window = ["five_hour", "seven_day"].iter().any(|key| {
        value
            .get(*key)
            .map(|window| !window.is_null())
            .unwrap_or(false)
    });
    if !has_non_null_expected_window {
        return Err(super::FetchError::Malformed);
    }
    for key in ["five_hour", "seven_day"] {
        let Some(window) = value.get(key) else {
            continue;
        };
        if window.is_null() {
            continue;
        }
        let Some(window) = window.as_object() else {
            return Err(super::FetchError::Malformed);
        };
        if window
            .get("utilization")
            .and_then(|value| value.as_f64())
            .is_none()
        {
            return Err(super::FetchError::Malformed);
        }
        if !is_valid_reset_timestamp(window.get("resets_at")) {
            return Err(super::FetchError::Malformed);
        }
    }
    Ok(parse_usage(value, fetched_at, state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::FetchError;

    fn headers(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn reads_both_windows_from_the_unified_rate_limit_headers() {
        let value = parse_unified_rate_limit_headers(
            &headers(&[
                ("anthropic-ratelimit-unified-5h-utilization", "0.125"),
                ("anthropic-ratelimit-unified-5h-reset", "1700000000"),
                ("anthropic-ratelimit-unified-7d-utilization", "0.48"),
                ("anthropic-ratelimit-unified-7d-reset", "1700600000"),
            ]),
            0,
            SnapshotState::Fresh,
        )
        .unwrap();

        assert_eq!(value.windows.len(), 2);
        assert_eq!(value.windows[0].label, "5 hour");
        assert_eq!(value.windows[0].used_percent, 12.5);
        assert_eq!(value.windows[0].resets_at, 1_700_000_000);
        assert_eq!(value.windows[1].label, "Weekly");
        assert_eq!(value.windows[1].used_percent, 48.0);
    }

    #[test]
    fn a_response_carrying_no_unified_headers_is_not_a_usage_reading() {
        // Every non-Claude response, and any future contract change, lands here — the caller
        // has to fall back rather than render a card full of zeroes.
        assert!(parse_unified_rate_limit_headers(&headers(&[]), 0, SnapshotState::Fresh).is_none());
        assert!(parse_unified_rate_limit_headers(
            &headers(&[("content-type", "application/json")]),
            0,
            SnapshotState::Fresh
        )
        .is_none());
    }

    #[test]
    fn one_window_present_is_still_a_reading() {
        let value = parse_unified_rate_limit_headers(
            &headers(&[("anthropic-ratelimit-unified-7d-utilization", "0.9")]),
            0,
            SnapshotState::Fresh,
        )
        .unwrap();

        assert_eq!(value.windows.len(), 1);
        assert_eq!(value.windows[0].label, "Weekly");
    }

    #[test]
    fn a_missing_reset_reads_as_unknown_rather_than_the_epoch() {
        // resets_at 0 is what the UI already renders as "reset time unavailable".
        let value = parse_unified_rate_limit_headers(
            &headers(&[("anthropic-ratelimit-unified-5h-utilization", "0.2")]),
            0,
            SnapshotState::Fresh,
        )
        .unwrap();

        assert_eq!(value.windows[0].resets_at, 0);
    }

    #[test]
    fn a_utilization_already_expressed_as_a_percent_is_not_multiplied_again() {
        // Documented scale is 0.0-1.0. If it ever ships as 0-100 instead, multiplying would
        // pin every user's card at 100% — a loud, wrong reading. Values above 1 are therefore
        // taken as already-percent, which cannot misread anything in the documented range.
        let value = parse_unified_rate_limit_headers(
            &headers(&[("anthropic-ratelimit-unified-5h-utilization", "73")]),
            0,
            SnapshotState::Fresh,
        )
        .unwrap();

        assert_eq!(value.windows[0].used_percent, 73.0);
    }

    #[test]
    fn a_full_window_reads_as_one_hundred_percent_either_way() {
        for raw in ["1", "1.0", "100"] {
            let value = parse_unified_rate_limit_headers(
                &headers(&[("anthropic-ratelimit-unified-5h-utilization", raw)]),
                0,
                SnapshotState::Fresh,
            )
            .unwrap();
            assert_eq!(value.windows[0].used_percent, 100.0, "raw value {raw}");
        }
    }

    #[test]
    fn unparseable_header_values_are_skipped_not_read_as_zero() {
        assert!(parse_unified_rate_limit_headers(
            &headers(&[("anthropic-ratelimit-unified-5h-utilization", "unknown")]),
            0,
            SnapshotState::Fresh
        )
        .is_none());
    }

    #[test]
    fn the_caller_supplied_state_is_carried_onto_the_snapshot() {
        // A 429 still carries real headers; the caller decides how to grade it.
        let value = parse_unified_rate_limit_headers(
            &headers(&[("anthropic-ratelimit-unified-5h-utilization", "1.0")]),
            77,
            SnapshotState::Fresh,
        )
        .unwrap();

        assert_eq!(value.state, SnapshotState::Fresh);
        assert_eq!(value.fetched_at, 77);
    }

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
    fn checked_parser_rejects_payload_without_expected_window_fields() {
        for raw in ["{}", r#"{"unknown":{"utilization":42.0}}"#] {
            let v: serde_json::Value = serde_json::from_str(raw).unwrap();
            assert_eq!(
                parse_usage_checked(&v, 0, SnapshotState::Fresh),
                Err(FetchError::Malformed)
            );
        }
    }
    #[test]
    fn checked_parser_rejects_scalar_or_invalid_window_payloads() {
        for raw in [
            r#"{"five_hour":42}"#,
            r#"{"seven_day":"unavailable"}"#,
            r#"{"five_hour":{}}"#,
            r#"{"seven_day":{"utilization":"48.0"}}"#,
        ] {
            let v: serde_json::Value = serde_json::from_str(raw).unwrap();
            assert_eq!(
                parse_usage_checked(&v, 0, SnapshotState::Fresh),
                Err(FetchError::Malformed)
            );
        }
    }
    #[test]
    fn checked_parser_accepts_one_window_and_null_window_payloads() {
        let one_window: serde_json::Value =
            serde_json::from_str(r#"{"five_hour":{"utilization":48.0},"seven_day":null}"#).unwrap();
        let parsed = parse_usage_checked(&one_window, 0, SnapshotState::Fresh).unwrap();
        assert_eq!(parsed.windows.len(), 1);

        let missing_window: serde_json::Value =
            serde_json::from_str(r#"{"seven_day":{"utilization":48.0}}"#).unwrap();
        assert_eq!(
            parse_usage_checked(&missing_window, 0, SnapshotState::Fresh)
                .unwrap()
                .windows
                .len(),
            1
        );
    }
    #[test]
    fn checked_parser_rejects_both_expected_windows_as_null() {
        let value: serde_json::Value =
            serde_json::from_str(r#"{"five_hour":null,"seven_day":null}"#).unwrap();
        assert_eq!(
            parse_usage_checked(&value, 0, SnapshotState::Fresh),
            Err(FetchError::Malformed)
        );
    }
    #[test]
    fn checked_parser_rejects_invalid_reset_timestamps() {
        for resets_at in [
            serde_json::json!("not-a-timestamp"),
            serde_json::json!(true),
            serde_json::json!(12.5),
        ] {
            let value = serde_json::json!({
                "five_hour": { "utilization": 48.0, "resets_at": resets_at },
                "seven_day": null,
            });
            assert_eq!(
                parse_usage_checked(&value, 0, SnapshotState::Fresh),
                Err(FetchError::Malformed)
            );
        }
    }
    #[test]
    fn parses_rfc3339_reset_time() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"five_hour":{"utilization":1.0,"resets_at":"2026-07-31T18:49:59.785955+00:00"}}"#,
        )
        .unwrap();
        assert_eq!(
            parse_usage_checked(&v, 0, SnapshotState::Fresh)
                .unwrap()
                .windows[0]
                .resets_at,
            1785523799
        );
    }

    #[test]
    fn parses_the_newest_sample_from_the_desktop_usage_history_cache() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"version":2,"samples":[
                {"t":100,"org":"00000000-0000-0000-0000-000000000000","u":{"fh":3,"sd":39}},
                {"t":300,"org":"00000000-0000-0000-0000-000000000000","u":{"fh":13,"sd":40}},
                {"t":200,"org":"00000000-0000-0000-0000-000000000000","u":{"fh":9,"sd":40}}
            ]}"#,
        )
        .unwrap();
        let snapshot = parse_desktop_usage_history(&v, 0, SnapshotState::Stale).unwrap();
        assert_eq!(snapshot.windows.len(), 2);
        assert_eq!(snapshot.windows[0].label, "5 hour");
        assert_eq!(snapshot.windows[0].used_percent, 13.0);
        assert_eq!(snapshot.windows[1].label, "Weekly");
        assert_eq!(snapshot.windows[1].used_percent, 40.0);
        assert_eq!(snapshot.windows[0].resets_at, 0);
    }
    #[test]
    fn desktop_usage_history_with_no_samples_yields_nothing() {
        let v: serde_json::Value = serde_json::from_str(r#"{"version":2,"samples":[]}"#).unwrap();
        assert!(parse_desktop_usage_history(&v, 0, SnapshotState::Stale).is_none());
    }
    #[test]
    fn desktop_usage_history_missing_the_samples_array_yields_nothing() {
        let v: serde_json::Value = serde_json::from_str(r#"{"version":2}"#).unwrap();
        assert!(parse_desktop_usage_history(&v, 0, SnapshotState::Stale).is_none());
    }
    #[test]
    fn reads_desktop_usage_history_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plan-usage-history.json");
        std::fs::write(
            &path,
            r#"{"version":2,"samples":[{"t":1,"org":"x","u":{"fh":5,"sd":20}}]}"#,
        )
        .unwrap();
        let value = read_desktop_usage_history(&path).unwrap();
        let snapshot = parse_desktop_usage_history(&value, 0, SnapshotState::Stale).unwrap();
        assert_eq!(snapshot.windows[0].used_percent, 5.0);
    }
    #[test]
    fn a_missing_desktop_usage_history_file_reads_as_nothing_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_desktop_usage_history(&dir.path().join("missing.json")).is_none());
    }
}
