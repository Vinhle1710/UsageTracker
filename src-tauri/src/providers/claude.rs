use crate::model::{SnapshotState, UsageSnapshot, UsageWindow};

const CLAUDE_CODE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const OAUTH_BETA: &str = "oauth-2025-04-20";

pub async fn refresh_access_token(
    client: &reqwest::Client,
    url: &str,
    refresh_token: &str,
) -> Result<crate::creds::ClaudeTokenRefresh, super::FetchError> {
    let response = client
        .post(url)
        .header("anthropic-beta", OAUTH_BETA)
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": CLAUDE_CODE_CLIENT_ID,
        }))
        .send()
        .await
        .map_err(|_| super::FetchError::Network)?;
    if let Some(error) = super::classify_status(response.status().as_u16()) {
        return Err(error);
    }
    response
        .json()
        .await
        .map_err(|_| super::FetchError::Malformed)
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

    #[tokio::test]
    async fn refreshes_an_expired_oauth_token_with_the_claude_code_contract() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/token")
            .match_body(mockito::Matcher::Json(serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": "refresh-123",
                "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
            })))
            .match_header("anthropic-beta", "oauth-2025-04-20")
            .with_status(200)
            .with_body(
                r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":3600}"#,
            )
            .create_async()
            .await;

        let refreshed = refresh_access_token(
            &reqwest::Client::new(),
            &format!("{}/token", server.url()),
            "refresh-123",
        )
        .await
        .unwrap();

        assert_eq!(refreshed.access_token, "new-access");
        assert_eq!(refreshed.refresh_token.as_deref(), Some("new-refresh"));
        mock.assert_async().await;
    }
}
