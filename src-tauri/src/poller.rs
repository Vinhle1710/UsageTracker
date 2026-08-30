use crate::model::{SnapshotState, UsageSnapshot};

pub fn age_state(snapshot: &UsageSnapshot, now: i64, poll_interval: i64) -> SnapshotState {
    // Neither verdict decays with time: no amount of waiting turns a rejected token or an absent
    // credential file into "temporarily unavailable".
    if matches!(
        snapshot.state,
        SnapshotState::Error | SnapshotState::SignedOut
    ) {
        return snapshot.state;
    }
    if now - snapshot.fetched_at > poll_interval * 3 {
        SnapshotState::Stale
    } else {
        snapshot.state
    }
}
pub fn worst_percent(snapshots: &[&UsageSnapshot]) -> Option<f32> {
    snapshots
        .iter()
        .flat_map(|s| s.windows.iter().map(|w| w.used_percent))
        .reduce(f32::max)
}

/// Consecutive failures tolerated before a provider with no data yet stops reading as "checking".
pub const PENDING_FAILURE_GRACE: u32 = 2;

/// A transient refresh failure should not be dressed up as a verdict about usage. With numbers
/// already on screen they are kept and dimmed; with nothing fetched yet the card stays pending
/// until the failure has persisted. Auth failures skip the grace period because they are
/// actionable straight away, as does a missing credential file.
pub fn state_for_failed_refresh(
    last: Option<&UsageSnapshot>,
    consecutive_failures: u32,
    error_state: SnapshotState,
) -> SnapshotState {
    if matches!(error_state, SnapshotState::Error | SnapshotState::SignedOut) || last.is_some() {
        return error_state;
    }
    if consecutive_failures <= PENDING_FAILURE_GRACE {
        SnapshotState::Pending
    } else {
        error_state
    }
}

/// Back off as failures repeat. The usage endpoints rate-limit the *usage call itself* (Claude
/// answers 429 under polling), so retrying harder after a failure makes the failure worse. The
/// first retry keeps the normal one-minute cadence and only sustained failure slows down.
pub fn retry_delay_seconds(consecutive_failures: u32, configured: u64) -> u64 {
    match consecutive_failures {
        0 => configured,
        1 => configured.max(60),
        2 => configured.max(120),
        _ => configured.max(300),
    }
}

pub fn retain_last_good(
    last: Option<&UsageSnapshot>,
    fetched_at: i64,
    state: SnapshotState,
) -> UsageSnapshot {
    // Unlike Error/Stale — which may well be transient, so the last-known numbers are still
    // meaningful while dimmed — SignedOut is a definitive verdict (see `age_state`, which never
    // lets it decay either). There is no active session left for a retained percent to describe,
    // so showing one alongside a "Sign in" hint reads as a broken UI, not a comforting last-known
    // value.
    if state == SnapshotState::SignedOut {
        return UsageSnapshot {
            windows: vec![],
            fetched_at,
            state,
            details: None,
        };
    }
    let mut snapshot = last.cloned().unwrap_or(UsageSnapshot {
        windows: vec![],
        fetched_at,
        state,
        details: None,
    });
    snapshot.fetched_at = fetched_at;
    snapshot.state = state;
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::UsageWindow;
    fn snap(state: SnapshotState, fetched_at: i64, percentages: &[f32]) -> UsageSnapshot {
        UsageSnapshot {
            windows: percentages
                .iter()
                .map(|p| UsageWindow {
                    label: "w".into(),
                    used_percent: *p,
                    resets_at: 0,
                    pace: None,
                })
                .collect(),
            fetched_at,
            state,
            details: None,
        }
    }
    #[test]
    fn recent_snapshot_stays_fresh() {
        assert_eq!(
            age_state(&snap(SnapshotState::Fresh, 1000, &[10.0]), 1030, 60),
            SnapshotState::Fresh
        );
    }
    #[test]
    fn old_snapshot_becomes_stale() {
        assert_eq!(
            age_state(&snap(SnapshotState::Fresh, 1000, &[10.0]), 1500, 60),
            SnapshotState::Stale
        );
    }
    #[test]
    fn error_state_is_not_downgraded() {
        assert_eq!(
            age_state(&snap(SnapshotState::Error, 1000, &[]), 1500, 60),
            SnapshotState::Error
        );
    }
    #[test]
    fn signed_out_is_not_downgraded_by_age() {
        // Time passing does not make a missing credential file "temporarily unavailable".
        assert_eq!(
            age_state(&snap(SnapshotState::SignedOut, 1000, &[]), 1500, 60),
            SnapshotState::SignedOut
        );
    }

    #[test]
    fn signed_out_is_surfaced_immediately_without_a_pending_grace_period() {
        // Like an auth failure, this is actionable on the first cycle: there is nothing to wait
        // for, so pretending to still be "checking usage" only delays the fix.
        assert_eq!(
            state_for_failed_refresh(None, 1, SnapshotState::SignedOut),
            SnapshotState::SignedOut
        );
    }

    #[test]
    fn worst_percent_spans_providers() {
        let a = snap(SnapshotState::Fresh, 0, &[10.0, 20.0]);
        let b = snap(SnapshotState::Fresh, 0, &[55.0]);
        assert_eq!(worst_percent(&[&a, &b]), Some(55.0));
    }
    #[test]
    fn empty_windows_have_no_worst_percent() {
        assert_eq!(worst_percent(&[&snap(SnapshotState::Fresh, 0, &[])]), None);
    }

    #[test]
    fn a_first_failure_with_no_data_yet_reads_as_pending_not_unavailable() {
        assert_eq!(
            state_for_failed_refresh(None, 1, SnapshotState::Stale),
            SnapshotState::Pending
        );
        assert_eq!(
            state_for_failed_refresh(None, PENDING_FAILURE_GRACE, SnapshotState::Stale),
            SnapshotState::Pending
        );
    }

    #[test]
    fn a_sustained_failure_with_no_data_yet_finally_reports_unavailable() {
        assert_eq!(
            state_for_failed_refresh(None, PENDING_FAILURE_GRACE + 1, SnapshotState::Stale),
            SnapshotState::Stale
        );
    }

    #[test]
    fn previously_fetched_numbers_are_kept_and_dimmed_rather_than_replaced() {
        let previous = snap(SnapshotState::Fresh, 1000, &[42.0]);
        assert_eq!(
            state_for_failed_refresh(Some(&previous), 1, SnapshotState::Stale),
            SnapshotState::Stale
        );
    }

    #[test]
    fn an_actionable_auth_failure_is_surfaced_immediately_without_a_grace_period() {
        assert_eq!(
            state_for_failed_refresh(None, 1, SnapshotState::Error),
            SnapshotState::Error
        );
    }

    #[test]
    fn a_repeatedly_failing_provider_is_polled_less_often_not_more() {
        // The usage endpoints throttle the usage call itself, so a failure must never shorten
        // the interval; sustained failure has to widen it.
        assert_eq!(retry_delay_seconds(0, 15), 15);
        assert_eq!(retry_delay_seconds(1, 15), 60);
        assert_eq!(retry_delay_seconds(2, 15), 120);
        assert_eq!(retry_delay_seconds(3, 15), 300);
        assert_eq!(retry_delay_seconds(99, 15), 300);
        for failures in 1..10 {
            assert!(retry_delay_seconds(failures, 15) >= 60);
            assert!(retry_delay_seconds(failures + 1, 15) >= retry_delay_seconds(failures, 15));
        }
    }

    #[test]
    fn retains_values_when_a_refresh_fails() {
        let previous = snap(SnapshotState::Fresh, 1000, &[42.0]);
        let retained = retain_last_good(Some(&previous), 1060, SnapshotState::Stale);
        assert_eq!(retained.windows[0].used_percent, 42.0);
        assert_eq!(retained.fetched_at, 1060);
        assert_eq!(retained.state, SnapshotState::Stale);
    }

    #[test]
    fn signing_out_clears_retained_windows_instead_of_showing_a_stale_percent() {
        // A signed-out card should read as cleanly "not signed in", not as a stale percent
        // sitting alongside a "Sign in" hint.
        let previous = snap(SnapshotState::Fresh, 1000, &[42.0]);
        let retained = retain_last_good(Some(&previous), 1060, SnapshotState::SignedOut);
        assert!(retained.windows.is_empty());
        assert_eq!(retained.fetched_at, 1060);
        assert_eq!(retained.state, SnapshotState::SignedOut);
    }
}
