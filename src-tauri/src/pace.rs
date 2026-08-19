use serde::Serialize;

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct PaceInput { pub used_percent: f32, pub sampled_at: i64, pub window_started_at: i64, pub resets_at: i64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PaceStatus { Behind, OnPace, Ahead }
#[derive(Debug, Clone, Copy, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pace { pub expected_percent: f32, pub delta_percent: f32, pub status: PaceStatus }

pub fn calculate(i: PaceInput) -> Option<Pace> {
    let duration = i.resets_at.checked_sub(i.window_started_at)?;
    let elapsed = i.sampled_at.checked_sub(i.window_started_at)?;
    if duration <= 0 || elapsed < 0 || elapsed > duration || !i.used_percent.is_finite() { return None; }
    let expected = (elapsed as f32 / duration as f32 * 100.0).clamp(0.0, 100.0);
    let delta = i.used_percent - expected;
    Some(Pace { expected_percent: expected, delta_percent: delta, status: if delta > 5.0 { PaceStatus::Ahead } else if delta < -5.0 { PaceStatus::Behind } else { PaceStatus::OnPace } })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn computes_expected_marker_and_status() {
        let p = calculate(PaceInput { used_percent: 60.0, sampled_at: 4_500, window_started_at: 0, resets_at: 10_000 }).unwrap();
        assert_eq!(p.expected_percent, 45.0); assert_eq!(p.delta_percent, 15.0); assert_eq!(p.status, PaceStatus::Ahead);
    }
    #[test]
    fn rejects_unknown_or_elapsed_windows() {
        assert_eq!(calculate(PaceInput { used_percent: 10.0, sampled_at: 10, window_started_at: 10, resets_at: 0 }), None);
    }
}
