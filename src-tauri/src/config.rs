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
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_background_color")]
    pub background_color: String,
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default)]
    pub offscreen_peek: bool,
    #[serde(default = "default_true")]
    pub launch_at_startup: bool,
    #[serde(default = "default_poll")]
    pub poll_interval_sec: u64,
    #[serde(default = "default_detect")]
    pub detect_interval_sec: u64,
    #[serde(default = "default_true")]
    pub show_tray_indicator: bool,
    #[serde(default = "default_true")]
    pub show_screen_overlay: bool,
    #[serde(default = "default_history_retention")]
    pub history_retention_days: u16,
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
fn default_theme() -> String {
    "frosted".into()
}
fn default_background_color() -> String {
    "#07101f".into()
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
    1
}
fn default_history_retention() -> u16 {
    180
}

impl Default for Config {
    fn default() -> Self {
        Self {
            monitor_id: None,
            corner: default_corner(),
            scale: default_scale(),
            card_opacity: default_card_opacity(),
            theme: default_theme(),
            background_color: default_background_color(),
            layout: default_layout(),
            always_on_top: true,
            offscreen_peek: false,
            launch_at_startup: true,
            poll_interval_sec: 60,
            detect_interval_sec: 1,
            show_tray_indicator: true,
            show_screen_overlay: true,
            history_retention_days: 180,
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
        if !self.show_tray_indicator && !self.show_screen_overlay {
            self.show_tray_indicator = true;
        }
        self.scale = self.scale.clamp(0.75, 1.5);
        self.card_opacity = self.card_opacity.clamp(0.82, 1.0);
        if matches!(self.theme.as_str(), "acrylic" | "opaque" | "custom") {
            self.theme = "frosted".into();
        }
        if !matches!(self.theme.as_str(), "clear" | "frosted" | "blur" | "solid") {
            self.theme = default_theme();
        }
        if !valid_hex_color(&self.background_color) {
            self.background_color = default_background_color();
        }
        if !matches!(self.layout.as_str(), "stacked-compact" | "provider-columns") {
            self.layout = default_layout();
        }
        self.poll_interval_sec = self.poll_interval_sec.max(30);
        self.detect_interval_sec = self.detect_interval_sec.max(1);
        self.history_retention_days = self.history_retention_days.clamp(30, 730);
        self
    }
}

fn valid_hex_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn missing_file_yields_defaults() {
        let d = tempdir().unwrap();
        assert_eq!(Config::load(&d.path().join("none")), Config::default());
        assert_eq!(Config::default().detect_interval_sec, 1);
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
        assert_eq!(c.theme, "frosted");
        assert_eq!(c.background_color, "#07101f");
        assert_eq!(c.layout, "stacked-compact");
        assert!(c.always_on_top);
        assert!(c.launch_at_startup);
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
    fn sanitize_rejects_unknown_theme_and_invalid_background() {
        let sanitized = Config {
            theme: "sunset".into(),
            background_color: "navy".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(sanitized.theme, "frosted");
        assert_eq!(sanitized.background_color, "#07101f");
    }
    #[test]
    fn sanitize_migrates_acrylic_theme_to_frosted() {
        let sanitized = Config {
            theme: "acrylic".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(sanitized.theme, "frosted");
    }
    #[test]
    fn sanitize_migrates_opaque_theme_to_frosted() {
        let sanitized = Config {
            theme: "opaque".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(sanitized.theme, "frosted");
    }
    #[test]
    fn sanitize_migrates_old_custom_theme_to_frosted() {
        let sanitized = Config {
            theme: "custom".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(sanitized.theme, "frosted");
    }
    #[test]
    fn sanitize_accepts_blur_theme() {
        let sanitized = Config {
            theme: "blur".into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(sanitized.theme, "blur");
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

    #[test]
    fn config_never_disables_both_presentation_surfaces() {
        let c = Config {
            show_tray_indicator: false,
            show_screen_overlay: false,
            ..Default::default()
        }
        .sanitized();
        assert!(c.show_tray_indicator);
        assert!(!c.show_screen_overlay);
    }
}
