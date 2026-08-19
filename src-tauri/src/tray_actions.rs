#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn right_click_menu_has_required_order() {
        assert_eq!(menu_action_ids(), ["refresh", "settings", "quit"]);
    }
    #[test]
    fn repeated_refreshes_coalesce() {
        let gate = RefreshGate::default();
        assert!(gate.request());
        assert!(!gate.request());
        gate.complete();
        assert!(gate.request());
    }
}
pub fn menu_action_ids() -> [&'static str; 3] {
    ["refresh", "settings", "quit"]
}
use std::sync::atomic::{AtomicBool, Ordering};
#[derive(Default)]
pub struct RefreshGate(AtomicBool);
impl RefreshGate {
    pub fn request(&self) -> bool {
        !self.0.swap(true, Ordering::AcqRel)
    }
    pub fn complete(&self) {
        self.0.store(false, Ordering::Release);
    }
}
