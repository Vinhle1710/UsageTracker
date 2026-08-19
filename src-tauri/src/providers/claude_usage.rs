//! Claude usage adapter.
//!
//! Claude.ai usage payloads are undocumented. This module deliberately has no guessed source
//! field names: until a redacted fixture is captured, production reports the section as
//! unavailable instead of interpreting arbitrary JSON as usage.
use crate::model::{ClaudeExtra, ClaudeModelLimit, DataSection, DataSectionState};

pub fn merge_data_section<T: Clone>(previous: &DataSection<T>, incoming: DataSection<T>) -> DataSection<T> {
    if incoming.value.is_some() { incoming } else { DataSection { value: previous.value.clone(), fetched_at: incoming.fetched_at, state: incoming.state, error_code: incoming.error_code } }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageParseError {
    ContractChanged,
}

pub fn unavailable_limits(now: i64) -> DataSection<Vec<ClaudeModelLimit>> {
    DataSection {
        value: None,
        fetched_at: now,
        state: DataSectionState::Unavailable,
        error_code: Some("feature-unavailable".into()),
    }
}

pub fn unavailable_extra(now: i64) -> DataSection<ClaudeExtra> {
    DataSection {
        value: None,
        fetched_at: now,
        state: DataSectionState::Unavailable,
        error_code: Some("feature-unavailable".into()),
    }
}

/// No parser is enabled until the exact source keys are present in a verified fixture.
pub fn parse_verified_usage(
    _value: &serde_json::Value,
    _now: i64,
) -> Result<(DataSection<Vec<ClaudeModelLimit>>, DataSection<ClaudeExtra>), UsageParseError> {
    Err(UsageParseError::ContractChanged)
}

/// Header extraction remains intentionally empty until exact provider header names/formats are
/// observed. Raw headers are never returned to callers.
pub fn parse_rate_limit_headers(
    _status: u16,
    _headers: &std::collections::BTreeMap<String, String>,
    now: i64,
) -> DataSection<Vec<ClaudeModelLimit>> {
    unavailable_limits(now)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unverified_payload_is_not_treated_as_a_claude_contract() {
        let error = parse_verified_usage(&serde_json::json!({"limits": []}), 10).unwrap_err();
        assert_eq!(error, UsageParseError::ContractChanged);
    }
    #[test]
    fn unavailable_sections_have_safe_user_facing_error_code() {
        assert_eq!(
            unavailable_limits(1).error_code.as_deref(),
            Some("feature-unavailable")
        );
        assert_eq!(
            unavailable_extra(1).error_code.as_deref(),
            Some("feature-unavailable")
        );
    }
    #[test]
    fn unknown_headers_are_not_exposed() {
        let mut headers = std::collections::BTreeMap::new();
        headers.insert("x-secret-token".into(), "redacted".into());
        assert!(parse_rate_limit_headers(429, &headers, 1).value.is_none());
    }
    #[test]
    fn stale_refresh_keeps_last_good_value() {
        let previous = DataSection { value: Some(vec![1]), fetched_at: 1, state: DataSectionState::Fresh, error_code: None };
        let merged = merge_data_section(&previous, DataSection { value: None, fetched_at: 2, state: DataSectionState::Error, error_code: Some("contract-changed".into()) });
        assert_eq!(merged.value, Some(vec![1])); assert_eq!(merged.state, DataSectionState::Error);
    }
}
