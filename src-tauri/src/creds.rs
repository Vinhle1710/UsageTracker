use std::path::Path;
use std::{io::Write, path::PathBuf};

#[derive(Debug, PartialEq)]
pub enum TokenError {
    NotFound,
    Unreadable,
    Malformed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeOauthCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    /// The account this session belongs to, for display only — never used to authenticate.
    pub organization_uuid: Option<String>,
}

impl ClaudeOauthCredentials {
    pub fn needs_refresh(&self, now_millis: i64) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= now_millis + 30_000)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ClaudeTokenRefresh {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    #[serde(default)]
    pub refresh_token_expires_in: Option<i64>,
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
        refresh_token: value
            .pointer("/claudeAiOauth/refreshToken")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        expires_at: value
            .pointer("/claudeAiOauth/expiresAt")
            .and_then(|value| value.as_i64()),
        organization_uuid: value
            .get("organizationUuid")
            .and_then(|value| value.as_str())
            .map(str::to_string),
    })
}

pub fn claude_token_from_str(s: &str) -> Result<String, TokenError> {
    claude_oauth_from_str(s).map(|auth| auth.access_token)
}

pub fn merge_claude_refresh(
    original: &str,
    refresh: &ClaudeTokenRefresh,
    now_millis: i64,
) -> Result<String, TokenError> {
    let mut value: serde_json::Value =
        serde_json::from_str(original).map_err(|_| TokenError::Malformed)?;
    let oauth = value
        .get_mut("claudeAiOauth")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or(TokenError::Malformed)?;
    oauth.insert("accessToken".into(), refresh.access_token.clone().into());
    oauth.insert(
        "expiresAt".into(),
        (now_millis + refresh.expires_in.saturating_mul(1_000)).into(),
    );
    if let Some(refresh_token) = &refresh.refresh_token {
        oauth.insert("refreshToken".into(), refresh_token.clone().into());
    }
    if let Some(expires_in) = refresh.refresh_token_expires_in {
        oauth.insert(
            "refreshTokenExpiresAt".into(),
            (now_millis + expires_in.saturating_mul(1_000)).into(),
        );
    }
    serde_json::to_string_pretty(&value).map_err(|_| TokenError::Malformed)
}

/// Writes `contents` to `path` via temp-file-plus-rename so a crash or concurrent read never
/// observes a half-written credentials file.
fn atomic_write(path: &Path, contents: &str) -> Result<(), TokenError> {
    let parent: PathBuf = path.parent().ok_or(TokenError::Unreadable)?.into();
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| TokenError::Unreadable)?;
    temporary
        .write_all(contents.as_bytes())
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| TokenError::Unreadable)?;
    temporary
        .persist(path)
        .map_err(|_| TokenError::Unreadable)?;
    Ok(())
}

pub fn persist_claude_refresh(
    path: &Path,
    refresh: &ClaudeTokenRefresh,
    now_millis: i64,
) -> Result<ClaudeOauthCredentials, TokenError> {
    let original = std::fs::read_to_string(path).map_err(|_| TokenError::Unreadable)?;
    let merged = merge_claude_refresh(&original, refresh, now_millis)?;
    atomic_write(path, &merged)?;
    claude_oauth_from_str(&merged)
}

/// The token pair a fresh OAuth authorization-code exchange returns. Distinct from
/// `ClaudeTokenRefresh` because a first-time login always has a refresh token and a known scope
/// grant, where a refresh response may omit either.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeLoginTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub scopes: Vec<String>,
}

/// Writes a brand-new login into `claudeAiOauth`, creating that key if it's missing instead of
/// requiring it to already exist the way `merge_claude_refresh` does — a first-ever sign-in has
/// no prior `claudeAiOauth` object to update in place, and may have no file at all yet.
pub fn merge_claude_login(
    original: &str,
    tokens: &ClaudeLoginTokens,
    now_millis: i64,
) -> Result<String, TokenError> {
    let mut value: serde_json::Value = if original.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(original).map_err(|_| TokenError::Malformed)?
    };
    let root = value.as_object_mut().ok_or(TokenError::Malformed)?;
    let oauth = root
        .entry("claudeAiOauth")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or(TokenError::Malformed)?;
    oauth.insert("accessToken".into(), tokens.access_token.clone().into());
    oauth.insert("refreshToken".into(), tokens.refresh_token.clone().into());
    oauth.insert(
        "expiresAt".into(),
        (now_millis + tokens.expires_in.saturating_mul(1_000)).into(),
    );
    oauth.insert("scopes".into(), tokens.scopes.clone().into());
    serde_json::to_string_pretty(&value).map_err(|_| TokenError::Malformed)
}

pub fn persist_claude_login(
    path: &Path,
    tokens: &ClaudeLoginTokens,
    now_millis: i64,
) -> Result<ClaudeOauthCredentials, TokenError> {
    let original = if path.exists() {
        std::fs::read_to_string(path).map_err(|_| TokenError::Unreadable)?
    } else {
        String::new()
    };
    let merged = merge_claude_login(&original, tokens, now_millis)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| TokenError::Unreadable)?;
    }
    atomic_write(path, &merged)?;
    claude_oauth_from_str(&merged)
}

/// Clears only the Claude Code session from the credentials file, leaving unrelated keys such as
/// `mcpOAuth` untouched. A missing file is already logged out, so that's success, not an error.
pub fn logout_claude(path: &Path) -> Result<(), TokenError> {
    if !path.exists() {
        return Ok(());
    }
    let original = std::fs::read_to_string(path).map_err(|_| TokenError::Unreadable)?;
    let mut value: serde_json::Value =
        serde_json::from_str(&original).map_err(|_| TokenError::Malformed)?;
    let root = value.as_object_mut().ok_or(TokenError::Malformed)?;
    root.remove("claudeAiOauth");
    let updated = serde_json::to_string_pretty(&value).map_err(|_| TokenError::Malformed)?;
    atomic_write(path, &updated)
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
    fn reads_refreshable_claude_oauth_credentials() {
        let auth = claude_oauth_from_str(
            r#"{"claudeAiOauth":{"accessToken":"old","refreshToken":"refresh","expiresAt":1000}}"#,
        )
        .unwrap();
        assert_eq!(auth.access_token, "old");
        assert_eq!(auth.refresh_token.as_deref(), Some("refresh"));
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
    fn merge_claude_login_creates_the_oauth_key_in_a_brand_new_file() {
        let merged = merge_claude_login(
            "",
            &ClaudeLoginTokens {
                access_token: "new-access".into(),
                refresh_token: "new-refresh".into(),
                expires_in: 3600,
                scopes: vec!["user:inference".into(), "user:profile".into()],
            },
            1_000,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(value["claudeAiOauth"]["accessToken"], "new-access");
        assert_eq!(value["claudeAiOauth"]["refreshToken"], "new-refresh");
        assert_eq!(value["claudeAiOauth"]["expiresAt"], 3_601_000);
        assert_eq!(value["claudeAiOauth"]["scopes"][0], "user:inference");
    }

    #[test]
    fn merge_claude_login_adds_the_oauth_key_without_disturbing_other_cli_state() {
        let merged = merge_claude_login(
            r#"{"mcpOAuth":{"server":"kept"}}"#,
            &ClaudeLoginTokens {
                access_token: "a".into(),
                refresh_token: "r".into(),
                expires_in: 60,
                scopes: vec!["user:inference".into()],
            },
            0,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(value["mcpOAuth"]["server"], "kept");
        assert_eq!(value["claudeAiOauth"]["accessToken"], "a");
    }

    #[test]
    fn persist_claude_login_creates_the_claude_directory_for_a_first_time_user() {
        // A desktop-app-only user who has never run `claude` has no `~/.claude` directory at
        // all yet; the very first successful sign-in has to create it, not assume it exists.
        let directory = tempdir().unwrap();
        let path = directory.path().join("nested").join(".credentials.json");
        let saved = persist_claude_login(
            &path,
            &ClaudeLoginTokens {
                access_token: "a".into(),
                refresh_token: "r".into(),
                expires_in: 60,
                scopes: vec!["user:inference".into()],
            },
            0,
        )
        .unwrap();
        assert_eq!(saved.access_token, "a");
        assert!(path.exists());
    }

    #[test]
    fn logout_removes_the_claude_session_but_keeps_other_local_state() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(".credentials.json");
        std::fs::write(
            &path,
            r#"{"mcpOAuth":{"server":"kept"},"claudeAiOauth":{"accessToken":"a"}}"#,
        )
        .unwrap();
        logout_claude(&path).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["mcpOAuth"]["server"], "kept");
        assert!(value.get("claudeAiOauth").is_none());
        assert_eq!(
            claude_oauth_from_str(&std::fs::read_to_string(&path).unwrap()),
            Err(TokenError::NotFound)
        );
    }

    #[test]
    fn logging_out_when_never_signed_in_is_not_an_error() {
        let directory = tempdir().unwrap();
        assert!(logout_claude(&directory.path().join("missing.json")).is_ok());
    }

    #[test]
    fn merges_rotated_claude_tokens_without_discarding_other_credentials() {
        let original = r#"{"mcpOAuth":{"server":"kept"},"claudeAiOauth":{"accessToken":"old","refreshToken":"old-refresh","expiresAt":1000}}"#;
        let merged = merge_claude_refresh(
            original,
            &ClaudeTokenRefresh {
                access_token: "new".into(),
                refresh_token: Some("new-refresh".into()),
                expires_in: 3600,
                refresh_token_expires_in: None,
            },
            2_000,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(value["mcpOAuth"]["server"], "kept");
        assert_eq!(value["claudeAiOauth"]["accessToken"], "new");
        assert_eq!(value["claudeAiOauth"]["refreshToken"], "new-refresh");
        assert_eq!(value["claudeAiOauth"]["expiresAt"], 3_602_000);
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
