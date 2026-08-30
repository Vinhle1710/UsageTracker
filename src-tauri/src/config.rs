use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub monitor_id: Option<String>,
    #[serde(default = "default_corner")]
    pub corner: String,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default = "default_card_opacity")]
    pub card_opacity: f32,
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default)]
    pub offscreen_peek: bool,
    #[serde(default = "default_poll")]
    pub poll_interval_sec: u64,
    #[serde(default = "default_detect")]
    pub detect_interval_sec: u64,
}

fn default_corner() -> String {
    "bottom-right".into()
}
fn default_scale() -> f32 {
    1.0
}
fn default_card_opacity() -> f32 {
    0.98
}
fn default_layout() -> String {
    "stacked-compact".into()
}
fn default_true() -> bool {
    true
}
fn default_poll() -> u64 {
    60
}
fn default_detect() -> u64 {
    5
}

impl Default for Config {
    fn default() -> Self {
        Self {
            monitor_id: None,
            corner: default_corner(),
            scale: default_scale(),
            card_opacity: default_card_opacity(),
            layout: default_layout(),
            always_on_top: true,
            offscreen_peek: false,
            poll_interval_sec: 60,
            detect_interval_sec: 5,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap())
    }
    pub fn sanitized(mut self) -> Self {
        self.scale = self.scale.clamp(0.75, 1.5);
        self.card_opacity = self.card_opacity.clamp(0.82, 1.0);
        if !matches!(self.layout.as_str(), "stacked-compact" | "provider-columns") {
            self.layout = default_layout();
        }
        self.poll_interval_sec = self.poll_interval_sec.max(30);
        self.detect_interval_sec = self.detect_interval_sec.max(1);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn missing_file_yields_defaults() {
        let d = tempdir().unwrap();
        assert_eq!(Config::load(&d.path().join("none")), Config::default());
    }
    #[test]
    fn invalid_json_yields_defaults() {
        let d = tempdir().unwrap();
        let p = d.path().join("c.json");
        std::fs::write(&p, "{ bad").unwrap();
        assert_eq!(Config::load(&p), Config::default());
    }
    #[test]
    fn partial_config_fills_missing_fields() {
        let d = tempdir().unwrap();
        let p = d.path().join("c.json");
        std::fs::write(&p, r#"{"corner":"top-left"}"#).unwrap();
        let c = Config::load(&p);
        assert_eq!(c.corner, "top-left");
        assert_eq!(c.scale, 1.0);
        assert_eq!(c.card_opacity, 0.98);
        assert_eq!(c.layout, "stacked-compact");
        assert!(c.always_on_top);
    }
    #[test]
    fn round_trips_through_disk() {
        let d = tempdir().unwrap();
        let p = d.path().join("c.json");
        let c = Config {
            corner: "top-right".into(),
            ..Default::default()
        };
        c.save(&p).unwrap();
        assert_eq!(Config::load(&p), c);
    }
    #[test]
    fn sanitize_clamps_scale() {
        assert_eq!(
            Config {
                scale: 9.0,
                ..Default::default()
            }
            .sanitized()
            .scale,
            1.5
        );
    }
    #[test]
    fn sanitize_clamps_card_opacity() {
        assert_eq!(
            Config {
                card_opacity: 2.0,
                ..Default::default()
            }
            .sanitized()
            .card_opacity,
            1.0
        );
    }
    #[test]
    fn sanitize_enforces_poll_floor() {
        assert_eq!(
            Config {
                poll_interval_sec: 1,
                ..Default::default()
            }
            .sanitized()
            .poll_interval_sec,
            30
        );
    }
}
