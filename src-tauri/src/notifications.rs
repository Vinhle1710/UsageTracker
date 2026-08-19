pub fn crossings(previous: Option<f32>, current: f32, thresholds: &[u8]) -> Vec<u8> {
    let Some(previous) = previous else {
        return vec![];
    };
    if !current.is_finite() || !previous.is_finite() {
        return vec![];
    };
    thresholds
        .iter()
        .copied()
        .filter(|t| previous < *t as f32 && current >= *t as f32)
        .collect()
}
pub fn deliver(
    app: &tauri::AppHandle,
    ledger_path: &std::path::Path,
    provider: &str,
    window: &str,
    resets_at: i64,
    threshold: u8,
    sound: &str,
    body: &str,
) -> Result<bool, String> {
    use tauri::plugin::PermissionState;
    use tauri_plugin_notification::NotificationExt;
    let permission = app
        .notification()
        .permission_state()
        .map_err(|e| e.to_string())?;
    if permission == PermissionState::Denied {
        return Ok(false);
    }
    if permission != PermissionState::Granted
        && app
            .notification()
            .request_permission()
            .map_err(|e| e.to_string())?
            != PermissionState::Granted
    {
        return Ok(false);
    }
    let mut store = crate::notification_store::NotificationStore::load(ledger_path);
    if store.was_sent(provider, window, resets_at, threshold) {
        return Ok(false);
    }
    app.notification()
        .builder()
        .title("Usage Tracker")
        .body(body)
        .show()
        .map_err(|e| e.to_string())?;
    store.mark_sent(
        provider,
        window,
        resets_at,
        threshold,
        chrono::Utc::now().timestamp(),
    );
    store.save(ledger_path).map_err(|e| e.to_string())?;
    crate::sound::Sound::parse(sound)
        .unwrap_or(crate::sound::Sound::Default)
        .play();
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn emits_each_crossed_threshold_in_order() {
        assert_eq!(crossings(Some(74.0), 96.0, &[75, 90, 95]), vec![75, 90, 95]);
    }
    #[test]
    fn first_observation_does_not_backfill_alerts() {
        assert!(crossings(None, 96.0, &[75, 90, 95]).is_empty());
    }
    #[test]
    fn falling_usage_does_not_alert() {
        assert!(crossings(Some(96.0), 20.0, &[75, 90, 95]).is_empty());
    }
}
