pub fn should_display(active_sources: bool, manually_hidden: bool) -> bool {
    active_sources && !manually_hidden
}

pub fn next_manual_hidden(active_sources: bool, manually_hidden: bool) -> bool {
    active_sources && manually_hidden
}

#[cfg(test)]
mod tests {
    use super::{next_manual_hidden, should_display};

    #[test]
    fn active_sources_display_when_not_manually_hidden() {
        assert!(should_display(true, false));
    }

    #[test]
    fn manual_hide_survives_detection_ticks() {
        assert!(!should_display(true, true));
        assert!(next_manual_hidden(true, true));
    }

    #[test]
    fn closing_all_sources_clears_manual_hide_for_next_launch() {
        assert!(!next_manual_hidden(false, true));
        assert!(!should_display(false, false));
    }
}
