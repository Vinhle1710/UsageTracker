pub mod console;
pub mod discovery;
pub mod oauth;
pub mod secret_store;
pub mod windows;

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccountKind {
    ClaudeAi,
    AnthropicConsole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CredentialSource {
    ClaudeCode,
    SecureStore,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccountStatus {
    SignedIn,
    NeedsReauthentication,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub id: String,
    pub kind: AccountKind,
    pub source: CredentialSource,
    pub email: Option<String>,
    pub status: AccountStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_hint: Option<String>,
}

impl AccountSummary {
    pub fn signed_in(
        id: impl Into<String>,
        kind: AccountKind,
        source: CredentialSource,
        email: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            source,
            email,
            status: AccountStatus::SignedIn,
            credential_hint: None,
        }
    }
}

#[derive(Debug)]
pub struct SecretRecord {
    pub account_id: String,
    pub kind: AccountKind,
    pub secret: Zeroizing<String>,
    pub expires_at: Option<i64>,
}

pub fn resolve_claude_ai_account(
    selected: Option<&AccountSummary>,
    discovered: Option<&AccountSummary>,
    app_owned: Option<&AccountSummary>,
) -> Option<AccountSummary> {
    selected
        .filter(|a| a.kind == AccountKind::ClaudeAi)
        .cloned()
        .or_else(|| {
            discovered
                .filter(|a| a.kind == AccountKind::ClaudeAi)
                .cloned()
        })
        .or_else(|| {
            app_owned
                .filter(|a| a.kind == AccountKind::ClaudeAi)
                .cloned()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn account_summary_never_serializes_secrets() {
        let value = serde_json::to_value(AccountSummary::signed_in(
            "claude-code:local",
            AccountKind::ClaudeAi,
            CredentialSource::ClaudeCode,
            Some("person@example.com".into()),
        ))
        .unwrap();
        assert_eq!(value["kind"], "claude-ai");
        assert!(value.get("accessToken").is_none());
        assert!(value.get("secret").is_none());
    }
    #[test]
    fn resolver_prioritizes_selected_then_discovery_and_rejects_console() {
        let console = AccountSummary::signed_in(
            "console",
            AccountKind::AnthropicConsole,
            CredentialSource::Manual,
            None,
        );
        let discovered = AccountSummary::signed_in(
            "discovered",
            AccountKind::ClaudeAi,
            CredentialSource::ClaudeCode,
            None,
        );
        assert_eq!(
            resolve_claude_ai_account(Some(&console), Some(&discovered), None)
                .unwrap()
                .id,
            "discovered"
        );
    }
}
