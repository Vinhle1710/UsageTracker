use super::{AccountKind, AccountStatus, AccountSummary, CredentialSource};
use crate::creds::{claude_oauth_from_str, TokenError};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryResult {
    NotInstalled,
    Invalid,
    SignedIn(AccountSummary),
    Expired(AccountSummary),
}

pub fn discover_claude_code(path: &Path, now_ms: i64) -> DiscoveryResult {
    let bytes = match std::fs::read(path) {
        Ok(v) => v,
        Err(_) => return DiscoveryResult::NotInstalled,
    };
    let text = match std::str::from_utf8(&bytes) {
        Ok(v) => v,
        Err(_) => return DiscoveryResult::Invalid,
    };
    let creds = match claude_oauth_from_str(text) {
        Ok(v) => v,
        Err(TokenError::NotFound) => return DiscoveryResult::NotInstalled,
        Err(_) => return DiscoveryResult::Invalid,
    };
    let mut hash = Sha256::new();
    hash.update(path.to_string_lossy().as_bytes());
    let suffix = hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let id = format!("claude-code:{}", &suffix[..16]);
    let status = if creds.needs_refresh(now_ms) {
        AccountStatus::NeedsReauthentication
    } else {
        AccountStatus::SignedIn
    };
    let summary = AccountSummary {
        id,
        kind: AccountKind::ClaudeAi,
        source: CredentialSource::ClaudeCode,
        email: None,
        status,
        credential_hint: None,
    };
    if status == AccountStatus::NeedsReauthentication {
        DiscoveryResult::Expired(summary)
    } else {
        DiscoveryResult::SignedIn(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn missing_is_not_installed() {
        let d = tempdir().unwrap();
        assert_eq!(
            discover_claude_code(&d.path().join("x"), 0),
            DiscoveryResult::NotInstalled
        );
    }
    #[test]
    fn valid_is_read_only() {
        let d = tempdir().unwrap();
        let p = d.path().join("c");
        let s = r#"{"claudeAiOauth":{"accessToken":"fixture","expiresAt":999999999999}}"#;
        std::fs::write(&p, s).unwrap();
        let before = std::fs::read(&p).unwrap();
        assert!(matches!(
            discover_claude_code(&p, 0),
            DiscoveryResult::SignedIn(_)
        ));
        assert_eq!(before, std::fs::read(&p).unwrap());
    }
    #[test]
    fn expired_is_reauth() {
        let d = tempdir().unwrap();
        let p = d.path().join("c");
        std::fs::write(
            &p,
            r#"{"claudeAiOauth":{"accessToken":"fixture","expiresAt":1}}"#,
        )
        .unwrap();
        assert!(matches!(
            discover_claude_code(&p, 1000),
            DiscoveryResult::Expired(_)
        ));
    }
}
