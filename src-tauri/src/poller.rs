use crate::model::{SnapshotState, UsageSnapshot};

pub fn age_state(snapshot: &UsageSnapshot, now: i64, poll_interval: i64) -> SnapshotState {
    if snapshot.state == SnapshotState::Error {
        return SnapshotState::Error;
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
/// actionable straight away.
pub fn state_for_failed_refresh(
    last: Option<&UsageSnapshot>,
    consecutive_failures: u32,
    error_state: SnapshotState,
) -> SnapshotState {
    if error_state == SnapshotState::Error || last.is_some() {
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
pub fn retry_delay_seconds(consecutive_failures: u32) -> u64 {
    match consecutive_failures {
        0 | 1 => 60,
        2 => 120,
        _ => 300,
    }
}

pub fn retain_last_good(
    last: Option<&UsageSnapshot>,
    fetched_at: i64,
    state: SnapshotState,
) -> UsageSnapshot {
    let mut snapshot = last.cloned().unwrap_or(UsageSnapshot {
        windows: vec![],
        fetched_at,
        state,
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
                })
                .collect(),
            fetched_at,
            state,
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
        assert_eq!(retry_delay_seconds(0), 60);
        assert_eq!(retry_delay_seconds(1), 60);
        assert_eq!(retry_delay_seconds(2), 120);
        assert_eq!(retry_delay_seconds(3), 300);
        assert_eq!(retry_delay_seconds(99), 300);
        for failures in 0..10 {
            assert!(retry_delay_seconds(failures) >= 60);
            assert!(retry_delay_seconds(failures + 1) >= retry_delay_seconds(failures));
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
}
