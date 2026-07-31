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
    fn retains_values_when_a_refresh_fails() {
        let previous = snap(SnapshotState::Fresh, 1000, &[42.0]);
        let retained = retain_last_good(Some(&previous), 1060, SnapshotState::Stale);
        assert_eq!(retained.windows[0].used_percent, 42.0);
        assert_eq!(retained.fetched_at, 1060);
        assert_eq!(retained.state, SnapshotState::Stale);
    }
}
