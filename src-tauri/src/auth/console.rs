use super::{AccountKind, AccountSummary, CredentialSource};
use zeroize::Zeroizing;

pub const MAX_CREDENTIAL_LEN: usize = 16 * 1024;
pub fn validate_manual_credential(input: &str) -> Result<Zeroizing<String>, &'static str> {
    let value = input.trim();
    if value.is_empty() {
        return Err("credential is required");
    }
    if value.len() > MAX_CREDENTIAL_LEN {
        return Err("credential is too long");
    }
    if value
        .chars()
        .any(|c| c == '\r' || c == '\n' || c.is_control())
    {
        return Err("credential contains invalid characters");
    }
    Ok(Zeroizing::new(value.to_owned()))
}
pub fn manual_summary(account_id: impl Into<String>, credential: &str) -> AccountSummary {
    let hint = credential
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    AccountSummary {
        id: account_id.into(),
        kind: AccountKind::AnthropicConsole,
        source: CredentialSource::Manual,
        email: None,
        status: super::AccountStatus::Unavailable,
        credential_hint: Some(hint),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_and_hints() {
        let v = validate_manual_credential(" sk-ant-test-1234 ").unwrap();
        assert_eq!(&*v, "sk-ant-test-1234");
        assert_eq!(
            manual_summary("x", &v).credential_hint.as_deref(),
            Some("1234")
        );
    }
    #[test]
    fn rejects_controls_and_empty() {
        assert!(validate_manual_credential("\n").is_err());
        assert!(validate_manual_credential(&"x".repeat(MAX_CREDENTIAL_LEN + 1)).is_err());
    }
}
