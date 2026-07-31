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
