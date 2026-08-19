pub mod claude;
pub mod claude_status;
pub mod claude_usage;
pub mod codex;
pub mod console_client;
pub mod console_costs;

use crate::model::SnapshotState;

#[derive(Debug, PartialEq)]
pub enum FetchError {
    Unauthorized,
    Network,
    Malformed,
    /// The credential file is absent entirely, so there is nothing to authenticate with. Only a
    /// local file check can produce this — a served 401 means credentials existed and were
    /// refused, which is `Unauthorized`.
    SignedOut,
}

pub fn classify_status(status: u16) -> Option<FetchError> {
    match status {
        200..=299 => None,
        401 | 403 => Some(FetchError::Unauthorized),
        _ => Some(FetchError::Network),
    }
}
pub fn state_for_error(error: &FetchError) -> SnapshotState {
    match error {
        FetchError::Unauthorized | FetchError::Malformed => SnapshotState::Error,
        FetchError::Network => SnapshotState::Stale,
        FetchError::SignedOut => SnapshotState::SignedOut,
    }
}

/// Numbers recovered from a non-2xx body are real and current, but the request itself did not
/// succeed, so they are surfaced dimmed rather than as a clean read.
pub fn state_for_status(status: u16) -> SnapshotState {
    match classify_status(status) {
        None => SnapshotState::Fresh,
        Some(error) => state_for_error(&error),
    }
}

/// A response that reached the server, keeping the body regardless of status. Error responses
/// from the usage endpoints frequently still carry the usage payload, and discarding it is what
/// turns a readable number into "temporarily unavailable".
#[derive(Debug, PartialEq)]
pub struct FetchResponse {
    pub status: u16,
    pub body: Option<serde_json::Value>,
    pub headers: std::collections::BTreeMap<String, String>,
}

pub async fn fetch_response(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    extra: &[(&str, &str)],
) -> Result<FetchResponse, FetchError> {
    let mut request = client.get(url).bearer_auth(token);
    for (key, value) in extra {
        request = request.header(*key, *value);
    }
    let response = request.send().await.map_err(|_| FetchError::Network)?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            let key = name.as_str().to_ascii_lowercase();
            // This allowlist is intentionally empty until a redacted, verified provider fixture
            // records an exact header contract.
            let allowed = [""].contains(&key.as_str());
            allowed
                .then(|| Some((key, value.to_str().ok()?.to_string())))
                .flatten()
        })
        .collect();
    Ok(FetchResponse {
        status,
        body: response.json().await.ok(),
        headers,
    })
}

pub async fn fetch_json(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    extra: &[(&str, &str)],
) -> Result<serde_json::Value, FetchError> {
    let mut request = client.get(url).bearer_auth(token);
    for (key, value) in extra {
        request = request.header(*key, *value);
    }
    let response = request.send().await.map_err(|_| FetchError::Network)?;
    if let Some(error) = classify_status(response.status().as_u16()) {
        return Err(error);
    }
    response.json().await.map_err(|_| FetchError::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn success_status_is_not_error() {
        assert_eq!(classify_status(200), None);
    }
    #[test]
    fn unauthorized_is_error() {
        let e = classify_status(401).unwrap();
        assert_eq!(state_for_error(&e), SnapshotState::Error);
    }
    #[test]
    fn an_absent_credential_file_is_signed_out_rather_than_an_auth_failure() {
        assert_eq!(
            state_for_error(&FetchError::SignedOut),
            SnapshotState::SignedOut
        );
    }

    #[test]
    fn a_rejected_token_stays_an_auth_failure_rather_than_reading_as_signed_out() {
        // A served 401 means credentials exist and were refused. Telling that user to "sign in"
        // when they already are would send them down the wrong path.
        assert_eq!(classify_status(401), Some(FetchError::Unauthorized));
        assert_eq!(state_for_status(401), SnapshotState::Error);
    }

    #[test]
    fn rate_limit_is_stale() {
        let e = classify_status(429).unwrap();
        assert_eq!(state_for_error(&e), SnapshotState::Stale);
    }
    #[test]
    fn server_error_is_stale() {
        let e = classify_status(500).unwrap();
        assert_eq!(state_for_error(&e), SnapshotState::Stale);
    }
    #[tokio::test]
    async fn fetch_returns_unauthorized_on_401() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/u")
            .with_status(401)
            .create_async()
            .await;
        let result = fetch_json(
            &reqwest::Client::new(),
            &format!("{}/u", server.url()),
            "tok",
            &[],
        )
        .await;
        assert_eq!(result.unwrap_err(), FetchError::Unauthorized);
        mock.assert_async().await;
    }
    #[tokio::test]
    async fn fetch_returns_malformed_on_bad_body() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/u")
            .with_status(200)
            .with_body("bad")
            .create_async()
            .await;
        assert_eq!(
            fetch_json(
                &reqwest::Client::new(),
                &format!("{}/u", server.url()),
                "tok",
                &[]
            )
            .await
            .unwrap_err(),
            FetchError::Malformed
        );
        mock.assert_async().await;
    }
    #[tokio::test]
    async fn a_rate_limited_response_keeps_its_usage_body_instead_of_discarding_it() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/u")
            .with_status(429)
            .with_body(r#"{"rate_limits":{"primary":{"used_percent":100.0,"window_minutes":300}}}"#)
            .create_async()
            .await;

        let response = fetch_response(
            &reqwest::Client::new(),
            &format!("{}/u", server.url()),
            "tok",
            &[],
        )
        .await
        .expect("a served error response is not a transport failure");

        assert_eq!(response.status, 429);
        assert_eq!(
            response.body.expect("body retained")["rate_limits"]["primary"]["used_percent"],
            100.0
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn an_error_response_without_a_body_reports_its_status() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/u")
            .with_status(503)
            .with_body("upstream down")
            .create_async()
            .await;

        let response = fetch_response(
            &reqwest::Client::new(),
            &format!("{}/u", server.url()),
            "tok",
            &[],
        )
        .await
        .expect("a served error response is not a transport failure");

        assert_eq!(response.status, 503);
        assert_eq!(response.body, None);
        mock.assert_async().await;
    }

    #[test]
    fn recovered_error_body_numbers_are_marked_stale_but_clean_reads_stay_fresh() {
        assert_eq!(state_for_status(200), SnapshotState::Fresh);
        assert_eq!(state_for_status(429), SnapshotState::Stale);
        assert_eq!(state_for_status(500), SnapshotState::Stale);
        // An auth failure stays actionable rather than being softened into "temporarily stale".
        assert_eq!(state_for_status(401), SnapshotState::Error);
        assert_eq!(state_for_status(403), SnapshotState::Error);
    }

    #[tokio::test]
    async fn fetch_parses_successful_body() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/u")
            .with_status(200)
            .with_body(r#"{"ok":true}"#)
            .create_async()
            .await;
        assert_eq!(
            fetch_json(
                &reqwest::Client::new(),
                &format!("{}/u", server.url()),
                "tok",
                &[]
            )
            .await
            .unwrap()["ok"],
            true
        );
        mock.assert_async().await;
    }
}
