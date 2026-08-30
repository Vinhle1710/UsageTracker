use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum TokenError {
    NotFound,
    Unreadable,
    Malformed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeOauthCredentials {
    pub access_token: String,
    pub expires_at: Option<i64>,
    /// Validated local metadata for display and org-scoped claude.ai session-cookie requests.
    pub organization_uuid: Option<String>,
}

impl ClaudeOauthCredentials {
    pub fn needs_refresh(&self, now_millis: i64) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= now_millis + 30_000)
    }
}

fn valid_organization_uuid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub fn claude_oauth_from_str(s: &str) -> Result<ClaudeOauthCredentials, TokenError> {
    let value: serde_json::Value = serde_json::from_str(s).map_err(|_| TokenError::Malformed)?;
    // No `claudeAiOauth` key at all (and no legacy top-level `accessToken`) means this machine
    // has no recorded Claude session — distinct from a key that exists but is broken, which is
    // a real re-authentication problem rather than a "never signed in" one.
    if value.get("claudeAiOauth").is_none() && value.get("accessToken").is_none() {
        return Err(TokenError::NotFound);
    }
    let access_token = value
        .pointer("/claudeAiOauth/accessToken")
        .or_else(|| value.pointer("/accessToken"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or(TokenError::Malformed)?;
    Ok(ClaudeOauthCredentials {
        access_token: access_token.into(),
        expires_at: value
            .pointer("/claudeAiOauth/expiresAt")
            .and_then(|value| value.as_i64()),
        organization_uuid: value
            .get("organizationUuid")
            .and_then(|value| value.as_str())
            .filter(|value| valid_organization_uuid(value))
            .map(str::to_string),
    })
}

pub fn claude_token_from_str(s: &str) -> Result<String, TokenError> {
    claude_oauth_from_str(s).map(|auth| auth.access_token)
}

/// `chatgpt.com/backend-api/codex/usage` requires both the bearer token and the
/// `chatgpt-account-id` header — omitting the account id gets a WAF-level 403, not a clean
/// API response, because the request can't be routed to an account.
#[derive(Debug, Clone, PartialEq)]
pub struct CodexCredentials {
    pub access_token: String,
    pub account_id: String,
}

pub fn codex_credentials_from_str(s: &str) -> Result<CodexCredentials, TokenError> {
    let value: serde_json::Value = serde_json::from_str(s).map_err(|_| TokenError::Malformed)?;
    let access_token = value
        .pointer("/tokens/access_token")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or(TokenError::Malformed)?;
    let account_id = value
        .pointer("/tokens/account_id")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or(TokenError::Malformed)?;
    Ok(CodexCredentials {
        access_token: access_token.into(),
        account_id: account_id.into(),
    })
}

pub fn read_codex_credentials(path: &Path) -> Result<CodexCredentials, TokenError> {
    if !path.exists() {
        return Err(TokenError::NotFound);
    }
    codex_credentials_from_str(&std::fs::read_to_string(path).map_err(|_| TokenError::Unreadable)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn extracts_codex_credentials() {
        let credentials = codex_credentials_from_str(
            r#"{"tokens":{"access_token":"abc123","account_id":"acct-1"}}"#,
        )
        .unwrap();
        assert_eq!(credentials.access_token, "abc123");
        assert_eq!(credentials.account_id, "acct-1");
    }
    #[test]
    fn rejects_codex_credentials_missing_account_id() {
        assert_eq!(
            codex_credentials_from_str(r#"{"tokens":{"access_token":"abc123"}}"#),
            Err(TokenError::Malformed)
        );
    }
    #[test]
    fn extracts_nested_claude_token() {
        assert_eq!(
            claude_token_from_str(r#"{"claudeAiOauth":{"accessToken":"xyz789"}}"#).unwrap(),
            "xyz789"
        );
    }
    #[test]
    fn reads_claude_oauth_expiry_without_taking_ownership_of_refresh() {
        let auth = claude_oauth_from_str(
            r#"{"claudeAiOauth":{"accessToken":"old","refreshToken":"refresh","expiresAt":1000}}"#,
        )
        .unwrap();
        assert_eq!(auth.access_token, "old");
        assert_eq!(auth.expires_at, Some(1000));
    }

    #[test]
    fn reads_the_organization_uuid_for_account_display() {
        let auth = claude_oauth_from_str(
            r#"{"claudeAiOauth":{"accessToken":"old"},"organizationUuid":"org-1"}"#,
        )
        .unwrap();
        assert_eq!(auth.organization_uuid.as_deref(), Some("org-1"));
    }

    #[test]
    fn rejects_an_organization_id_that_could_escape_an_http_path_segment() {
        for organization_uuid in ["../usage", "org/other", "org?admin=true", "org#fragment"] {
            let input = format!(
                r#"{{"claudeAiOauth":{{"accessToken":"old"}},"organizationUuid":"{organization_uuid}"}}"#
            );
            let auth = claude_oauth_from_str(&input).unwrap();
            assert_eq!(auth.organization_uuid, None, "accepted {organization_uuid}");
        }
    }

    #[test]
    fn no_claude_oauth_key_at_all_is_not_found_rather_than_malformed() {
        // Distinct from a `claudeAiOauth` key that exists but can't be used: this machine has
        // simply never recorded a Claude session, so it should read as "never signed in".
        assert_eq!(
            claude_oauth_from_str(r#"{"mcpOAuth":{"server":"kept"}}"#),
            Err(TokenError::NotFound)
        );
        assert_eq!(claude_oauth_from_str("{}"), Err(TokenError::NotFound));
    }

    #[test]
    fn a_present_but_broken_claude_oauth_key_stays_malformed() {
        assert_eq!(
            claude_oauth_from_str(r#"{"claudeAiOauth":{"accessToken":""}}"#),
            Err(TokenError::Malformed)
        );
    }

    #[test]
    fn rejects_empty_codex_token() {
        assert_eq!(
            codex_credentials_from_str(r#"{"tokens":{"access_token":"","account_id":"acct-1"}}"#),
            Err(TokenError::Malformed)
        );
    }
    #[test]
    fn rejects_malformed_codex_json() {
        assert_eq!(
            codex_credentials_from_str("{oops"),
            Err(TokenError::Malformed)
        );
    }
    #[test]
    fn missing_codex_file_reports_not_found() {
        let d = tempdir().unwrap();
        assert_eq!(
            read_codex_credentials(&d.path().join("none")),
            Err(TokenError::NotFound)
        );
    }
    #[test]
    fn error_debug_contains_no_token() {
        let err = format!("{:?}", TokenError::Malformed);
        assert!(!err.contains("SUPERSECRET"));
    }
}
