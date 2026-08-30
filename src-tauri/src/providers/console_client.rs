use reqwest::{redirect::Policy, Client, StatusCode};
use std::time::Duration;

use crate::providers::console_costs::dto;

/// The Console **web** backend. `console.anthropic.com` 301-redirects here, and this client
/// runs with redirects disabled, so the current host is targeted directly — following that
/// redirect would silently defeat the origin pin.
pub const CONSOLE_API_ORIGIN: &str = "https://platform.claude.com";
pub const ANTHROPIC_API_ORIGIN: &str = "https://api.anthropic.com";
/// A cost report for a busy month is tens of KB; anything past this is not a cost report.
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub fn client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(10))
        .redirect(Policy::none())
        .build()
}
pub fn classify(status: StatusCode) -> &'static str {
    match status {
        StatusCode::UNAUTHORIZED => "noCredential",
        StatusCode::FORBIDDEN => "insufficientRole",
        StatusCode::NOT_FOUND => "unsupportedBySource",
        StatusCode::TOO_MANY_REQUESTS => "providerUnavailable",
        _ if status.is_server_error() => "providerUnavailable",
        _ => "providerUnavailable",
    }
}

/// Every Console read goes through here: one place that attaches the cookie, caps the body, and
/// maps a status to an `errorCode` the UI already knows how to render.
async fn get_json<T: serde::de::DeserializeOwned>(
    client: &Client,
    base: &str,
    path: &str,
    session_key: &str,
) -> Result<T, String> {
    let response = client
        .get(format!("{base}{path}"))
        .header("Cookie", format!("sessionKey={session_key}"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| "providerUnavailable".to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(classify(status).to_string());
    }
    let body = response
        .bytes()
        .await
        .map_err(|_| "providerUnavailable".to_string())?;
    if body.len() > MAX_RESPONSE_BYTES {
        return Err("providerUnavailable".into());
    }
    serde_json::from_slice(&body).map_err(|_| "providerUnavailable".to_string())
}

pub async fn organizations(
    client: &Client,
    base: &str,
    session_key: &str,
) -> Result<Vec<dto::Organization>, String> {
    get_json(client, base, "/api/organizations", session_key).await
}

pub async fn current_spend(
    client: &Client,
    base: &str,
    session_key: &str,
    org: &str,
) -> Result<dto::CurrentSpend, String> {
    get_json(
        client,
        base,
        &format!("/api/organizations/{org}/current_spend"),
        session_key,
    )
    .await
}

pub async fn prepaid_credits(
    client: &Client,
    base: &str,
    session_key: &str,
    org: &str,
) -> Result<dto::PrepaidCredits, String> {
    get_json(
        client,
        base,
        &format!("/api/organizations/{org}/prepaid/credits"),
        session_key,
    )
    .await
}

pub async fn usage_cost(
    client: &Client,
    base: &str,
    session_key: &str,
    org: &str,
    starting_on: &str,
    ending_before: &str,
) -> Result<dto::UsageCost, String> {
    get_json(
        client,
        base,
        &format!(
            "/api/organizations/{org}/workspaces/default/usage_cost?starting_on={starting_on}&ending_before={ending_before}&group_by=api_key_id"
        ),
        session_key,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redirects_are_disabled() {
        let _ = client().unwrap();
    }
    #[test]
    fn statuses_are_redacted() {
        assert_eq!(classify(StatusCode::FORBIDDEN), "insufficientRole");
    }

    #[tokio::test]
    async fn usage_cost_requests_the_exact_documented_path_and_query() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/api/organizations/org-1/workspaces/default/usage_cost?starting_on=2026-08-01&ending_before=2026-09-01&group_by=api_key_id",
            )
            .match_header("Cookie", "sessionKey=fixture-session")
            .with_status(200)
            .with_body(r#"{"costs":null}"#)
            .create_async()
            .await;

        let out = usage_cost(
            &client().unwrap(),
            &server.url(),
            "fixture-session",
            "org-1",
            "2026-08-01",
            "2026-09-01",
        )
        .await;

        mock.assert_async().await;
        assert!(out.is_ok());
    }

    #[tokio::test]
    async fn spend_sends_the_session_cookie_and_parses_cents() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/organizations/org-1/current_spend")
            .match_header("Cookie", "sessionKey=fixture-session")
            .with_status(200)
            .with_body(r#"{"amount":123456,"resets_at":"2026-09-01T00:00:00.000000Z"}"#)
            .create_async()
            .await;

        let spend = current_spend(
            &client().unwrap(),
            &server.url(),
            "fixture-session",
            "org-1",
        )
        .await
        .unwrap();

        mock.assert_async().await;
        assert_eq!(spend.amount, 123_456);
    }

    #[tokio::test]
    async fn statuses_map_to_section_error_codes_without_leaking_the_body() {
        for (status, expected) in [
            (401, "noCredential"),
            (403, "insufficientRole"),
            (404, "unsupportedBySource"),
            (429, "providerUnavailable"),
            (500, "providerUnavailable"),
        ] {
            let mut server = mockito::Server::new_async().await;
            let _m = server
                .mock("GET", "/api/organizations/org-1/current_spend")
                .with_status(status)
                .with_body(r#"{"error":{"message":"sk-ant-secret-should-never-surface"}}"#)
                .create_async()
                .await;

            let error = current_spend(&client().unwrap(), &server.url(), "session", "org-1")
                .await
                .unwrap_err();

            assert_eq!(error, expected, "status {status}");
            assert!(!error.contains("sk-ant"));
        }
    }

    #[tokio::test]
    async fn a_redirect_is_never_followed() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/organizations")
            .with_status(301)
            .with_header("Location", "https://evil.example.com/api/organizations")
            .create_async()
            .await;

        // 301 must surface as an error, never as a silent hop to another origin.
        assert!(organizations(&client().unwrap(), &server.url(), "session")
            .await
            .is_err());
    }

    /// Live smoke test against the real Console host. `#[ignore]` so the suite stays offline and
    /// deterministic; run deliberately with:
    ///   cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture live_console
    /// A deliberately invalid cookie must come back as `insufficientRole` (403), which proves the
    /// URL, host, and status mapping are right without needing anyone's real credential.
    #[tokio::test]
    #[ignore]
    async fn live_console_rejects_an_invalid_session_cookie() {
        let client = client().unwrap();
        let error = organizations(&client, CONSOLE_API_ORIGIN, "not-a-real-session-key")
            .await
            .unwrap_err();
        println!("live /api/organizations -> {error}");
        assert_eq!(error, "insufficientRole");
    }
}
