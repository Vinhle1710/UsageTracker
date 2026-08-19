use super::AccountKind;
use std::collections::HashMap;
use zeroize::Zeroizing;
pub trait SecretStore {
    fn put(&mut self, name: &str, secret: Zeroizing<String>) -> Result<(), String>;
    fn get(&self, name: &str) -> Result<Option<Zeroizing<String>>, String>;
    fn delete(&mut self, name: &str) -> Result<(), String>;
}
pub fn target_name(kind: AccountKind, id: &str) -> String {
    format!(
        "UsageTracker/anthropic/{}/{}",
        match kind {
            AccountKind::ClaudeAi => "claude-ai",
            AccountKind::AnthropicConsole => "anthropic-console",
        },
        id
    )
}

/// Migrates one explicitly app-owned legacy key. The source is only removed after a read-back
/// comparison; Claude Code files are intentionally outside this API.
pub fn migrate_legacy_secret<S: SecretStore>(
    store: &mut S,
    config: &mut serde_json::Value,
    key: &str,
    target: &str,
) -> Result<bool, String> {
    if !matches!(
        key,
        "claudeAccessToken" | "claudeRefreshToken" | "anthropicApiKey"
    ) {
        return Err("legacy key is not app-owned".into());
    }
    let Some(value) = config
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
    else {
        return Ok(false);
    };
    let secret = Zeroizing::new(value.to_owned());
    store.put(target, Zeroizing::new(secret.to_string()))?;
    let Some(saved) = store.get(target)? else {
        return Err("secure write could not be verified".into());
    };
    if *saved != *secret {
        return Err("secure write verification failed".into());
    }
    if let Some(obj) = config.as_object_mut() {
        obj.remove(key);
        obj.insert("secretMigrationVersion".into(), serde_json::json!(1));
    }
    Ok(true)
}
#[derive(Default)]
pub struct MemoryStore {
    pub values: HashMap<String, Zeroizing<String>>,
}
impl SecretStore for MemoryStore {
    fn put(&mut self, n: &str, s: Zeroizing<String>) -> Result<(), String> {
        if n.len() > 256 {
            return Err("name too long".into());
        }
        self.values.insert(n.into(), s);
        Ok(())
    }
    fn get(&self, n: &str) -> Result<Option<Zeroizing<String>>, String> {
        Ok(self.values.get(n).map(|v| Zeroizing::new(v.to_string())))
    }
    fn delete(&mut self, n: &str) -> Result<(), String> {
        self.values.remove(n);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memory_roundtrip() {
        let mut s = MemoryStore::default();
        let n = target_name(AccountKind::ClaudeAi, "id");
        s.put(&n, Zeroizing::new("fixture-secret".into())).unwrap();
        assert_eq!(&*s.get(&n).unwrap().unwrap(), "fixture-secret");
        s.delete(&n).unwrap();
        assert!(s.get(&n).unwrap().is_none());
    }
    #[test]
    fn migration_removes_only_after_verified_write() {
        let mut store = MemoryStore::default();
        let mut cfg = serde_json::json!({"anthropicApiKey":"sk-ant-test","other":"keep"});
        assert!(migrate_legacy_secret(&mut store, &mut cfg, "anthropicApiKey", "target").unwrap());
        assert!(cfg.get("anthropicApiKey").is_none());
        assert_eq!(cfg["other"], "keep");
        assert_eq!(cfg["secretMigrationVersion"], 1);
        assert!(!migrate_legacy_secret(&mut store, &mut cfg, "anthropicApiKey", "target").unwrap());
    }
    #[test]
    fn migration_rejects_unknown_keys_without_touching_config() {
        let mut store = MemoryStore::default();
        let mut cfg = serde_json::json!({"token":"access-secret"});
        assert!(migrate_legacy_secret(&mut store, &mut cfg, "token", "target").is_err());
        assert!(cfg.get("token").is_some());
    }
}
