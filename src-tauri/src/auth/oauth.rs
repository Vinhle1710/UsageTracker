use base64::Engine;
use sha2::{Digest, Sha256};
fn constant_time_equal(a: &str, b: &str) -> bool {
    let mut diff = (a.len() ^ b.len()) as u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub struct OAuthAttempt {
    pub verifier: String,
    pub state: String,
    created_ms: i64,
}
pub struct OAuthAttemptStore {
    attempt: Option<OAuthAttempt>,
}
impl Default for OAuthAttemptStore {
    fn default() -> Self {
        Self::new()
    }
}
impl OAuthAttemptStore {
    pub fn new() -> Self {
        Self { attempt: None }
    }
    pub fn begin(&mut self, verifier: String, state: String, now_ms: i64) {
        self.attempt = Some(OAuthAttempt {
            verifier,
            state,
            created_ms: now_ms,
        });
    }
    pub fn consume(&mut self, state: &str, now_ms: i64) -> Option<String> {
        let a = self.attempt.take()?;
        if now_ms.saturating_sub(a.created_ms) > 600_000 {
            return None;
        }
        if !constant_time_equal(&a.state, state) {
            return None;
        }
        Some(a.verifier)
    }
}
pub fn pkce_challenge(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
pub enum NavigationDecision {
    Allow,
    External,
    Block,
}
pub fn navigation_policy(target: &str, callback: &str) -> NavigationDecision {
    let Ok(url) = url::Url::parse(target) else {
        return NavigationDecision::Block;
    };
    if target == callback {
        return NavigationDecision::Allow;
    }
    if url.scheme() == "http" && url.host_str() == Some("127.0.0.1") {
        return NavigationDecision::Allow;
    }
    if url.scheme() != "https" {
        return NavigationDecision::Block;
    }
    let host = url.host_str().unwrap_or("");
    if host == "claude.ai"
        || host == "accounts.google.com"
        || host == "anthropic.com"
        || host.ends_with(".anthropic.com")
    {
        NavigationDecision::Allow
    } else {
        NavigationDecision::External
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn state_single_use_and_expiry() {
        let mut s = OAuthAttemptStore::new();
        s.begin("v".into(), "s".into(), 0);
        assert_eq!(s.consume("s", 1), Some("v".into()));
        assert_eq!(s.consume("s", 1), None);
        s.begin("v".into(), "s".into(), 0);
        assert_eq!(s.consume("s", 600_001), None);
    }
    #[test]
    fn policy() {
        assert!(matches!(
            navigation_policy("https://claude.ai/x", "https://callback"),
            NavigationDecision::Allow
        ));
        assert!(matches!(
            navigation_policy("https://evil.test", "https://callback"),
            NavigationDecision::External
        ));
        assert!(matches!(
            navigation_policy("file:///x", "https://callback"),
            NavigationDecision::Block
        ));
    }
    #[test]
    fn challenge() {
        assert!(!pkce_challenge("v").is_empty());
    }
}
