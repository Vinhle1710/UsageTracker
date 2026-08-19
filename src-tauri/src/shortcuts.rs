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
    fn register(&mut self, shortcut: &str, slot: ShortcutSlot) -> Result<(), String>;
    fn unregister(&mut self, shortcut: &str) -> Result<(), String>;
}

fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn same_binding(
    left: &str,
    left_slot: ShortcutSlot,
    right: &str,
    right_slot: ShortcutSlot,
) -> bool {
    normalized(left) == normalized(right) && left_slot == right_slot
}

pub fn transactional_replace<R: Registrar>(
    registrar: &mut R,
    old: &ShortcutConfig,
    new: &ShortcutConfig,
) -> Result<(), String> {
    validate(new).map_err(|e| format!("shortcut conflict: {e:?}"))?;
    let old_bindings: Vec<_> = configured(old)
        .map(|(value, slot)| (value.to_string(), slot))
        .collect();
    let new_bindings: Vec<_> = configured(new)
        .map(|(value, slot)| (value.to_string(), slot))
        .collect();
    let removed: Vec<_> = old_bindings
        .iter()
        .filter(|(value, slot)| {
            !new_bindings
                .iter()
                .any(|(new_value, new_slot)| same_binding(value, *slot, new_value, *new_slot))
        })
        .cloned()
        .collect();
    let staged: Vec<_> = new_bindings
        .iter()
        .filter(|(value, slot)| {
            !old_bindings
                .iter()
                .any(|(old_value, old_slot)| same_binding(value, *slot, old_value, *old_slot))
        })
        .cloned()
        .collect();

    let restore = |registrar: &mut R, removed: &[(String, ShortcutSlot)]| {
        removed
            .iter()
            .filter_map(|(value, slot)| registrar.register(value, *slot).err())
            .collect::<Vec<_>>()
    };
    let mut removed_done = Vec::new();
    for (value, slot) in &removed {
        if let Err(error) = registrar.unregister(value) {
            let rollback = restore(registrar, &removed_done);
            return Err(if rollback.is_empty() {
                format!("unregister failed for {value}: {error}")
            } else {
                format!("inconsistent shortcut state: unregister failed for {value}: {error}; restoration failed: {}", rollback.join("; "))
            });
        }
        removed_done.push((value.clone(), *slot));
    }
    let mut added = Vec::new();
    for (value, slot) in &staged {
        if let Err(error) = registrar.register(value, *slot) {
            let mut rollback = added
                .iter()
                .filter_map(|(v, _): &(String, ShortcutSlot)| registrar.unregister(v).err())
                .collect::<Vec<_>>();
            rollback.extend(restore(registrar, &removed));
            return Err(if rollback.is_empty() {
                format!("registration failed for {value}: {error}")
            } else {
                format!("inconsistent shortcut state: registration failed for {value}: {error}; restoration failed: {}", rollback.join("; "))
            });
        }
        added.push((value.clone(), *slot));
    }
    Ok(())
}

pub fn register_all(app: &tauri::AppHandle, c: &ShortcutConfig) -> Result<(), String> {
    let empty = ShortcutConfig {
        popover: None,
        refresh: None,
        settings: None,
    };
    replace(app, &empty, c)
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
    fn register(&mut self, shortcut: &str, slot: ShortcutSlot) -> Result<(), String> {
        let mut one = ShortcutConfig {
            popover: None,
            refresh: None,
            settings: None,
        };
        let value = configured(self.desired)
            .find(|(value, desired_slot)| {
                normalized(value) == normalized(shortcut) && *desired_slot == slot
            })
            .map(|(value, _)| value)
            .ok_or_else(|| format!("no desired action for shortcut {shortcut}"))?;
        match slot {
            ShortcutSlot::Popover => one.popover = Some(value.into()),
            ShortcutSlot::Refresh => one.refresh = Some(value.into()),
            ShortcutSlot::Settings => one.settings = Some(value.into()),
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
        active: Vec<String>,
        fail_register: bool,
        fail_unregister: bool,
        fail_unregister_at: Option<usize>,
    }
    impl Registrar for Fake {
        fn register(&mut self, s: &str, _slot: ShortcutSlot) -> Result<(), String> {
            self.ops.push(format!("+{s}"));
            if self.fail_register || self.active.iter().any(|v| v.eq_ignore_ascii_case(s)) {
                Err("os register".into())
            } else {
                self.active.push(s.to_string());
                Ok(())
            }
        }
        fn unregister(&mut self, s: &str) -> Result<(), String> {
            self.ops.push(format!("-{s}"));
            let attempt = self.ops.iter().filter(|op| op.starts_with('-')).count();
            if self.fail_unregister || self.fail_unregister_at == Some(attempt) {
                Err("os unregister".into())
            } else {
                self.active.retain(|v| !v.eq_ignore_ascii_case(s));
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
            active: vec![],
            fail_register: false,
            fail_unregister: false,
            fail_unregister_at: None,
        };
        transactional_replace(&mut f, &old, &new).unwrap();
        assert_eq!(f.ops, vec!["-B", "+C"]);
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
            active: vec!["A".into()],
            fail_register: true,
            fail_unregister: false,
            fail_unregister_at: None,
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
            active: vec!["A".into()],
            fail_register: false,
            fail_unregister: true,
            fail_unregister_at: None,
        };
        assert!(transactional_replace(&mut f, &old, &new).is_err());
    }

    #[test]
    fn remap_same_accelerator_unregisters_old_before_registering_new_action() {
        let old = ShortcutConfig {
            popover: Some("A".into()),
            refresh: None,
            settings: None,
        };
        let new = ShortcutConfig {
            popover: None,
            refresh: Some("A".into()),
            settings: None,
        };
        let mut f = Fake {
            ops: vec![],
            active: vec!["A".into()],
            fail_register: false,
            fail_unregister: false,
            fail_unregister_at: None,
        };
        transactional_replace(&mut f, &old, &new).unwrap();
        assert_eq!(f.ops, vec!["-A", "+A"]);
    }

    #[test]
    fn swap_accelerators_removes_both_old_bindings_before_registering_replacements() {
        let old = ShortcutConfig {
            popover: Some("A".into()),
            refresh: Some("B".into()),
            settings: None,
        };
        let new = ShortcutConfig {
            popover: Some("B".into()),
            refresh: Some("A".into()),
            settings: None,
        };
        let mut f = Fake {
            ops: vec![],
            active: vec!["A".into(), "B".into()],
            fail_register: false,
            fail_unregister: false,
            fail_unregister_at: None,
        };
        transactional_replace(&mut f, &old, &new).unwrap();
        assert_eq!(f.ops, vec!["-A", "-B", "+B", "+A"]);
    }

    #[test]
    fn partial_removal_failure_rolls_back_exact_old_bindings_and_removes_new_bindings() {
        let old = ShortcutConfig {
            popover: Some("A".into()),
            refresh: Some("B".into()),
            settings: None,
        };
        let new = ShortcutConfig {
            popover: Some("C".into()),
            refresh: Some("D".into()),
            settings: None,
        };
        let mut f = Fake {
            ops: vec![],
            active: vec!["A".into(), "B".into()],
            fail_register: false,
            fail_unregister: false,
            fail_unregister_at: Some(2),
        };
        assert!(transactional_replace(&mut f, &old, &new).is_err());
        let mut active = f.active.clone();
        active.sort();
        assert_eq!(active, vec!["A", "B"]);
        assert_eq!(f.ops, vec!["-A", "-B", "+A"]);
    }

    #[test]
    fn restoration_failure_reports_inconsistency_explicitly() {
        let old = ShortcutConfig {
            popover: Some("A".into()),
            refresh: Some("B".into()),
            settings: None,
        };
        let new = ShortcutConfig {
            popover: Some("C".into()),
            refresh: Some("D".into()),
            settings: None,
        };
        let mut f = Fake {
            ops: vec![],
            active: vec!["A".into(), "B".into()],
            fail_register: true,
            fail_unregister: false,
            fail_unregister_at: Some(2),
        };
        let error = transactional_replace(&mut f, &old, &new).unwrap_err();
        assert!(error.to_ascii_lowercase().contains("inconsisten"));
    }
}
