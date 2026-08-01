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

pub fn persist_claude_refresh(
    path: &Path,
    refresh: &ClaudeTokenRefresh,
    now_millis: i64,
) -> Result<ClaudeOauthCredentials, TokenError> {
    let original = std::fs::read_to_string(path).map_err(|_| TokenError::Unreadable)?;
    let merged = merge_claude_refresh(&original, refresh, now_millis)?;
    let parent: PathBuf = path.parent().ok_or(TokenError::Unreadable)?.into();
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| TokenError::Unreadable)?;
    temporary
        .write_all(merged.as_bytes())
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| TokenError::Unreadable)?;
    temporary
        .persist(path)
        .map_err(|_| TokenError::Unreadable)?;
    claude_oauth_from_str(&merged)
}

pub fn codex_token_from_str(s: &str) -> Result<String, TokenError> {
    let value: serde_json::Value = serde_json::from_str(s).map_err(|_| TokenError::Malformed)?;
    value
        .pointer("/tokens/access_token")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .ok_or(TokenError::Malformed)
}

pub fn read_token(
    path: &Path,
    parse: fn(&str) -> Result<String, TokenError>,
) -> Result<String, TokenError> {
    if !path.exists() {
        return Err(TokenError::NotFound);
    }
    parse(&std::fs::read_to_string(path).map_err(|_| TokenError::Unreadable)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn extracts_codex_token() {
        assert_eq!(
            codex_token_from_str(r#"{"tokens":{"access_token":"abc123"}}"#).unwrap(),
            "abc123"
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
    fn rejects_empty_token() {
        assert_eq!(
            codex_token_from_str(r#"{"tokens":{"access_token":""}}"#),
            Err(TokenError::Malformed)
        );
    }
    #[test]
    fn rejects_malformed_json() {
        assert_eq!(codex_token_from_str("{oops"), Err(TokenError::Malformed));
    }
    #[test]
    fn missing_file_reports_not_found() {
        let d = tempdir().unwrap();
        assert_eq!(
            read_token(&d.path().join("none"), codex_token_from_str),
            Err(TokenError::NotFound)
        );
    }
    #[test]
    fn error_debug_contains_no_token() {
        let err = format!("{:?}", TokenError::Malformed);
        assert!(!err.contains("SUPERSECRET"));
    }
}
