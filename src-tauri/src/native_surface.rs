use crate::material::NativeWindowState;
use std::sync::Mutex;
use tauri::Emitter;

pub const MAX_REPAIR_RETRIES: u8 = 1;

pub fn enqueue_non_blocking<F, S>(schedule: S, operation: F) -> Result<(), String>
where
    F: FnOnce() + Send + 'static,
    S: FnOnce(F) -> Result<(), String>,
{
    schedule(operation).map_err(|error| format!("native surface scheduling failed: {error}"))
}

pub fn repair_regions(label: &str, cached: &NativeWindowState) -> Vec<crate::material::CardRegion> {
    if label == "main" {
        cached.regions.clone()
    } else {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingRepair {
    pub label: &'static str,
    pub force_region: bool,
    pub retry_count: u8,
}

impl PendingRepair {
    fn new(label: &'static str, force_region: bool, retry_count: u8) -> Self {
        Self {
            label,
            force_region,
            retry_count,
        }
    }
}

#[derive(Debug, Default)]
pub struct PendingRepairController {
    main: Option<PendingRepair>,
    settings: Option<PendingRepair>,
    main_in_flight: Option<PendingRepair>,
    settings_in_flight: Option<PendingRepair>,
}

impl PendingRepairController {
    fn slot_mut(&mut self, label: &str) -> Option<&mut Option<PendingRepair>> {
        match label {
            "main" => Some(&mut self.main),
            "settings" => Some(&mut self.settings),
            _ => None,
        }
    }

    fn slot(&self, label: &str) -> Option<&Option<PendingRepair>> {
        match label {
            "main" => Some(&self.main),
            "settings" => Some(&self.settings),
            _ => None,
        }
    }

    fn in_flight_slot_mut(&mut self, label: &str) -> Option<&mut Option<PendingRepair>> {
        match label {
            "main" => Some(&mut self.main_in_flight),
            "settings" => Some(&mut self.settings_in_flight),
            _ => None,
        }
    }

    pub fn request(&mut self, label: &'static str, force_region: bool) -> bool {
        let Some(slot) = self.slot_mut(label) else {
            return false;
        };
        if let Some(pending) = slot {
            pending.force_region |= force_region;
            false
        } else {
            *slot = Some(PendingRepair::new(label, force_region, 0));
            true
        }
    }

    pub fn request_retry(&mut self, failed: PendingRepair) -> bool {
        if failed.retry_count >= MAX_REPAIR_RETRIES {
            return false;
        }
        let Some(slot) = self.slot_mut(failed.label) else {
            return false;
        };
        if let Some(pending) = slot {
            pending.force_region |= failed.force_region;
            pending.retry_count = pending.retry_count.max(failed.retry_count + 1);
            false
        } else {
            *slot = Some(PendingRepair::new(
                failed.label,
                failed.force_region,
                failed.retry_count + 1,
            ));
            true
        }
    }

    pub fn take(&mut self, label: &str) -> Option<PendingRepair> {
        let pending = self.slot_mut(label)?.take()?;
        *self.in_flight_slot_mut(label)? = Some(pending);
        Some(pending)
    }

    pub fn clear(&mut self, label: &str) {
        if let Some(slot) = self.slot_mut(label) {
            *slot = None;
        }
        if let Some(slot) = self.in_flight_slot_mut(label) {
            *slot = None;
        }
    }

    pub fn complete(&mut self, label: &str) {
        if let Some(slot) = self.in_flight_slot_mut(label) {
            *slot = None;
        }
    }

    pub fn pending(&self, label: &str) -> Option<PendingRepair> {
        self.slot(label).and_then(|slot| *slot)
    }
}

pub struct NativeSurfaceState {
    // These locks are held only while copying or replacing state. Native/Tauri calls happen
    // after the lock is released, and all callers that touch the cache do so on the main thread.
    pub cache: Mutex<NativeWindowState>,
    pub pending_repairs: Mutex<PendingRepairController>,
}

impl Default for NativeSurfaceState {
    fn default() -> Self {
        Self {
            cache: Mutex::new(NativeWindowState::default()),
            pending_repairs: Mutex::new(PendingRepairController::default()),
        }
    }
}

pub fn report_diagnostic(app: &tauri::AppHandle, operation: &'static str, _error: &str) {
    let _ = app.emit(
        "native-surface-diagnostic",
        serde_json::json!({
            "operation": operation,
            "status": "failed",
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_repairs_are_coalesced_independently_per_label() {
        let mut controller = PendingRepairController::default();

        assert!(controller.request("main", false));
        assert!(controller.request("settings", false));
        assert!(!controller.request("main", false));

        assert_eq!(controller.take("main").unwrap().label, "main");
        assert_eq!(controller.take("settings").unwrap().label, "settings");
    }

    #[test]
    fn coalesced_force_region_is_or_only_for_the_same_label() {
        let mut controller = PendingRepairController::default();

        assert!(controller.request("main", false));
        assert!(!controller.request("main", true));
        assert!(controller.take("main").unwrap().force_region);

        assert!(controller.request("settings", false));
        assert!(!controller.take("settings").unwrap().force_region);
    }

    #[test]
    fn schedule_failure_and_completion_clear_pending_repairs() {
        let mut controller = PendingRepairController::default();

        assert!(controller.request("main", false));
        controller.clear("main");
        assert_eq!(controller.take("main"), None);

        assert!(controller.request("settings", false));
        let _ = controller.take("settings");
        controller.complete("settings");
        assert_eq!(controller.take("settings"), None);
    }

    #[test]
    fn completion_does_not_clear_a_new_request_submitted_while_running() {
        let mut controller = PendingRepairController::default();

        assert!(controller.request("main", false));
        let running = controller.take("main").unwrap();
        assert!(controller.request("main", true));
        controller.complete(running.label);

        assert!(controller.take("main").unwrap().force_region);
    }

    #[test]
    fn native_failure_allows_exactly_one_bounded_retry() {
        let mut controller = PendingRepairController::default();

        assert!(controller.request("main", true));
        let failed = controller.take("main").unwrap();
        assert!(controller.request_retry(failed));
        let retried = controller.take("main").unwrap();
        assert_eq!(retried.retry_count, 1);
        assert!(retried.force_region);
        assert!(!controller.request_retry(retried));
    }

    #[test]
    fn background_work_is_only_submitted_and_never_waited_for() {
        let mut submitted = false;
        let result = enqueue_non_blocking(
            |_operation| {
                submitted = true;
                Ok(())
            },
            || panic!("background caller must not run the main-thread closure"),
        );

        assert!(result.is_ok());
        assert!(submitted);
    }

    #[test]
    fn repair_reads_the_latest_cached_regions_for_main() {
        let cached = NativeWindowState {
            regions: vec![crate::material::CardRegion {
                x: 8,
                y: 8,
                width: 310,
                height: 168,
                radius: 14,
            }],
            ..NativeWindowState::default()
        };

        assert_eq!(repair_regions("main", &cached), cached.regions);
        assert!(repair_regions("settings", &cached).is_empty());
    }
}
