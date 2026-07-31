pub mod claude;
pub mod codex;

use crate::model::SnapshotState;

#[derive(Debug, PartialEq)]
pub enum FetchError {
    Unauthorized,
    Network,
    Malformed,
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
    }
}

pub async fn fetch_json(
    url: &str,
    token: &str,
    extra: &[(&str, &str)],
) -> Result<serde_json::Value, FetchError> {
    let client = reqwest::Client::new();
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
        let result = fetch_json(&format!("{}/u", server.url()), "tok", &[]).await;
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
            fetch_json(&format!("{}/u", server.url()), "tok", &[])
                .await
                .unwrap_err(),
            FetchError::Malformed
        );
        mock.assert_async().await;
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
            fetch_json(&format!("{}/u", server.url()), "tok", &[])
                .await
                .unwrap()["ok"],
            true
        );
        mock.assert_async().await;
    }
}
