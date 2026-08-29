use crate::model::{SnapshotState, UsageSnapshot, UsageWindow};
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

const CLAUDE_CODE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const OAUTH_BETA: &str = "oauth-2025-04-20";

/// The same public client this app already uses for token refresh, now also driving a full
/// authorization-code+PKCE login — for a user who has only ever used the Claude desktop app,
/// this is what actually creates `.claude/.credentials.json` for the first time.
const AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
pub const LOGIN_TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
const REDIRECT_URI: &str = "https://platform.claude.com/oauth/code/callback";
const LOGIN_SCOPE: &str =
    "user:inference user:profile user:sessions:claude_code user:mcp_servers user:file_upload";

/// A PKCE (RFC 7636) verifier/challenge pair for one login attempt. `verifier` never leaves this
/// process until the final token exchange; only `challenge` (its SHA-256 hash) goes in the
/// browser-facing authorize URL, so a party that only observes that URL can't forge the code
/// exchange.
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

fn random_url_safe_token(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_pkce() -> PkceChallenge {
    let verifier = random_url_safe_token(32);
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    PkceChallenge {
        verifier,
        challenge,
    }
}

/// A random CSRF token, independent of the PKCE verifier, sent as `state` and echoed back by
/// Anthropic's callback page.
pub fn generate_state() -> String {
    random_url_safe_token(24)
}

pub fn build_authorize_url(code_challenge: &str, state: &str) -> String {
    let mut url = url::Url::parse(AUTHORIZE_URL).expect("AUTHORIZE_URL is a valid constant URL");
    url.query_pairs_mut()
        .append_pair("code", "true")
        .append_pair("client_id", CLAUDE_CODE_CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("scope", LOGIN_SCOPE)
        .append_pair("code_challenge", code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state);
    url.to_string()
}

/// Anthropic's callback page displays the authorization result as `CODE#STATE` for the user to
/// copy — some accounts only ever show a bare code with no `#STATE` suffix, so a pasted value
/// with no `#` is treated as the code alone, leaving the caller to fall back to whatever state
/// it already remembers from `generate_state`.
pub fn parse_pasted_code(pasted: &str) -> (String, Option<String>) {
    let trimmed = pasted.trim();
    match trimmed.split_once('#') {
        Some((code, state)) => (code.to_string(), Some(state.to_string())),
        None => (trimmed.to_string(), None),
    }
}

#[derive(serde::Deserialize)]
struct LoginTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    #[serde(default)]
    scope: Option<String>,
}

pub async fn exchange_code_for_tokens(
    client: &reqwest::Client,
    url: &str,
    code: &str,
    state: &str,
    code_verifier: &str,
) -> Result<crate::creds::ClaudeLoginTokens, super::FetchError> {
    let response = client
        .post(url)
        .header("anthropic-beta", OAUTH_BETA)
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
            "state": state,
            "client_id": CLAUDE_CODE_CLIENT_ID,
            "redirect_uri": REDIRECT_URI,
            "code_verifier": code_verifier,
        }))
        .send()
        .await
        .map_err(|_| super::FetchError::Network)?;
    if let Some(error) = super::classify_status(response.status().as_u16()) {
        return Err(error);
    }
    let parsed: LoginTokenResponse = response
        .json()
        .await
        .map_err(|_| super::FetchError::Malformed)?;
    let scopes = parsed
        .scope
        .as_deref()
        .unwrap_or(LOGIN_SCOPE)
        .split_whitespace()
        .map(str::to_string)
        .collect();
    Ok(crate::creds::ClaudeLoginTokens {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expires_in: parsed.expires_in,
        scopes,
    })
}

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

/// Extracts the signed-in account's email from `GET /api/oauth/profile`'s response — the OAuth
/// credential file itself never carries an email (confirmed against a real `.credentials.json`,
/// which had only token/scope fields), so this is the only source for it. Confirmed against a
/// real response body: `account.email`, not `account.email_address` as some third-party writeups
/// of this undocumented endpoint claim.
pub fn parse_profile_email(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/account/email")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
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

    #[test]
    fn generated_pkce_challenge_is_the_sha256_of_the_verifier() {
        let pkce = generate_pkce();
        assert!(pkce.verifier.len() >= 32);
        let expected = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(pkce.verifier.as_bytes()));
        assert_eq!(pkce.challenge, expected);
    }

    #[test]
    fn two_pkce_challenges_never_collide() {
        // Not a cryptographic proof, just a guard against an RNG that got wired up wrong
        // (e.g. always seeded the same way, or generating from a fixed buffer).
        assert_ne!(generate_pkce().verifier, generate_pkce().verifier);
        assert_ne!(generate_state(), generate_state());
    }

    #[test]
    fn authorize_url_carries_every_parameter_the_token_exchange_needs_to_match() {
        let url = build_authorize_url("challenge-abc", "state-xyz");
        let parsed = url::Url::parse(&url).unwrap();
        assert_eq!(parsed.host_str(), Some("claude.ai"));
        let pairs: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        assert_eq!(pairs.get("client_id").unwrap(), CLAUDE_CODE_CLIENT_ID);
        assert_eq!(pairs.get("response_type").unwrap(), "code");
        assert_eq!(pairs.get("code_challenge").unwrap(), "challenge-abc");
        assert_eq!(pairs.get("code_challenge_method").unwrap(), "S256");
        assert_eq!(pairs.get("state").unwrap(), "state-xyz");
        assert_eq!(pairs.get("redirect_uri").unwrap(), REDIRECT_URI);
    }

    #[test]
    fn parses_a_code_and_state_pasted_together() {
        assert_eq!(
            parse_pasted_code("abc123#xyz789"),
            ("abc123".to_string(), Some("xyz789".to_string()))
        );
    }

    #[test]
    fn parses_a_bare_code_with_no_state_suffix() {
        assert_eq!(
            parse_pasted_code("  abc123  "),
            ("abc123".to_string(), None)
        );
    }

    #[tokio::test]
    async fn exchanges_an_authorization_code_for_a_fresh_login() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/token")
            .match_body(mockito::Matcher::Json(serde_json::json!({
                "grant_type": "authorization_code",
                "code": "the-code",
                "state": "the-state",
                "client_id": CLAUDE_CODE_CLIENT_ID,
                "redirect_uri": REDIRECT_URI,
                "code_verifier": "the-verifier",
            })))
            .match_header("anthropic-beta", "oauth-2025-04-20")
            .with_status(200)
            .with_body(
                r#"{"access_token":"new-access","refresh_token":"new-refresh","expires_in":3600}"#,
            )
            .create_async()
            .await;

        let tokens = exchange_code_for_tokens(
            &reqwest::Client::new(),
            &format!("{}/token", server.url()),
            "the-code",
            "the-state",
            "the-verifier",
        )
        .await
        .unwrap();

        assert_eq!(tokens.access_token, "new-access");
        assert_eq!(tokens.refresh_token, "new-refresh");
        assert!(tokens.scopes.contains(&"user:inference".to_string()));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn a_rejected_authorization_code_reports_unauthorized() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/token")
            .with_status(401)
            .create_async()
            .await;

        let result = exchange_code_for_tokens(
            &reqwest::Client::new(),
            &format!("{}/token", server.url()),
            "bad-code",
            "state",
            "verifier",
        )
        .await;

        assert_eq!(result.unwrap_err(), FetchError::Unauthorized);
        mock.assert_async().await;
    }

    #[test]
    fn parses_the_email_address_out_of_the_profile_response() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"account":{"email":"person@example.com","uuid":"acc-1"}}"#)
                .unwrap();
        assert_eq!(
            parse_profile_email(&v).as_deref(),
            Some("person@example.com")
        );
    }
    #[test]
    fn a_profile_response_with_no_email_field_yields_nothing() {
        let v: serde_json::Value = serde_json::from_str(r#"{"account":{"uuid":"acc-1"}}"#).unwrap();
        assert!(parse_profile_email(&v).is_none());
    }
    #[test]
    fn a_blank_email_in_the_profile_response_yields_nothing() {
        let v: serde_json::Value = serde_json::from_str(r#"{"account":{"email":""}}"#).unwrap();
        assert!(parse_profile_email(&v).is_none());
    }

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
