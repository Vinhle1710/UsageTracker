use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationStore {
    pub schema_version: u8,
    pub sent: Vec<SentThreshold>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentThreshold {
    pub provider: String,
    pub window_kind: String,
    pub resets_at: i64,
    pub threshold: u8,
    pub sent_at: i64,
}
impl NotificationStore {
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
        {
            Some(s) => s,
            None => {
                if path.exists() {
                    let _ =
                        std::fs::rename(path, path.with_file_name("notifications.corrupt.json"));
                }
                Self {
                    schema_version: 1,
                    sent: vec![],
                }
            }
        }
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap())?;
        std::fs::rename(tmp, path)
    }
    pub fn was_sent(&self, p: &str, w: &str, r: i64, t: u8) -> bool {
        self.sent
            .iter()
            .any(|x| x.provider == p && x.window_kind == w && x.resets_at == r && x.threshold == t)
    }
    pub fn mark_sent(&mut self, p: &str, w: &str, r: i64, t: u8, at: i64) {
        if !self.was_sent(p, w, r, t) {
            self.sent.push(SentThreshold {
                provider: p.into(),
                window_kind: w.into(),
                resets_at: r,
                threshold: t,
                sent_at: at,
            });
        }
    }
    pub fn reset(&mut self) {
        self.sent.clear();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn round_trip_preserves_sent_crossing() {
        let d = tempdir().unwrap();
        let p = d.path().join("notifications.json");
        let mut s = NotificationStore::load(&p);
        s.mark_sent("claude", "session_5h", 2000, 90, 1000);
        s.save(&p).unwrap();
        assert!(NotificationStore::load(&p).was_sent("claude", "session_5h", 2000, 90));
    }
    #[test]
    fn corrupt_file_is_quarantined_and_starts_empty() {
        let d = tempdir().unwrap();
        let p = d.path().join("notifications.json");
        std::fs::write(&p, "{broken").unwrap();
        let s = NotificationStore::load(&p);
        assert!(s.sent.is_empty());
        assert!(d.path().join("notifications.corrupt.json").exists());
    }
}
