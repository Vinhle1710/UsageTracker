use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub popover_detached: bool,
    #[serde(default)]
    pub popover_position: Option<[i32; 2]>,
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
    #[serde(default = "default_value_mode")]
    pub value_mode: String,
    #[serde(default = "default_indicator_style")]
    pub indicator_style: String,
    /// Shape of the overlay card's usage readout. Distinct from `indicator_style`, which shapes
    /// the tray icon.
    #[serde(default = "default_meter_shape")]
    pub meter_shape: String,
    #[serde(default = "default_metrics")]
    pub enabled_metrics: Vec<String>,
    #[serde(default = "default_metrics")]
    pub metric_order: Vec<String>,
    #[serde(default = "default_color_mode")]
    pub color_mode: String,
    #[serde(default = "default_display_colors")]
    pub display_colors: DisplayColors,
    #[serde(default = "default_true")]
    pub adapt_to_system_theme: bool,
    #[serde(default)]
    pub glow_enabled: bool,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(default = "default_thresholds")]
    pub notification_thresholds: Vec<u8>,
    #[serde(default = "default_notification_sound")]
    pub notification_sound: String,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayColors {
    pub session: String,
    pub weekly: String,
    pub api: String,
    pub single: String,
    pub background: String,
    pub text: String,
}
fn default_value_mode() -> String {
    "used".into()
}
fn default_locale() -> String {
    "en".into()
}
fn default_meter_shape() -> String {
    "ring".into()
}
fn default_indicator_style() -> String {
    "compact".into()
}
fn default_metrics() -> Vec<String> {
    vec!["session".into(), "weekly".into(), "api".into()]
}
fn default_color_mode() -> String {
    "multicolor".into()
}
fn default_display_colors() -> DisplayColors {
    DisplayColors {
        session: "#22c55e".into(),
        weekly: "#f59e0b".into(),
        api: "#60a5fa".into(),
        single: "#60a5fa".into(),
        background: "#07101f".into(),
        text: "#f9fafb".into(),
    }
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
fn default_thresholds() -> Vec<u8> {
    vec![75, 90, 95]
}
fn default_notification_sound() -> String {
    "Default".into()
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
            locale: default_locale(),
            popover_detached: false,
            popover_position: None,
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
            value_mode: default_value_mode(),
            indicator_style: default_indicator_style(),
            meter_shape: default_meter_shape(),
            enabled_metrics: default_metrics(),
            metric_order: default_metrics(),
            color_mode: default_color_mode(),
            display_colors: default_display_colors(),
            adapt_to_system_theme: true,
            glow_enabled: false,
            notifications_enabled: true,
            notification_thresholds: default_thresholds(),
            notification_sound: default_notification_sound(),
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
        if !matches!(
            self.locale.as_str(),
            "en" | "vi"
                | "es"
                | "fr"
                | "de"
                | "it"
                | "pt"
                | "pt-BR"
                | "ja"
                | "ko"
                | "zh-CN"
                | "zh-TW"
                | "tr"
                | "uk"
        ) {
            self.locale = default_locale();
        }
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
        if !matches!(self.value_mode.as_str(), "used" | "remaining") {
            self.value_mode = default_value_mode();
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
            self.indicator_style.as_str(),
            "battery" | "horizontal-progress" | "percentage" | "provider-icon-bar" | "compact"
        ) {
            self.indicator_style = default_indicator_style();
        }
        if !matches!(
            self.color_mode.as_str(),
            "multicolor" | "greyscale" | "single-color"
        ) {
            self.color_mode = default_color_mode();
        }
        self.enabled_metrics
            .retain(|m| matches!(m.as_str(), "session" | "weekly" | "api"));
        if self.enabled_metrics.is_empty() {
            self.enabled_metrics = default_metrics();
        }
        self.metric_order
            .retain(|m| matches!(m.as_str(), "session" | "weekly" | "api"));
        self.metric_order.dedup();
        for m in default_metrics() {
            if !self.metric_order.contains(&m) {
                self.metric_order.push(m);
            }
        }
        if !valid_hex_color(&self.display_colors.session) {
            self.display_colors.session = default_display_colors().session;
        }
        if !valid_hex_color(&self.display_colors.weekly) {
            self.display_colors.weekly = default_display_colors().weekly;
        }
        if !valid_hex_color(&self.display_colors.api) {
            self.display_colors.api = default_display_colors().api;
        }
        if !valid_hex_color(&self.display_colors.single) {
            self.display_colors.single = default_display_colors().single;
        }
        if !valid_hex_color(&self.display_colors.background) {
            self.display_colors.background = default_display_colors().background;
        }
        if !valid_hex_color(&self.display_colors.text) {
            self.display_colors.text = default_display_colors().text;
        }
        if !matches!(self.layout.as_str(), "stacked-compact" | "provider-columns") {
            self.layout = default_layout();
        }
        self.poll_interval_sec = self.poll_interval_sec.clamp(15, 3600);
        self.detect_interval_sec = self.detect_interval_sec.max(1);
        self.notification_thresholds
            .retain(|v| (1..=100).contains(v));
        self.notification_thresholds.sort_unstable();
        self.notification_thresholds.dedup();
        if self.notification_thresholds.is_empty() {
            self.notification_thresholds = default_thresholds();
        }
        if !matches!(
            self.notification_sound.as_str(),
            "Default" | "None" | "Asterisk" | "Exclamation" | "Hand"
        ) {
            self.notification_sound = default_notification_sound();
        }
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
    #[test]
    fn notification_defaults_are_safe() {
        let c = Config::default();
        assert_eq!(c.notification_thresholds, vec![75, 90, 95]);
        assert_eq!(c.notification_sound, "Default");
    }
    #[test]
    fn thresholds_are_sorted_deduped_and_bounded() {
        let c = Config {
            notification_thresholds: vec![95, 0, 75, 75, 101],
            ..Default::default()
        }
        .sanitized();
        assert_eq!(c.notification_thresholds, vec![75, 95]);
    }
}
