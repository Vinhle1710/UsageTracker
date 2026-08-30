#[cfg(test)]
use std::collections::HashMap;
use zeroize::Zeroizing;
pub trait SecretStore {
    fn put(&mut self, name: &str, secret: Zeroizing<String>) -> Result<(), String>;
    fn get(&self, name: &str) -> Result<Option<Zeroizing<String>>, String>;
    fn delete(&mut self, name: &str) -> Result<(), String>;
}
pub fn target_name(kind: &str, id: &str) -> String {
    format!("UsageTracker/anthropic/{kind}/{id}")
}

pub struct FallbackStore<P, F> {
    primary: P,
    fallback: F,
}

impl<P, F> FallbackStore<P, F> {
    pub fn new(primary: P, fallback: F) -> Self {
        Self { primary, fallback }
    }
}

impl<P: SecretStore, F: SecretStore> SecretStore for FallbackStore<P, F> {
    fn put(&mut self, name: &str, secret: Zeroizing<String>) -> Result<(), String> {
        let primary_copy = Zeroizing::new(secret.to_string());
        if self.primary.put(name, primary_copy).is_ok() {
            self.fallback.delete(name).map_err(|error| {
                format!(
                    "primary credential stored, but stale fallback could not be cleared: {error}"
                )
            })?;
            return Ok(());
        }
        self.fallback.put(name, secret)
    }

    fn get(&self, name: &str) -> Result<Option<Zeroizing<String>>, String> {
        match self.primary.get(name) {
            Ok(Some(secret)) => Ok(Some(secret)),
            Ok(None) | Err(_) => self.fallback.get(name),
        }
    }

    fn delete(&mut self, name: &str) -> Result<(), String> {
        let primary = self.primary.delete(name);
        let fallback = self.fallback.delete(name);
        match (primary, fallback) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(primary), Ok(())) => Err(format!(
                "fallback credential deleted, but primary secure storage could not be cleared: {primary}"
            )),
            (Ok(()), Err(fallback)) => Err(format!(
                "primary credential deleted, but fallback secure storage could not be cleared: {fallback}"
            )),
            (Err(primary), Err(fallback)) => Err(format!(
                "secure storage could not be cleared (primary: {primary}; fallback: {fallback})"
            )),
        }
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct MemoryStore {
    pub values: HashMap<String, Zeroizing<String>>,
}
#[cfg(test)]
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
        let n = target_name("claude-ai", "id");
        s.put(&n, Zeroizing::new("fixture-secret".into())).unwrap();
        assert_eq!(&*s.get(&n).unwrap().unwrap(), "fixture-secret");
        s.delete(&n).unwrap();
        assert!(s.get(&n).unwrap().is_none());
    }
    struct FailingStore;
    impl SecretStore for FailingStore {
        fn put(&mut self, _: &str, _: Zeroizing<String>) -> Result<(), String> {
            Err("primary unavailable".into())
        }
        fn get(&self, _: &str) -> Result<Option<Zeroizing<String>>, String> {
            Err("primary unavailable".into())
        }
        fn delete(&mut self, _: &str) -> Result<(), String> {
            Err("primary unavailable".into())
        }
    }

    struct DeleteFailingStore {
        value: Option<Zeroizing<String>>,
    }

    impl SecretStore for DeleteFailingStore {
        fn put(&mut self, _: &str, secret: Zeroizing<String>) -> Result<(), String> {
            self.value = Some(secret);
            Ok(())
        }

        fn get(&self, _: &str) -> Result<Option<Zeroizing<String>>, String> {
            Ok(self
                .value
                .as_ref()
                .map(|value| Zeroizing::new(value.to_string())))
        }

        fn delete(&mut self, _: &str) -> Result<(), String> {
            Err("fallback delete failed".into())
        }
    }

    #[test]
    fn fallback_store_survives_primary_failures() {
        let mut store = FallbackStore::new(FailingStore, MemoryStore::default());
        store
            .put("target", Zeroizing::new("fixture-secret".into()))
            .unwrap();
        assert_eq!(&*store.get("target").unwrap().unwrap(), "fixture-secret");
        assert!(store.delete("target").is_err());
        assert!(store.get("target").unwrap().is_none());
    }

    #[test]
    fn primary_write_reports_failure_when_the_stale_fallback_cannot_be_removed() {
        let fallback = DeleteFailingStore {
            value: Some(Zeroizing::new("old-secret".into())),
        };
        let mut store = FallbackStore::new(MemoryStore::default(), fallback);

        assert!(store
            .put("target", Zeroizing::new("new-secret".into()))
            .is_err());
    }
}
