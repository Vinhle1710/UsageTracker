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
/// Drops the account *and* its stored secret. Removing only the summary leaves the API key
/// live in the OS credential store, so a user who clicked "Remove" still has a working
/// credential on disk — the one thing that action promises to undo.
pub fn remove_account<S: super::secret_store::SecretStore + ?Sized>(
    store: &mut S,
    accounts: &mut Vec<AccountSummary>,
    account_id: &str,
) -> Result<(), String> {
    let Some(index) = accounts.iter().position(|a| a.id == account_id) else {
        return Ok(());
    };
    let account = accounts.remove(index);
    store.delete(&super::secret_store::target_name(account.kind, &account.id))
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

    #[test]
    fn removing_an_account_purges_its_stored_secret() {
        use super::super::secret_store::{target_name, MemoryStore, SecretStore};
        let mut store = MemoryStore::default();
        let summary = manual_summary("console:manual-abc", "sk-ant-test-1234");
        let name = target_name(summary.kind, &summary.id);
        store
            .put(&name, Zeroizing::new("sk-ant-test-1234".into()))
            .unwrap();
        let mut accounts = vec![summary];

        remove_account(&mut store, &mut accounts, "console:manual-abc").unwrap();

        assert!(accounts.is_empty());
        assert!(store.get(&name).unwrap().is_none(), "secret outlived the account");
    }

    #[test]
    fn removing_an_unknown_account_is_a_no_op() {
        use super::super::secret_store::MemoryStore;
        let mut accounts = vec![manual_summary("console:manual-abc", "sk-ant-1234")];
        remove_account(&mut MemoryStore::default(), &mut accounts, "nope").unwrap();
        assert_eq!(accounts.len(), 1);
    }
}
