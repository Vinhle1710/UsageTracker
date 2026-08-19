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

pub trait Registrar {
    fn register(&mut self, shortcut: &str) -> Result<(), String>;
    fn unregister(&mut self, shortcut: &str) -> Result<(), String>;
}

pub fn transactional_replace<R: Registrar>(
    registrar: &mut R,
    old: &ShortcutConfig,
    new: &ShortcutConfig,
) -> Result<(), String> {
    validate(new).map_err(|e| format!("shortcut conflict: {e:?}"))?;
    let staged: Vec<(String, ShortcutSlot)> = configured(new)
        .filter(|(v, slot)| {
            !configured(old).any(|(o, old_slot)| o.eq_ignore_ascii_case(v) && old_slot == *slot)
        })
        .map(|(v, slot)| (v.to_string(), slot))
        .collect();
    let mut removed: Vec<(String, ShortcutSlot)> = Vec::new();
    let mut added = Vec::new();
    for (value, _) in staged {
        if let Err(error) = registrar.register(&value) {
            let mut rollback: Vec<_> = added
                .iter()
                .filter_map(|v: &String| registrar.unregister(v).err())
                .collect();
            rollback.extend(
                removed
                    .iter()
                    .filter_map(|(v, _)| registrar.register(v).err()),
            );
            return Err(if rollback.is_empty() {
                format!("registration failed for {value}: {error}")
            } else {
                format!(
                    "registration failed for {value}: {error}; rollback failed: {}",
                    rollback.join("; ")
                )
            });
        }
        added.push(value);
    }
    for (value, slot) in configured(old) {
        if !configured(new).any(|(v, new_slot)| v.eq_ignore_ascii_case(value) && new_slot == slot) {
            if let Err(error) = registrar.unregister(value) {
                let mut rollback: Vec<_> = added
                    .iter()
                    .filter_map(|v: &String| registrar.unregister(v).err())
                    .collect();
                rollback.extend(
                    removed
                        .iter()
                        .filter_map(|(v, _)| registrar.register(v).err()),
                );
                return Err(format!(
                    "unregister failed for {value}: {error}; rollback: {}",
                    rollback.join("; ")
                ));
            }
            removed.push((value.to_string(), slot));
        }
    }
    Ok(())
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
                        app.state::<crate::AppState>()
                            .manual_refresh_requested
                            .store(true, std::sync::atomic::Ordering::Release);
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
            let rollback: Vec<_> = added
                .into_iter()
                .filter_map(|previous| {
                    app.global_shortcut()
                        .unregister(previous.as_str())
                        .err()
                        .map(|e| e.to_string())
                })
                .collect();
            return Err(if rollback.is_empty() {
                format!("shortcut registration failed for {value}: {error}")
            } else {
                format!(
                    "shortcut registration failed for {value}: {error}; rollback failed: {}",
                    rollback.join("; ")
                )
            });
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
    let mut registrar = TauriRegistrar { app, desired: new };
    transactional_replace(&mut registrar, old, new)
}

struct TauriRegistrar<'a> {
    app: &'a tauri::AppHandle,
    desired: &'a ShortcutConfig,
}
impl Registrar for TauriRegistrar<'_> {
    fn register(&mut self, shortcut: &str) -> Result<(), String> {
        let mut one = ShortcutConfig {
            popover: None,
            refresh: None,
            settings: None,
        };
        for (value, slot) in configured(self.desired) {
            if value.eq_ignore_ascii_case(shortcut) {
                match slot {
                    ShortcutSlot::Popover => one.popover = Some(value.into()),
                    ShortcutSlot::Refresh => one.refresh = Some(value.into()),
                    ShortcutSlot::Settings => one.settings = Some(value.into()),
                }
            }
        }
        register_all(self.app, &one)
    }
    fn unregister(&mut self, shortcut: &str) -> Result<(), String> {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        self.app
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        ops: Vec<String>,
        fail_register: bool,
        fail_unregister: bool,
    }
    impl Registrar for Fake {
        fn register(&mut self, s: &str) -> Result<(), String> {
            self.ops.push(format!("+{s}"));
            if self.fail_register {
                Err("os register".into())
            } else {
                Ok(())
            }
        }
        fn unregister(&mut self, s: &str) -> Result<(), String> {
            self.ops.push(format!("-{s}"));
            if self.fail_unregister {
                Err("os unregister".into())
            } else {
                Ok(())
            }
        }
    }
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
    #[test]
    fn overlap_keeps_unchanged_binding() {
        let old = ShortcutConfig {
            popover: Some("A".into()),
            refresh: Some("B".into()),
            settings: None,
        };
        let new = ShortcutConfig {
            popover: Some("A".into()),
            refresh: Some("C".into()),
            settings: None,
        };
        let mut f = Fake {
            ops: vec![],
            fail_register: false,
            fail_unregister: false,
        };
        transactional_replace(&mut f, &old, &new).unwrap();
        assert_eq!(f.ops, vec!["+C", "-B"]);
    }
    #[test]
    fn registration_failure_reports_rollback() {
        let old = ShortcutConfig {
            popover: Some("A".into()),
            refresh: None,
            settings: None,
        };
        let new = ShortcutConfig {
            popover: Some("B".into()),
            refresh: None,
            settings: None,
        };
        let mut f = Fake {
            ops: vec![],
            fail_register: true,
            fail_unregister: false,
        };
        assert!(transactional_replace(&mut f, &old, &new).is_err());
    }
    #[test]
    fn unregister_failure_is_an_error() {
        let old = ShortcutConfig {
            popover: Some("A".into()),
            refresh: None,
            settings: None,
        };
        let new = ShortcutConfig {
            popover: Some("B".into()),
            refresh: None,
            settings: None,
        };
        let mut f = Fake {
            ops: vec![],
            fail_register: false,
            fail_unregister: true,
        };
        assert!(transactional_replace(&mut f, &old, &new).is_err());
    }
}
