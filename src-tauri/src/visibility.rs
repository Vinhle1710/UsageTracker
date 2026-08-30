/// Which of the two overlay surfaces should be on screen.
///
/// The overlay has three resting states, not two: the full cards, the provider bubbles, and —
/// added here — a single tab against the screen edge with no bubbles at all. Deciding it in one
/// pure function keeps "exactly one of the two windows is visible" a property of the type
/// rather than something every call site has to remember to uphold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySurface {
    /// The cards/bubbles window.
    Overlay,
    /// The edge tab only; the main window is hidden.
    EdgeTab,
    /// Neither — no provider is running, or the user hid the overlay outright.
    None,
}

pub fn overlay_surface(
    active_sources: bool,
    webview_ready: bool,
    manually_hidden: bool,
    tucked: bool,
) -> OverlaySurface {
    if !active_sources || !webview_ready || manually_hidden {
        return OverlaySurface::None;
    }
    if tucked {
        OverlaySurface::EdgeTab
    } else {
        OverlaySurface::Overlay
    }
}

pub fn should_display(active_sources: bool, manually_hidden: bool) -> bool {
    active_sources && !manually_hidden
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationPhase {
    Stable,
    Hiding,
    Revealing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayVisibilityState {
    pub enabled: bool,
    pub provider_available: bool,
    pub user_hidden: bool,
    pub generation: u64,
    pub phase: AnimationPhase,
    pub stable_position: (i32, i32),
}

impl OverlayVisibilityState {
    pub fn request_hidden(&mut self, hidden: bool) {
        self.generation = self.generation.wrapping_add(1);
        self.user_hidden = hidden;
        self.phase = if hidden {
            AnimationPhase::Hiding
        } else {
            AnimationPhase::Revealing
        };
    }
    pub fn settle(&mut self, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        self.phase = AnimationPhase::Stable;
        true
    }
}

pub fn next_manual_hidden(active_sources: bool, manually_hidden: bool) -> bool {
    active_sources && manually_hidden
}

pub fn new_provider_activated(
    previous: crate::detect::ActiveSources,
    current: crate::detect::ActiveSources,
) -> bool {
    (!previous.claude && current.claude) || (!previous.openai && current.openai)
}

/// The frontend's own `get_bootstrap` read races the synchronous startup scan that populates
/// `state.sources`: the webview can start executing (and call `get_bootstrap`) before that write
/// lands, getting a stale default `{false,false}` snapshot. Because the detection loop only
/// emits `sources-changed` on an actual transition, and the *backend's* state was already
/// correct from the first scan, a lost race was never corrected — the frontend stayed wrong
/// forever with no further event to fix it. Forcing one emission on the loop's first tick,
/// regardless of whether anything changed, gives every frontend a corrective update within
/// ~1 second of startup even when it lost that race.
pub fn should_emit_sources_changed(
    previous: crate::detect::ActiveSources,
    current: crate::detect::ActiveSources,
    is_first_tick: bool,
) -> bool {
    is_first_tick || previous != current
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowTransition {
    Show,
    Hide,
    Unchanged,
}

/// tao rewrites `GWL_STYLE` wholesale from its own flag set whenever a window flag changes, and
/// its flag set always contains `WS_CAPTION` (it hides the frame via `WM_NCCALCSIZE` instead).
/// Any transition that toggles visibility therefore undoes `enforce_borderless` and needs it re-run.
pub fn requires_borderless_reenforcement(transition: WindowTransition) -> bool {
    matches!(transition, WindowTransition::Show | WindowTransition::Hide)
}

#[derive(Default)]
pub struct VisibilityTransitionController {
    currently_visible: bool,
}

impl VisibilityTransitionController {
    pub fn new(currently_visible: bool) -> Self {
        Self { currently_visible }
    }

    pub fn next(
        &mut self,
        active_sources: bool,
        webview_ready: bool,
        manually_hidden: bool,
    ) -> WindowTransition {
        let transition = next_window_transition(
            active_sources,
            webview_ready,
            manually_hidden,
            self.currently_visible,
        );
        self.apply(transition);
        transition
    }

    fn apply(&mut self, transition: WindowTransition) {
        match transition {
            WindowTransition::Show => self.currently_visible = true,
            WindowTransition::Hide => self.currently_visible = false,
            WindowTransition::Unchanged => {}
        }
    }

    pub fn currently_visible(&self) -> bool {
        self.currently_visible
    }
}

pub fn next_window_transition(
    active_sources: bool,
    webview_ready: bool,
    manually_hidden: bool,
    currently_visible: bool,
) -> WindowTransition {
    let should_be_visible = active_sources && webview_ready && !manually_hidden;
    match (should_be_visible, currently_visible) {
        (true, false) => WindowTransition::Show,
        (false, true) => WindowTransition::Hide,
        _ => WindowTransition::Unchanged,
    }
}

pub fn should_reveal_window(
    active_sources: bool,
    webview_ready: bool,
    manually_hidden: bool,
    currently_visible: bool,
) -> bool {
    next_window_transition(
        active_sources,
        webview_ready,
        manually_hidden,
        currently_visible,
    ) == WindowTransition::Show
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
mod borderless_tests {
    use super::*;

    #[test]
    fn showing_or_hiding_the_overlay_requires_reenforcing_the_borderless_frame() {
        // Showing is what makes the caption reappear, so repairing only beforehand is too early.
        assert!(requires_borderless_reenforcement(WindowTransition::Show));
        assert!(requires_borderless_reenforcement(WindowTransition::Hide));
    }

    #[test]
    fn an_unchanged_transition_leaves_the_frame_alone() {
        assert!(!requires_borderless_reenforcement(
            WindowTransition::Unchanged
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        new_provider_activated, next_manual_hidden, next_window_transition, should_display,
        should_emit_sources_changed, should_reveal_window, usage_cycle_is_complete,
        VisibilityTransitionController, WindowTransition,
    };

    #[test]
    fn visibility_generation_ignores_stale_animation_settlement() {
        let mut state = super::OverlayVisibilityState {
            enabled: true,
            provider_available: true,
            user_hidden: false,
            generation: 0,
            phase: super::AnimationPhase::Stable,
            stable_position: (10, 20),
        };
        state.request_hidden(true);
        let first = state.generation;
        state.request_hidden(false);
        assert!(!state.settle(first));
        assert!(state.settle(state.generation));
        assert_eq!(state.phase, super::AnimationPhase::Stable);
    }
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
                details: None,
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
    fn reveal_requires_an_active_ready_unhidden_webview() {
        assert!(should_reveal_window(true, true, false, false));
        assert!(!should_reveal_window(true, false, false, false));
        assert!(!should_reveal_window(true, true, true, false));
    }

    #[test]
    fn an_active_provider_reveals_as_soon_as_the_webview_is_ready() {
        assert!(should_reveal_window(true, true, false, false));
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
    fn activating_the_first_provider_requests_an_immediate_poll_wake() {
        let inactive = ActiveSources::default();
        let claude_only = ActiveSources {
            claude: true,
            openai: false,
        };

        assert!(new_provider_activated(inactive, claude_only));
    }

    #[test]
    fn the_first_tick_always_emits_even_when_nothing_changed_to_correct_a_lost_bootstrap_race() {
        let both = ActiveSources {
            claude: true,
            openai: true,
        };

        assert!(should_emit_sources_changed(both, both, true));
        assert!(!should_emit_sources_changed(both, both, false));
    }

    #[test]
    fn later_ticks_still_emit_on_a_genuine_transition() {
        let inactive = ActiveSources::default();
        let active = ActiveSources {
            claude: true,
            openai: false,
        };

        assert!(should_emit_sources_changed(inactive, active, false));
        assert!(!should_emit_sources_changed(active, active, false));
    }

    #[test]
    fn provider_deactivation_supersedes_a_stale_show_decision() {
        assert_eq!(
            next_window_transition(true, true, false, false),
            WindowTransition::Show
        );
        assert_eq!(
            next_window_transition(false, true, false, true),
            WindowTransition::Hide
        );
    }

    #[test]
    fn manual_hide_supersedes_a_stale_show_decision() {
        assert_eq!(
            next_window_transition(true, true, false, false),
            WindowTransition::Show
        );
        assert_eq!(
            next_window_transition(true, true, true, true),
            WindowTransition::Hide
        );
    }

    #[test]
    fn an_already_visible_overlay_is_not_shown_again() {
        assert!(!should_reveal_window(true, true, false, true));
        assert!(should_reveal_window(true, true, false, false));
    }

    #[test]
    fn serialized_show_followed_by_hide_converges_to_hidden() {
        let mut controller = VisibilityTransitionController::new(false);

        assert_eq!(controller.next(true, true, false), WindowTransition::Show);
        assert_eq!(controller.next(false, true, false), WindowTransition::Hide);

        assert!(!controller.currently_visible());
    }
}

#[cfg(test)]
mod edge_tab_tests {
    use super::*;

    #[test]
    fn tucking_away_swaps_the_cards_for_the_edge_tab() {
        assert_eq!(
            overlay_surface(true, true, false, false),
            OverlaySurface::Overlay
        );
        assert_eq!(
            overlay_surface(true, true, false, true),
            OverlaySurface::EdgeTab
        );
    }

    #[test]
    fn a_hidden_overlay_shows_no_tab_either() {
        // Hiding is "get off my screen", which a tab still on the edge would disobey.
        assert_eq!(
            overlay_surface(true, true, true, true),
            OverlaySurface::None
        );
    }

    #[test]
    fn no_running_provider_means_no_surface_at_all_even_when_tucked() {
        assert_eq!(
            overlay_surface(false, true, false, true),
            OverlaySurface::None
        );
    }

    #[test]
    fn the_tab_waits_for_the_webview_the_same_way_the_cards_do() {
        assert_eq!(
            overlay_surface(true, false, false, true),
            OverlaySurface::None
        );
    }
}
