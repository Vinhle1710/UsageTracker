pub fn should_display(active_sources: bool, manually_hidden: bool) -> bool {
    active_sources && !manually_hidden
}

pub fn next_manual_hidden(active_sources: bool, manually_hidden: bool) -> bool {
    active_sources && manually_hidden
}

pub fn should_show_prefetched_overlay(
    active_sources: bool,
    usage_ready: bool,
    webview_ready: bool,
    manually_hidden: bool,
) -> bool {
    active_sources && usage_ready && webview_ready && !manually_hidden
}

pub fn new_provider_activated(
    previous: crate::detect::ActiveSources,
    current: crate::detect::ActiveSources,
) -> bool {
    (!previous.claude && current.claude) || (!previous.openai && current.openai)
}

pub fn should_reveal_window(
    active_sources: bool,
    usage_ready: bool,
    webview_ready: bool,
    manually_hidden: bool,
    currently_visible: bool,
) -> bool {
    !currently_visible
        && should_show_prefetched_overlay(
            active_sources,
            usage_ready,
            webview_ready,
            manually_hidden,
        )
}

pub fn usage_cycle_is_complete(
    polled_sources: crate::detect::ActiveSources,
    current_sources: crate::detect::ActiveSources,
    events: &[crate::model::ProviderUsageEvent],
) -> bool {
    polled_sources == current_sources
        && (!current_sources.claude
            || events
                .iter()
                .any(|event| event.provider == crate::model::Provider::Claude))
        && (!current_sources.openai
            || events
                .iter()
                .any(|event| event.provider == crate::model::Provider::Openai))
}

#[cfg(test)]
mod tests {
    use super::{
        new_provider_activated, next_manual_hidden, should_display, should_reveal_window,
        should_show_prefetched_overlay, usage_cycle_is_complete,
    };
    use crate::{
        detect::ActiveSources,
        model::{Provider, ProviderUsageEvent, SnapshotState, UsageSnapshot},
    };

    fn event(provider: Provider) -> ProviderUsageEvent {
        ProviderUsageEvent {
            provider,
            snapshot: UsageSnapshot {
                windows: Vec::new(),
                fetched_at: 1,
                state: SnapshotState::Fresh,
            },
        }
    }

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

    #[test]
    fn waits_for_usage_and_the_webview_before_first_show() {
        assert!(!should_show_prefetched_overlay(true, false, true, false));
        assert!(!should_show_prefetched_overlay(true, true, false, false));
        assert!(should_show_prefetched_overlay(true, true, true, false));
        assert!(!should_show_prefetched_overlay(true, true, true, true));
    }

    #[test]
    fn rejects_a_prefetch_from_before_sources_changed() {
        let inactive = ActiveSources::default();
        let active = ActiveSources {
            claude: true,
            openai: true,
        };

        assert!(!usage_cycle_is_complete(inactive, active, &[]));
        assert!(!usage_cycle_is_complete(
            active,
            active,
            &[event(Provider::Claude)]
        ));
        assert!(usage_cycle_is_complete(
            active,
            active,
            &[event(Provider::Claude), event(Provider::Openai)]
        ));
    }

    #[test]
    fn activating_a_second_provider_requests_an_immediate_refresh() {
        let openai_only = ActiveSources {
            claude: false,
            openai: true,
        };
        let both = ActiveSources {
            claude: true,
            openai: true,
        };

        assert!(new_provider_activated(openai_only, both));
        assert!(!new_provider_activated(both, both));
    }

    #[test]
    fn an_already_visible_overlay_is_not_shown_again() {
        assert!(!should_reveal_window(true, true, true, false, true));
        assert!(should_reveal_window(true, true, true, false, false));
    }
}
