use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

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
    /// Shape of the overlay card's usage readout. Distinct from `indicator_style`, which shapes
    /// the tray icon.
    #[serde(default = "default_meter_shape")]
    pub meter_shape: String,
    #[serde(default)]
    pub auto_initialize_session: bool,
    #[serde(default)]
    pub auto_init_cost_warning_accepted: bool,
    #[serde(default = "default_model_task")]
    pub auto_init_task_kind: String,
    #[serde(default)]
    pub refresh_on_wake: bool,
    #[serde(default = "default_true")]
    pub monitor_network: bool,
    #[serde(default)]
    pub shortcut_popover: Option<String>,
    #[serde(default)]
    pub shortcut_refresh: Option<String>,
    #[serde(default)]
    pub shortcut_settings: Option<String>,
    #[serde(default)]
    pub last_auto_init_at: Option<i64>,
    #[serde(default = "default_history_retention")]
    pub history_retention_days: u16,
}

fn default_meter_shape() -> String {
    "ring".into()
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
fn default_model_task() -> String {
    "light".into()
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
            meter_shape: default_meter_shape(),
            auto_initialize_session: false,
            auto_init_cost_warning_accepted: false,
            auto_init_task_kind: default_model_task(),
            refresh_on_wake: true,
            monitor_network: true,
            shortcut_popover: None,
            shortcut_refresh: None,
            shortcut_settings: None,
            last_auto_init_at: None,
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
        // "blur" joins the retired presets: Windows exposes no per-window backdrop blur behind
        // a transparent webview, so it only ever rendered as a dimmer Frosted.
        if matches!(
            self.theme.as_str(),
            "acrylic" | "opaque" | "custom" | "blur"
        ) {
            self.theme = "frosted".into();
        }
        if !matches!(self.theme.as_str(), "clear" | "frosted" | "solid" | "neon") {
            self.theme = default_theme();
        }
        // Opacity is not a knob on Solid: a translucent "solid" card is just Frosted with
        // extra steps. Enforced here, the one choke point every config write passes through,
        // so no caller can persist the contradiction.
        if self.theme == "solid" {
            self.card_opacity = 1.0;
        }
        if !valid_hex_color(&self.background_color) {
            self.background_color = default_background_color();
        }
        if self.meter_shape == "bar" {
            self.meter_shape = "charge".into();
        }
        if !matches!(
            self.meter_shape.as_str(),
            "ring" | "charge" | "reactor" | "columns" | "line" | "semicircle"
        ) {
            self.meter_shape = default_meter_shape();
        }
        if !matches!(
            self.layout.as_str(),
            "stacked-compact" | "provider-columns" | "minimal"
        ) {
            self.layout = default_layout();
        }
        self.poll_interval_sec = self.poll_interval_sec.clamp(15, 3600);
        self.detect_interval_sec = self.detect_interval_sec.max(1);
        if !matches!(
            self.auto_init_task_kind.as_str(),
            "light" | "standard" | "reasoning"
        ) {
            self.auto_init_task_kind = default_model_task();
        }
        if !self.auto_init_cost_warning_accepted {
            self.auto_initialize_session = false;
        }
        for shortcut in [
            &mut self.shortcut_popover,
            &mut self.shortcut_refresh,
            &mut self.shortcut_settings,
        ] {
            if shortcut.as_ref().is_some_and(|s| s.trim().is_empty()) {
                *shortcut = None;
            } else if let Some(s) = shortcut {
                *s = s.trim().to_string();
            }
        }
        self.history_retention_days = self.history_retention_days.clamp(30, 730);
        self
    }
}

/// Removes obsolete app-owned plaintext credentials from pre-1.0 config files. Claude Code and
/// Codex credential files are outside this path and are never modified.
pub fn remove_legacy_secrets(path: &Path) -> std::io::Result<bool> {
    let original = match std::fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let mut value: serde_json::Value = serde_json::from_str(&original)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let Some(object) = value.as_object_mut() else {
        return Ok(false);
    };
    let mut changed = false;
    for key in ["claudeAccessToken", "claudeRefreshToken", "anthropicApiKey"] {
        changed |= object.remove(key).is_some();
    }
    if !changed {
        return Ok(false);
    }
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("config path has no parent"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(serde_json::to_string_pretty(&value)?.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(true)
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
        assert!(!c.auto_initialize_session);
        assert!(!c.auto_init_cost_warning_accepted);
        assert_eq!(c.poll_interval_sec, 60);
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
    fn startup_cleanup_removes_legacy_plaintext_credentials_and_preserves_config() {
        let d = tempdir().unwrap();
        let p = d.path().join("config.json");
        std::fs::write(
            &p,
            r#"{"corner":"top-left","claudeAccessToken":"fixture-access","claudeRefreshToken":"fixture-refresh","anthropicApiKey":"fixture-api-key","unrelated":"keep"}"#,
        )
        .unwrap();

        assert!(remove_legacy_secrets(&p).unwrap());
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(value["corner"], "top-left");
        assert_eq!(value["unrelated"], "keep");
        for key in ["claudeAccessToken", "claudeRefreshToken", "anthropicApiKey"] {
            assert!(value.get(key).is_none(), "{key} remained in plaintext");
        }
        assert!(!remove_legacy_secrets(&p).unwrap());
    }
    #[test]
    fn blur_theme_migrates_to_frosted() {
        // Windows gives no API for a genuine per-window backdrop blur behind a transparent
        // webview, so the preset only ever looked like a dimmer Frosted. Existing configs
        // carrying it are migrated rather than reset to the default.
        assert_eq!(
            Config {
                theme: "blur".into(),
                ..Default::default()
            }
            .sanitized()
            .theme,
            "frosted"
        );
    }

    #[test]
    fn solid_theme_is_always_fully_opaque() {
        // "Solid" is a claim about the surface, not a starting point for a slider: a
        // translucent solid card is just Frosted with extra steps.
        let sanitized = Config {
            theme: "solid".into(),
            card_opacity: 0.85,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(sanitized.card_opacity, 1.0);
    }

    #[test]
    fn non_solid_themes_keep_their_opacity() {
        let sanitized = Config {
            theme: "frosted".into(),
            card_opacity: 0.85,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(sanitized.card_opacity, 0.85);
    }

    #[test]
    fn unknown_meter_shape_falls_back_to_the_ring() {
        assert_eq!(
            Config {
                meter_shape: "spiral".into(),
                ..Default::default()
            }
            .sanitized()
            .meter_shape,
            "ring"
        );
        for shape in ["ring", "charge", "reactor", "columns", "line", "semicircle"] {
            assert_eq!(
                Config {
                    meter_shape: shape.into(),
                    ..Default::default()
                }
                .sanitized()
                .meter_shape,
                shape
            );
        }
    }

    #[test]
    fn minimal_is_an_accepted_layout_and_unknown_layouts_fall_back() {
        for layout in ["stacked-compact", "provider-columns", "minimal"] {
            assert_eq!(
                Config {
                    layout: layout.into(),
                    ..Default::default()
                }
                .sanitized()
                .layout,
                layout
            );
        }
        assert_eq!(
            Config {
                layout: "tiles".into(),
                ..Default::default()
            }
            .sanitized()
            .layout,
            "stacked-compact"
        );
    }

    #[test]
    fn legacy_bar_meter_shape_migrates_to_charge() {
        assert_eq!(
            Config {
                meter_shape: "bar".into(),
                ..Default::default()
            }
            .sanitized()
            .meter_shape,
            "charge"
        );
    }

    #[test]
    fn neon_is_an_accepted_theme() {
        assert_eq!(
            Config {
                theme: "neon".into(),
                ..Default::default()
            }
            .sanitized()
            .theme,
            "neon"
        );
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
    fn sanitize_enforces_poll_floor() {
        assert_eq!(
            Config {
                poll_interval_sec: 1,
                ..Default::default()
            }
            .sanitized()
            .poll_interval_sec,
            15
        );
    }

    #[test]
    fn automation_is_off_by_default() {
        let c = Config::default();
        assert!(!c.auto_initialize_session);
        assert!(!c.auto_init_cost_warning_accepted);
        assert_eq!(c.poll_interval_sec, 60);
    }

    #[test]
    fn short_polling_is_bounded() {
        assert_eq!(
            Config {
                poll_interval_sec: 2,
                ..Default::default()
            }
            .sanitized()
            .poll_interval_sec,
            15
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
