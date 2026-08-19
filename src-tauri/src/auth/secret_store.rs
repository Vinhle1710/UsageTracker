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
}
