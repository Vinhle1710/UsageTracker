use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum TokenError {
    NotFound,
    Unreadable,
    Malformed,
}

pub fn claude_token_from_str(s: &str) -> Result<String, TokenError> {
    let value: serde_json::Value = serde_json::from_str(s).map_err(|_| TokenError::Malformed)?;
    for pointer in [
        "/claudeAiOauth/accessToken",
        "/accessToken",
        "/tokens/access_token",
    ] {
        if let Some(token) = value
            .pointer(pointer)
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
        {
            return Ok(token.to_string());
        }
    }
    Err(TokenError::Malformed)
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
