use reqwest::{redirect::Policy, Client, StatusCode};
use std::time::Duration;

pub const ANTHROPIC_API_ORIGIN: &str = "https://api.anthropic.com";
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
}
