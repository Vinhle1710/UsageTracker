use crate::material::NativeWindowState;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender, TrySendError},
        Mutex, OnceLock,
    },
    thread,
};
use tauri::{Emitter, Manager};

pub const MAX_REPAIR_RETRIES: u8 = 1;
const DIAGNOSTIC_LOG_FILE: &str = "native-surface.jsonl";
const DIAGNOSTIC_ROTATED_LOG_FILE: &str = "native-surface.1.jsonl";
const MAX_DIAGNOSTIC_LOG_BYTES: usize = 64 * 1024;
const MAX_DIAGNOSTIC_RECORD_BYTES: usize = 256;
const DIAGNOSTIC_QUEUE_CAPACITY: usize = 32;

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
    diagnostic_queue: OnceLock<DiagnosticQueue>,
}

impl Default for NativeSurfaceState {
    fn default() -> Self {
        Self {
            cache: Mutex::new(NativeWindowState::default()),
            pending_repairs: Mutex::new(PendingRepairController::default()),
            diagnostic_queue: OnceLock::new(),
        }
    }
}

impl NativeSurfaceState {
    pub fn initialize_diagnostic_writer(&self, log_directory: PathBuf) {
        self.diagnostic_queue.get_or_init(|| {
            let (sender, receiver) = sync_channel(DIAGNOSTIC_QUEUE_CAPACITY);
            let _ = thread::Builder::new()
                .name("native-diagnostic-writer".to_string())
                .spawn(move || run_diagnostic_writer(receiver, log_directory));
            DiagnosticQueue::new(sender)
        });
    }

    fn try_enqueue_diagnostic(&self, record: DiagnosticRecord) -> DiagnosticEnqueueOutcome {
        self.diagnostic_queue
            .get()
            .map(|queue| queue.try_enqueue(record))
            .unwrap_or(DiagnosticEnqueueOutcome::Dropped)
    }
}

fn sanitize_operation(operation: &str) -> String {
    match operation {
        "deferred-repair"
        | "deferred-repair-schedule"
        | "focus-repair"
        | "focus-repair-schedule"
        | "overlay-restore"
        | "settings-close-repair"
        | "settings-focus"
        | "settings-hide"
        | "settings-repair"
        | "settings-repair-schedule"
        | "settings-show"
        | "startup-repair"
        | "startup-repair-schedule"
        | "usage-fetch-claude"
        | "usage-fetch-codex"
        | "visibility-toggle"
        | "visibility-transition" => operation.to_string(),
        _ => "unknown-operation".to_string(),
    }
}

fn native_error_code(error: &str) -> String {
    if let Some(value) = error
        .strip_prefix("http status ")
        .filter(|value| value.len() == 3 && value.chars().all(|c| c.is_ascii_digit()))
    {
        return format!("http-{value}");
    }
    if let Some(value) = error
        .rsplit_once("(os error ")
        .and_then(|(_, suffix)| suffix.split_once(')'))
        .map(|(value, _)| value)
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 11
                && value
                    .chars()
                    .all(|character| character.is_ascii_digit() || character == '-')
        })
    {
        return format!("os-error-{value}");
    }
    if let Some(value) = error
        .split_once("HRESULT ")
        .map(|(_, suffix)| {
            suffix
                .chars()
                .take_while(|character| character.is_ascii_hexdigit() || *character == 'x')
                .take(18)
                .collect::<String>()
        })
        .filter(|value| !value.is_empty())
    {
        return format!("hresult-{}", value.to_ascii_lowercase());
    }
    "unavailable".to_string()
}

fn sanitize_timestamp(timestamp: &str) -> String {
    if !timestamp.is_empty()
        && timestamp.len() <= 32
        && timestamp.chars().all(|character| {
            character.is_ascii_digit() || matches!(character, 'T' | 'Z' | '+' | '-' | ':' | '.')
        })
    {
        timestamp.to_string()
    } else {
        "unavailable".to_string()
    }
}

fn format_diagnostic_line(timestamp: &str, operation: &str, error: &str) -> String {
    let record = serde_json::json!({
        "timestamp": sanitize_timestamp(timestamp),
        "operation": sanitize_operation(operation),
        "nativeCode": native_error_code(error),
        "message": "native operation failed",
    });
    format!("{record}\n")
}

#[derive(Debug, Clone, Copy)]
struct DiagnosticRecord {
    bytes: [u8; MAX_DIAGNOSTIC_RECORD_BYTES],
    len: u16,
}

impl DiagnosticRecord {
    fn new(timestamp: &str, operation: &str, error: &str) -> Self {
        let line = format_diagnostic_line(timestamp, operation, error);
        let line = if line.len() <= MAX_DIAGNOSTIC_RECORD_BYTES {
            line
        } else {
            format_diagnostic_line("unavailable", "unknown-operation", "")
        };
        let mut bytes = [0; MAX_DIAGNOSTIC_RECORD_BYTES];
        bytes[..line.len()].copy_from_slice(line.as_bytes());
        Self {
            bytes,
            len: line.len() as u16,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticEnqueueOutcome {
    Queued,
    Dropped,
}

#[derive(Clone)]
struct DiagnosticQueue {
    sender: SyncSender<DiagnosticRecord>,
}

impl DiagnosticQueue {
    fn new(sender: SyncSender<DiagnosticRecord>) -> Self {
        Self { sender }
    }

    fn try_enqueue(&self, record: DiagnosticRecord) -> DiagnosticEnqueueOutcome {
        match self.sender.try_send(record) {
            Ok(()) => DiagnosticEnqueueOutcome::Queued,
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                DiagnosticEnqueueOutcome::Dropped
            }
        }
    }
}

fn run_diagnostic_writer(receiver: Receiver<DiagnosticRecord>, log_directory: PathBuf) {
    while let Ok(record) = receiver.recv() {
        let _ =
            persist_diagnostic_record_with_limit(&log_directory, &record, MAX_DIAGNOSTIC_LOG_BYTES);
    }
}

fn persist_diagnostic_record_with_limit(
    log_directory: &Path,
    record: &DiagnosticRecord,
    max_bytes: usize,
) -> std::io::Result<()> {
    let line = record.as_bytes();
    if max_bytes == 0 || line.len() > max_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "diagnostic record exceeds log limit",
        ));
    }

    fs::create_dir_all(log_directory)?;
    let current = log_directory.join(DIAGNOSTIC_LOG_FILE);
    let rotated = log_directory.join(DIAGNOSTIC_ROTATED_LOG_FILE);
    if fs::metadata(&rotated)
        .map(|metadata| metadata.len() > max_bytes as u64)
        .unwrap_or(false)
    {
        fs::remove_file(&rotated)?;
    }

    let current_bytes = fs::metadata(&current)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if current_bytes > max_bytes as u64 {
        fs::remove_file(&current)?;
    } else if current_bytes.saturating_add(line.len() as u64) > max_bytes as u64 {
        if rotated.exists() {
            fs::remove_file(&rotated)?;
        }
        fs::rename(&current, &rotated)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(current)?;
    file.write_all(line)?;
    file.flush()?;
    file.sync_data()
}

pub fn report_diagnostic(app: &tauri::AppHandle, operation: &'static str, error: &str) {
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let record = DiagnosticRecord::new(&timestamp, operation, error);
    let _ = app
        .state::<crate::AppState>()
        .native_surface
        .try_enqueue_diagnostic(record);
    let _ = app.emit(
        "native-surface-diagnostic",
        serde_json::json!({
            "timestamp": timestamp,
            "operation": sanitize_operation(operation),
            "nativeCode": native_error_code(error),
            "status": "failed",
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::mpsc::sync_channel;

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

    #[test]
    fn a_usage_fetch_failure_records_its_http_status_so_the_cause_is_visible() {
        let line = format_diagnostic_line(
            "2026-08-02T12:34:56Z",
            "usage-fetch-codex",
            "http status 429",
        );
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();

        assert_eq!(value["operation"], "usage-fetch-codex");
        assert_eq!(value["nativeCode"], "http-429");
    }

    #[test]
    fn the_post_hide_caption_restrip_reports_under_its_own_operation_name() {
        // The settings close sequence repairs twice — once before hide, once after, because the
        // hide itself reinstates the caption style. They report separately so a diagnostic says
        // which one failed instead of collapsing to an indistinguishable "settings-repair".
        let line = format_diagnostic_line(
            "2026-08-03T12:34:56Z",
            "settings-close-repair",
            "Access is denied. (os error 5)",
        );
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();

        assert_eq!(value["operation"], "settings-close-repair");
    }

    #[test]
    fn diagnostic_format_discards_secrets_and_paths() {
        let line = format_diagnostic_line(
            "2026-08-02T12:34:56Z",
            r#"startup-repair C:\private"#,
            r#"token=secret-value C:\Users\person\.codex\auth.json (os error 5)"#,
        );
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();

        assert_eq!(value["timestamp"], "2026-08-02T12:34:56Z");
        assert_eq!(value["operation"], "unknown-operation");
        assert_eq!(value["nativeCode"], "os-error-5");
        assert_eq!(value["message"], "native operation failed");
        assert!(!line.contains("secret-value"));
        assert!(!line.contains("Users"));
        assert!(!line.contains("auth.json"));
        assert!(!line.contains("private"));
    }

    #[test]
    fn writer_persistence_helper_writes_a_durable_structured_record() {
        let directory = tempfile::tempdir().unwrap();
        let record = DiagnosticRecord::new(
            "2026-08-02T12:34:56Z",
            "settings-hide",
            "Access is denied. (os error 5)",
        );

        persist_diagnostic_record_with_limit(directory.path(), &record, 1024).unwrap();

        let contents = fs::read_to_string(directory.path().join(DIAGNOSTIC_LOG_FILE)).unwrap();
        assert!(contents.contains("settings-hide"));
        assert!(contents.contains("os-error-5"));
        assert!(contents.ends_with('\n'));
    }

    #[test]
    fn writer_persistence_helper_rotates_within_the_size_bound() {
        let directory = tempfile::tempdir().unwrap();
        let limit = 512;

        for second in 0..12 {
            let record = DiagnosticRecord::new(
                &format!("2026-08-02T12:34:{second:02}Z"),
                "deferred-repair",
                "native failure (os error 5)",
            );
            persist_diagnostic_record_with_limit(directory.path(), &record, limit).unwrap();
        }

        let current = directory.path().join(DIAGNOSTIC_LOG_FILE);
        let rotated = directory.path().join(DIAGNOSTIC_ROTATED_LOG_FILE);
        assert!(fs::metadata(&current).unwrap().len() <= limit as u64);
        assert!(fs::metadata(&rotated).unwrap().len() <= limit as u64);
        assert!(fs::read_to_string(current).unwrap().contains("12:34:11Z"));
    }

    #[test]
    fn bounded_diagnostic_queue_drops_without_blocking_when_full_or_disconnected() {
        let (sender, receiver) = sync_channel(1);
        let queue = DiagnosticQueue::new(sender);
        let record = DiagnosticRecord::new(
            "2026-08-02T12:34:56Z",
            "settings-hide",
            "native failure (os error 5)",
        );

        assert_eq!(queue.try_enqueue(record), DiagnosticEnqueueOutcome::Queued);
        assert_eq!(queue.try_enqueue(record), DiagnosticEnqueueOutcome::Dropped);
        drop(receiver);
        assert_eq!(queue.try_enqueue(record), DiagnosticEnqueueOutcome::Dropped);
    }
}
