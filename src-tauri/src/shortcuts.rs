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
