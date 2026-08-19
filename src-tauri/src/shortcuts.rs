#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutConfig {
    pub popover: Option<String>,
    pub refresh: Option<String>,
    pub settings: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutError {
    Duplicate(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutSlot {
    Popover,
    Refresh,
    Settings,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    TogglePopover,
    Refresh,
    OpenSettings,
}
pub fn validate(c: &ShortcutConfig) -> Result<(), ShortcutError> {
    let mut seen = std::collections::HashSet::new();
    for v in [&c.popover, &c.refresh, &c.settings].into_iter().flatten() {
        if !seen.insert(v.to_ascii_lowercase()) {
            return Err(ShortcutError::Duplicate(v.clone()));
        }
    }
    Ok(())
}
pub fn action_for(s: ShortcutSlot) -> ShortcutAction {
    match s {
        ShortcutSlot::Popover => ShortcutAction::TogglePopover,
        ShortcutSlot::Refresh => ShortcutAction::Refresh,
        ShortcutSlot::Settings => ShortcutAction::OpenSettings,
    }
}

pub fn configured(c: &ShortcutConfig) -> impl Iterator<Item = (&str, ShortcutSlot)> {
    [
        (c.popover.as_deref(), ShortcutSlot::Popover),
        (c.refresh.as_deref(), ShortcutSlot::Refresh),
        (c.settings.as_deref(), ShortcutSlot::Settings),
    ]
    .into_iter()
    .filter_map(|(value, slot)| value.map(|value| (value, slot)))
}
pub fn from_config(c: &crate::config::Config) -> ShortcutConfig {
    ShortcutConfig {
        popover: c.shortcut_popover.clone(),
        refresh: c.shortcut_refresh.clone(),
        settings: c.shortcut_settings.clone(),
    }
}

pub fn register_all(app: &tauri::AppHandle, c: &ShortcutConfig) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    validate(c).map_err(|e| format!("shortcut conflict: {e:?}"))?;
    let mut added: Vec<String> = Vec::new();
    for (value, slot) in configured(c) {
        let result = app
            .global_shortcut()
            .on_shortcut(value, move |app, _shortcut, event| {
                use tauri::{Emitter, Manager};
                use tauri_plugin_global_shortcut::ShortcutState;
                if event.state != ShortcutState::Pressed {
                    return;
                }
                match action_for(slot) {
                    ShortcutAction::TogglePopover => {
                        let _ = app.emit("shortcut-toggle-popover", ());
                    }
                    ShortcutAction::Refresh => {
                        app.state::<crate::AppState>().usage_wake.notify_one();
                        let _ =
                            app.emit("runtime-status-changed", crate::runtime_status(app.clone()));
                    }
                    ShortcutAction::OpenSettings => {
                        let _ = app.emit("shortcut-open-settings", ());
                    }
                }
            });
        if let Err(error) = result {
            for previous in added {
                let _ = app.global_shortcut().unregister(previous.as_str());
            }
            return Err(format!("shortcut registration failed for {value}: {error}"));
        }
        added.push(value.to_string());
    }
    Ok(())
}

pub fn replace(
    app: &tauri::AppHandle,
    old: &ShortcutConfig,
    new: &ShortcutConfig,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    validate(new).map_err(|e| format!("shortcut conflict: {e:?}"))?;
    // Keep unchanged bindings registered and stage only additions/replacements. This leaves the
    // exact old set intact if the OS rejects any new registration.
    let staged = ShortcutConfig {
        popover: (new.popover != old.popover)
            .then(|| new.popover.clone())
            .flatten(),
        refresh: (new.refresh != old.refresh)
            .then(|| new.refresh.clone())
            .flatten(),
        settings: (new.settings != old.settings)
            .then(|| new.settings.clone())
            .flatten(),
    };
    register_all(app, &staged)?;
    for (value, _) in configured(old) {
        let still_registered =
            configured(new).any(|(candidate, _)| candidate.eq_ignore_ascii_case(value));
        if !still_registered
            || configured(&staged).any(|(candidate, _)| candidate.eq_ignore_ascii_case(value))
        {
            let _ = app.global_shortcut().unregister(value);
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicates_before_registration() {
        let s = ShortcutConfig {
            popover: Some("Ctrl+Shift+U".into()),
            refresh: Some("Ctrl+Shift+U".into()),
            settings: None,
        };
        assert_eq!(
            validate(&s),
            Err(ShortcutError::Duplicate("Ctrl+Shift+U".into()))
        );
    }
    #[test]
    fn maps_actions_exactly() {
        assert_eq!(
            action_for(ShortcutSlot::Popover),
            ShortcutAction::TogglePopover
        );
        assert_eq!(action_for(ShortcutSlot::Refresh), ShortcutAction::Refresh);
    }
}
