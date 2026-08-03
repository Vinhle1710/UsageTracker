pub mod config;
pub mod creds;
pub mod detect;
pub mod material;
pub mod model;
pub mod native_surface;
pub mod poller;
pub mod providers;
pub mod startup;
pub mod visibility;
pub mod window;

use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{Emitter, Manager};

pub struct AppState {
    pub manual_hidden: Mutex<bool>,
    pub sources: Mutex<detect::ActiveSources>,
    pub usage: Mutex<Vec<model::ProviderUsageEvent>>,
    pub usage_ready: AtomicBool,
    pub webview_ready: AtomicBool,
    pub usage_wake: tokio::sync::Notify,
    pub native_surface: native_surface::NativeSurfaceState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            manual_hidden: Mutex::new(false),
            sources: Mutex::new(detect::ActiveSources::default()),
            usage: Mutex::new(Vec::new()),
            usage_ready: AtomicBool::new(false),
            webview_ready: AtomicBool::new(false),
            usage_wake: tokio::sync::Notify::new(),
            native_surface: native_surface::NativeSurfaceState::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapPayload {
    pub sources: detect::ActiveSources,
    pub usage: Vec<model::ProviderUsageEvent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct GeometryRequest {
    pub corner: String,
    pub preferred: Option<String>,
    pub layout: String,
    pub scale: f32,
    pub expanded_provider_count: usize,
    pub bubble_count: usize,
    pub theme: String,
    pub background_color: String,
    pub card_opacity: f32,
    #[serde(default)]
    pub regions: Vec<material::LogicalCardRegion>,
    #[serde(default)]
    pub content_width: Option<f64>,
    #[serde(default)]
    pub content_height: Option<f64>,
    /// Transparent slack baked into `content_width`/`content_height` and the region offsets (see
    /// OVERLAY_HEADROOM in geometry.ts). The window has to overhang the work area by this much
    /// for the *content* to stay flush against the screen corner.
    #[serde(default)]
    pub headroom: f64,
}

#[tauri::command]
fn get_config(app: tauri::AppHandle) -> config::Config {
    app.path()
        .app_config_dir()
        .map(|p| p.join("config.json"))
        .map(|p| config::Config::load(&p))
        .unwrap_or_default()
        .sanitized()
}

#[tauri::command]
fn set_config(app: tauri::AppHandle, cfg: config::Config) -> Result<(), String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");
    let sanitized = cfg.sanitized();
    sanitized.save(&path).map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_always_on_top(sanitized.always_on_top);
        // set_always_on_top diffs tao's window flags and rewrites GWL_STYLE, restoring the caption.
        if let Err(error) = repair_window_surface_ordered(&app, "main", false) {
            native_surface::report_diagnostic(&app, "focus-repair", &error);
        }
    }
    let _ = app.emit("config-changed", &sanitized);
    Ok(())
}

#[tauri::command]
async fn get_bootstrap(state: tauri::State<'_, AppState>) -> Result<BootstrapPayload, String> {
    Ok(BootstrapPayload {
        sources: state.sources.lock().map(|value| *value).unwrap_or_default(),
        usage: state
            .usage
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default(),
    })
}

#[tauri::command]
fn mark_overlay_ready(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.webview_ready.store(true, Ordering::Release);
    apply_overlay_visibility_transition(&app, false)
}

/// Widens the overlay's native window region for the duration of a minimize/restore morph, then
/// (`region: None`) restores the exact card/bubble shapes. Without this the OS clips the morph
/// ghost the instant it crosses the transparent gap between cards or the old card's rounded
/// corner, which reads as the card or bubble being cropped mid-flight.
#[tauri::command]
fn set_morph_region(
    app: tauri::AppHandle,
    region: Option<material::LogicalCardRegion>,
) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    #[cfg(target_os = "windows")]
    {
        let cached = cached_main_regions(&app);
        let physical = region.and_then(|region| {
            let scale_factor = window.scale_factor().ok()?;
            material::physical_card_regions(&[region], scale_factor)
                .into_iter()
                .next()
        });
        material::apply_transient_region(&window, physical.as_ref(), &cached)?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, region);
    }
    Ok(())
}

#[tauri::command]
fn close_settings(app: tauri::AppHandle) -> Result<(), String> {
    let settings_window = app.get_webview_window("settings");
    let failures = run_settings_close_steps(
        || repair_window_surface_ordered(&app, "settings", false),
        || {
            settings_window
                .as_ref()
                .ok_or_else(|| "settings window unavailable".to_string())?
                .hide()
                .map_err(|error| error.to_string())
        },
        || restore_overlay_surface_ordered(&app, true),
    );
    for failure in &failures {
        native_surface::report_diagnostic(&app, failure.operation, &failure.error);
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "settings close completed with failures: {}",
            failures
                .iter()
                .map(|failure| failure.operation)
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct SettingsCloseFailure {
    operation: &'static str,
    error: String,
}

fn run_settings_close_steps<Repair, Hide, Restore>(
    repair: Repair,
    hide: Hide,
    restore: Restore,
) -> Vec<SettingsCloseFailure>
where
    Repair: Fn() -> Result<(), String>,
    Hide: FnOnce() -> Result<(), String>,
    Restore: FnOnce() -> Result<(), String>,
{
    let mut failures = Vec::new();
    let mut record = |operation, result| {
        if let Err(error) = result {
            failures.push(SettingsCloseFailure { operation, error });
        }
    };
    // Repairing before the hide keeps the window chromeless in the case where the hide itself
    // fails and it stays on screen.
    record("settings-repair", repair());
    record("settings-hide", hide());
    // hide() is a tao window-flag change, and tao rewrites GWL_STYLE from a flag set that always
    // contains WS_CAPTION | WS_SYSMENU. Without this second strip the window sits hidden with
    // caption styles, leaving the next code path that shows it responsible for repairing first.
    record("settings-close-repair", repair());
    record("overlay-restore", restore());
    failures
}

#[tauri::command]
fn list_monitors(app: tauri::AppHandle) -> Result<Vec<MonitorOption>, String> {
    let source = app
        .get_webview_window("main")
        .or_else(|| app.get_webview_window("settings"))
        .ok_or_else(|| "no app window".to_string())?;
    let monitors = source
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| MonitorOption {
            id: monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("screen-{}", index + 1)),
            label: window::friendly_monitor_label(
                index,
                monitor.name().map(String::as_str).unwrap_or_default(),
                monitor.size().width,
                monitor.size().height,
            ),
        })
        .collect::<Vec<_>>();
    Ok(monitors)
}

#[tauri::command]
fn apply_overlay_geometry(app: tauri::AppHandle, request: GeometryRequest) -> Result<(), String> {
    apply_overlay_geometry_ordered(&app, request)
}

fn run_geometry_update(
    expanded_provider_count: usize,
    bubble_count: usize,
    update: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if expanded_provider_count == 0 && bubble_count == 0 {
        return Ok(());
    }
    update()
}

fn apply_overlay_geometry_ordered(
    app: &tauri::AppHandle,
    request: GeometryRequest,
) -> Result<(), String> {
    run_geometry_update(
        request.expanded_provider_count,
        request.bubble_count,
        || apply_nonempty_overlay_geometry_ordered(app, request),
    )
}

fn apply_nonempty_overlay_geometry_ordered(
    app: &tauri::AppHandle,
    request: GeometryRequest,
) -> Result<(), String> {
    let webview = app
        .get_webview_window("main")
        .ok_or_else(|| "no main window".to_string())?;
    let monitors: Vec<window::MonitorInfo> = webview
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| window::MonitorInfo {
            id: monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("screen-{}", index + 1)),
            area: window::Rect {
                x: monitor.work_area().position.x,
                y: monitor.work_area().position.y,
                w: monitor.work_area().size.width,
                h: monitor.work_area().size.height,
            },
        })
        .collect();
    let chosen = window::choose_monitor(&monitors, request.preferred.as_deref())
        .ok_or_else(|| "no monitors available".to_string())?;
    let base_size = window::overlay_size(
        &request.layout,
        request.scale,
        request.expanded_provider_count,
        request.bubble_count,
    );
    let scale_factor = webview.scale_factor().map_err(|error| error.to_string())?;
    // `overlay_size` is logical; measurements arrive physical. Both are converted before they
    // are compared so the width floor holds on high-DPI monitors too.
    let physical = |logical: f64| (logical * scale_factor).round() as u32;
    let size = (
        window::resolve_overlay_width(
            physical(base_size.0 as f64),
            request.content_width.map(&physical),
            request.expanded_provider_count,
        )
        .clamp(1, 2048),
        request
            .content_height
            .map(&physical)
            .unwrap_or_else(|| physical(base_size.1 as f64))
            .clamp(1, 2048),
    );
    let tint = material::parse_tint(&request.background_color, request.card_opacity)
        .unwrap_or((7, 16, 31, 240));
    let selected = material::material_for_theme(&request.theme);
    let measured_regions = material::physical_card_regions(&request.regions, scale_factor);
    let regions = if measured_regions.is_empty() {
        let mut fallback = material::card_regions(
            size,
            &request.layout,
            request.expanded_provider_count,
            request.bubble_count,
            request.scale,
            &request.corner,
        );
        fallback.extend(material::bubble_regions(
            size,
            request.bubble_count,
            request.expanded_provider_count,
            request.scale,
            &request.corner,
        ));
        fallback
    } else {
        measured_regions
    };
    let app_state = app.state::<AppState>();
    let mut current = app_state
        .native_surface
        .cache
        .lock()
        .map(|state| state.clone())
        .map_err(|_| "native window state unavailable".to_string())?;
    #[cfg(target_os = "windows")]
    {
        material::apply_to_window(
            &webview,
            material::NativeMaterialSpec {
                material: selected,
                tint,
            },
            &regions,
            size,
            &mut current,
        )?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        current.material = Some(material::NativeMaterialSpec {
            material: selected,
            tint,
        });
        current.regions = regions.clone();
        current.size = Some(size);
    }
    *app_state
        .native_surface
        .cache
        .lock()
        .map_err(|_| "native window state unavailable".to_string())? = current;
    let (x, y) = window::offset_for_headroom(
        window::corner_position(chosen.area, size, &request.corner),
        physical(request.headroom) as i32,
        &request.corner,
    );
    webview
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn surface_repair_plan_for_event(
    label: &str,
    event: &tauri::WindowEvent,
) -> Option<window::SurfaceRepairPlan> {
    match event {
        tauri::WindowEvent::Focused(focused) => window::focus_surface_repair_plan(label, *focused),
        _ => None,
    }
}

fn cached_main_regions(app: &tauri::AppHandle) -> Vec<material::CardRegion> {
    let cached = app
        .state::<AppState>()
        .native_surface
        .cache
        .lock()
        .map(|state| state.clone())
        .unwrap_or_default();
    native_surface::repair_regions("main", &cached)
}

fn repair_window_surface_ordered(
    app: &tauri::AppHandle,
    label: &str,
    force_region: bool,
) -> Result<(), String> {
    let Some(window) = app.get_webview_window(label) else {
        return Ok(());
    };
    let regions = if label == "main" {
        cached_main_regions(app)
    } else {
        Vec::new()
    };
    material::repair_window_surface(&window, label, &regions, force_region)
}

fn schedule_deferred_surface_repair(
    app: &tauri::AppHandle,
    label: &'static str,
    force_region: bool,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let should_schedule = state
        .native_surface
        .pending_repairs
        .lock()
        .map_err(|_| "native repair state unavailable".to_string())?
        .request(label, force_region);
    if !should_schedule {
        return Ok(());
    }
    schedule_pending_surface_repair(app, label)
}

fn schedule_pending_surface_repair(
    app: &tauri::AppHandle,
    label: &'static str,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let callback_app = app.clone();
    if let Err(error) = native_surface::enqueue_non_blocking(
        |operation| {
            app.run_on_main_thread(operation)
                .map_err(|error| error.to_string())
        },
        move || {
            let pending = callback_app
                .state::<AppState>()
                .native_surface
                .pending_repairs
                .lock()
                .ok()
                .and_then(|mut repairs| repairs.take(label));
            let Some(pending) = pending else {
                return;
            };
            let result =
                repair_window_surface_ordered(&callback_app, pending.label, pending.force_region);
            let retry = callback_app
                .state::<AppState>()
                .native_surface
                .pending_repairs
                .lock()
                .ok()
                .map(|mut repairs| {
                    repairs.complete(pending.label);
                    result.is_err() && repairs.request_retry(pending)
                })
                .unwrap_or(false);
            if let Err(error) = result {
                native_surface::report_diagnostic(&callback_app, "deferred-repair", &error);
                if retry {
                    if let Err(schedule_error) =
                        schedule_pending_surface_repair(&callback_app, pending.label)
                    {
                        native_surface::report_diagnostic(
                            &callback_app,
                            "deferred-repair-schedule",
                            &schedule_error,
                        );
                    }
                }
            }
        },
    ) {
        if let Ok(mut repairs) = state.native_surface.pending_repairs.lock() {
            repairs.clear(label);
        }
        return Err(format!("native surface scheduling failed: {error}"));
    }
    Ok(())
}

fn repair_windows_on_startup(app: &tauri::AppHandle) {
    for (label, force_region) in [("main", true), ("settings", false)] {
        if let Err(error) = repair_window_surface_ordered(app, label, force_region) {
            native_surface::report_diagnostic(app, "startup-repair", &error);
            if let Err(schedule_error) = schedule_deferred_surface_repair(app, label, force_region)
            {
                native_surface::report_diagnostic(app, "startup-repair-schedule", &schedule_error);
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin registered: it re-execs a duplicate launch into a message to
        // this callback and exits the duplicate before any other plugin or window gets created.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            toggle_overlay_visibility(app);
        }))
        .plugin(tauri_plugin_positioner::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            close_settings,
            list_monitors,
            apply_overlay_geometry,
            get_bootstrap,
            mark_overlay_ready,
            set_morph_region
        ])
        .on_window_event(|window, event| {
            if let Some(plan) = surface_repair_plan_for_event(window.label(), event) {
                let app = window.app_handle();
                if plan.immediate {
                    if let Err(error) = repair_window_surface_ordered(
                        app,
                        window.label(),
                        plan.restore_cached_main_region,
                    ) {
                        native_surface::report_diagnostic(app, "focus-repair", &error);
                    }
                }
                if plan.deferred {
                    if let Err(error) = schedule_deferred_surface_repair(
                        app,
                        if window.label() == "main" {
                            "main"
                        } else {
                            "settings"
                        },
                        plan.restore_cached_main_region,
                    ) {
                        native_surface::report_diagnostic(app, "focus-repair-schedule", &error);
                    }
                }
            }
        })
        .setup(|app| {
            if let Ok(log_directory) = app.path().app_log_dir() {
                app.state::<AppState>()
                    .native_surface
                    .initialize_diagnostic_writer(log_directory);
            }
            startup::register_current_executable();
            repair_windows_on_startup(app.handle());

            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let toggle = MenuItem::with_id(app, "toggle", "Show/Hide", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;
            TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .ok_or("missing default icon")?
                        .clone(),
                )
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        toggle_overlay_visibility(app);
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("settings") {
                            if let Err(error) =
                                repair_window_surface_ordered(app, "settings", false)
                            {
                                native_surface::report_diagnostic(app, "settings-repair", &error);
                                if let Err(schedule_error) =
                                    schedule_deferred_surface_repair(app, "settings", false)
                                {
                                    native_surface::report_diagnostic(
                                        app,
                                        "settings-repair-schedule",
                                        &schedule_error,
                                    );
                                }
                                return;
                            }
                            if let Err(error) = window.show() {
                                native_surface::report_diagnostic(
                                    app,
                                    "settings-show",
                                    &error.to_string(),
                                );
                                return;
                            }
                            if let Err(error) = window.set_focus() {
                                native_surface::report_diagnostic(
                                    app,
                                    "settings-focus",
                                    &error.to_string(),
                                );
                            }
                            // show/set_focus restore the caption style through tao's flag diffing.
                            if let Err(error) =
                                repair_window_surface_ordered(app, "settings", false)
                            {
                                native_surface::report_diagnostic(app, "settings-repair", &error);
                            }
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            let mut initial_system = sysinfo::System::new();
            let (initial_names, initial_pids) = detect::scan_processes(&mut initial_system);
            let initial_sources = detect::resolve(
                &initial_names,
                detect::has_live_ide_lock(&claude_ide_dir(), &initial_pids),
            );
            {
                let state = app.state::<AppState>();
                *state
                    .sources
                    .lock()
                    .unwrap_or_else(|error| error.into_inner()) = initial_sources;
            }

            let detection_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut system = initial_system;
                let mut previous = initial_sources;
                let mut first_tick = true;
                loop {
                    let (names, pids) = detect::scan_processes(&mut system);
                    let active = detect::resolve(
                        &names,
                        detect::has_live_ide_lock(&claude_ide_dir(), &pids),
                    );
                    let was_visible = previous.claude || previous.openai;
                    let visible = active.claude || active.openai;
                    let should_wake = visibility::new_provider_activated(previous, active);
                    let state = detection_handle.state::<AppState>();
                    *state
                        .sources
                        .lock()
                        .unwrap_or_else(|error| error.into_inner()) = active;
                    if !was_visible && visible {
                        state.usage_ready.store(false, Ordering::Release);
                    }
                    if !visible {
                        *state
                            .manual_hidden
                            .lock()
                            .unwrap_or_else(|error| error.into_inner()) = false;
                    }
                    let _ = reconcile_overlay_visibility(&detection_handle);
                    if should_wake {
                        state.usage_wake.notify_one();
                    }
                    if visibility::should_emit_sources_changed(previous, active, first_tick) {
                        let _ = detection_handle.emit("sources-changed", active);
                    }
                    previous = active;
                    first_tick = false;
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            });

            let usage_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::builder()
                    .connect_timeout(std::time::Duration::from_secs(3))
                    .timeout(std::time::Duration::from_secs(8))
                    .user_agent("usage-tracker-overlay/0.1")
                    .build()
                    .unwrap_or_default();
                let mut last_claude: Option<model::UsageSnapshot> = None;
                let mut last_codex: Option<model::UsageSnapshot> = None;
                let mut failures = ProviderFailures::default();
                let mut first = true;
                loop {
                    let sources = usage_handle
                        .state::<AppState>()
                        .sources
                        .lock()
                        .map(|value| *value)
                        .unwrap_or_default();
                    let visible = usage_handle
                        .get_webview_window("main")
                        .and_then(|window| window.is_visible().ok())
                        .unwrap_or(false);
                    if first
                        || visible
                        || !usage_handle
                            .state::<AppState>()
                            .usage_ready
                            .load(Ordering::Acquire)
                    {
                        let cycle = fetch_usage_cycle(
                            &client,
                            sources,
                            last_claude.as_ref(),
                            last_codex.as_ref(),
                            failures,
                        )
                        .await;
                        let UsageCycle {
                            events,
                            failures: next_failures,
                            diagnostics,
                        } = cycle;
                        failures = next_failures;
                        for (operation, detail) in &diagnostics {
                            native_surface::report_diagnostic(&usage_handle, operation, detail);
                        }
                        let usage_state = usage_handle.state::<AppState>();
                        let source_guard = {
                            let current_sources = usage_state
                                .sources
                                .lock()
                                .unwrap_or_else(|error| error.into_inner());
                            let complete = visibility::usage_cycle_is_complete(
                                sources,
                                *current_sources,
                                &events,
                            );
                            if complete {
                                Some(current_sources)
                            } else {
                                usage_state.usage_ready.store(false, Ordering::Release);
                                None
                            }
                        };
                        let Some(source_guard) = source_guard else {
                            first = true;
                            continue;
                        };
                        for event in &events {
                            match event.provider {
                                model::Provider::Claude => {
                                    last_claude = Some(event.snapshot.clone())
                                }
                                model::Provider::Openai => {
                                    last_codex = Some(event.snapshot.clone())
                                }
                            }
                            let _ = usage_handle.emit("usage-changed", event.clone());
                        }
                        cache_usage(&usage_handle, events);
                        usage_state.usage_ready.store(true, Ordering::Release);
                        drop(source_guard);
                        let _ = reconcile_overlay_visibility(&usage_handle);
                    }
                    first = false;
                    let app_state = usage_handle.state::<AppState>();
                    // A failing provider is retried sooner than the steady-state interval so a
                    // transient blip clears in seconds rather than lingering for a full minute.
                    let delay = poller::retry_delay_seconds(failures.claude.max(failures.openai));
                    wait_for_usage_poll(
                        &app_state.usage_wake,
                        std::time::Duration::from_secs(delay),
                    )
                    .await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running usage tracker");
}

fn toggle_overlay_visibility(app: &tauri::AppHandle) {
    if let Err(error) = apply_overlay_visibility_transition(app, true) {
        native_surface::report_diagnostic(app, "visibility-toggle", &error);
    }
}

fn reconcile_overlay_visibility(app: &tauri::AppHandle) -> Result<(), String> {
    dispatch_overlay_visibility_transition(app, false)
}

fn dispatch_overlay_visibility_transition(
    app: &tauri::AppHandle,
    toggle: bool,
) -> Result<(), String> {
    let callback_app = app.clone();
    native_surface::enqueue_non_blocking(
        |operation| {
            app.run_on_main_thread(operation)
                .map_err(|error| error.to_string())
        },
        move || {
            if let Err(error) = apply_overlay_visibility_transition(&callback_app, toggle) {
                native_surface::report_diagnostic(&callback_app, "visibility-transition", &error);
            }
        },
    )
}

fn apply_overlay_visibility_transition(app: &tauri::AppHandle, toggle: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let was_visible = window.is_visible().unwrap_or(false);
    if toggle {
        *state
            .manual_hidden
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = was_visible;
    }
    let active = state
        .sources
        .lock()
        .map(|sources| sources.claude || sources.openai)
        .unwrap_or(false);
    let manually_hidden = state
        .manual_hidden
        .lock()
        .map(|value| *value)
        .unwrap_or(false);
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let currently_visible = window.is_visible().unwrap_or(false);
    let mut controller = visibility::VisibilityTransitionController::new(currently_visible);
    let transition = controller.next(
        active,
        state.webview_ready.load(Ordering::Acquire),
        manually_hidden,
    );
    match transition {
        visibility::WindowTransition::Show => {
            restore_overlay_surface_ordered(app, true)?;
            window.show().map_err(|error| error.to_string())?;
        }
        visibility::WindowTransition::Hide => {
            window.hide().map_err(|error| error.to_string())?;
        }
        visibility::WindowTransition::Unchanged => {}
    }
    if toggle && !was_visible && window.is_visible().unwrap_or(false) {
        window.set_focus().map_err(|error| error.to_string())?;
    }
    // show/hide/set_focus all run through tao's window-flag diffing, which restores the caption
    // style. Repairing beforehand is not enough; the frame has to be re-stripped afterwards.
    if visibility::requires_borderless_reenforcement(transition) || toggle {
        restore_overlay_surface_ordered(app, false)?;
    }
    Ok(())
}

async fn wait_for_usage_poll(wake: &tokio::sync::Notify, interval: std::time::Duration) {
    tokio::select! {
        _ = tokio::time::sleep(interval) => {},
        _ = wake.notified() => {},
    }
}

fn restore_overlay_surface_ordered(
    app: &tauri::AppHandle,
    force_region: bool,
) -> Result<(), String> {
    repair_window_surface_ordered(app, "main", force_region)
}

fn cache_usage(app: &tauri::AppHandle, events: Vec<model::ProviderUsageEvent>) {
    let state = app.state::<AppState>();
    let Ok(mut cache) = state.usage.lock() else {
        return;
    };
    for event in events {
        if let Some(existing) = cache
            .iter_mut()
            .find(|item| item.provider == event.provider)
        {
            *existing = event;
        } else {
            cache.push(event);
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ProviderFailures {
    claude: u32,
    openai: u32,
}

struct UsageCycle {
    events: Vec<model::ProviderUsageEvent>,
    failures: ProviderFailures,
    diagnostics: Vec<(&'static str, String)>,
}

/// A served response that we could not turn into usage is worth recording: without it there is
/// no way to tell an expired token from a blocked request from a genuine outage.
fn usage_diagnostic(
    operation: &'static str,
    status: u16,
    snapshot: &model::UsageSnapshot,
) -> Option<(&'static str, String)> {
    if snapshot.state == model::SnapshotState::Fresh {
        return None;
    }
    Some((operation, format!("http status {status}")))
}

fn claude_snapshot_from_response(
    response: providers::FetchResponse,
    last: Option<&model::UsageSnapshot>,
    now: i64,
    previous_failures: u32,
) -> model::UsageSnapshot {
    let status_state = providers::state_for_status(response.status);
    let parsed = response
        .body
        .as_ref()
        .map(|value| providers::claude::parse_usage_checked(value, now, status_state));
    match parsed {
        Some(Ok(snapshot)) if !snapshot.windows.is_empty() => snapshot,
        // A body served on a good status that we cannot read is a contract break, not an outage.
        Some(Err(_)) if status_state == model::SnapshotState::Fresh => {
            poller::retain_last_good(last, now, model::SnapshotState::Error)
        }
        _ => poller::retain_last_good(
            last,
            now,
            poller::state_for_failed_refresh(
                last,
                next_failure_count(previous_failures, false),
                status_state,
            ),
        ),
    }
}

async fn fetch_usage_cycle(
    client: &reqwest::Client,
    sources: detect::ActiveSources,
    last_claude: Option<&model::UsageSnapshot>,
    last_codex: Option<&model::UsageSnapshot>,
    failures: ProviderFailures,
) -> UsageCycle {
    let now = unix_now();
    let claude = async {
        if !sources.claude {
            return (None, 0, None);
        }
        let (snapshot, diagnostic) = match claude_access_token(client, &claude_creds_path(), now)
            .await
        {
            Ok(token) => match providers::fetch_response(
                client,
                "https://api.anthropic.com/api/oauth/usage",
                &token,
                &[("anthropic-beta", "oauth-2025-04-20")],
            )
            .await
            {
                Ok(response) => {
                    let status = response.status;
                    let snapshot =
                        claude_snapshot_from_response(response, last_claude, now, failures.claude);
                    let diagnostic = usage_diagnostic("usage-fetch-claude", status, &snapshot);
                    (snapshot, diagnostic)
                }
                Err(error) => (
                    poller::retain_last_good(
                        last_claude,
                        now,
                        poller::state_for_failed_refresh(
                            last_claude,
                            next_failure_count(failures.claude, false),
                            providers::state_for_error(&error),
                        ),
                    ),
                    Some(("usage-fetch-claude", "transport failure".to_string())),
                ),
            },
            Err(error) => (
                claude_snapshot_for_error(last_claude, now, error),
                Some(("usage-fetch-claude", "token unavailable".to_string())),
            ),
        };
        let succeeded = snapshot.state == model::SnapshotState::Fresh;
        (
            Some(model::ProviderUsageEvent {
                provider: model::Provider::Claude,
                snapshot,
            }),
            next_failure_count(failures.claude, succeeded),
            diagnostic,
        )
    };
    let codex = async {
        if !sources.openai {
            return (None, 0, None);
        }
        let (snapshot, diagnostic) = match creds::read_codex_credentials(&codex_auth_path()) {
            Ok(credentials) => match providers::fetch_response(
                client,
                // No `/api/` segment: `/backend-api/api/codex/usage` 404s cleanly even with a
                // valid token. Confirmed against the live endpoint on 2026-08-02 — see
                // memory/codex-usage-endpoint-404.md for how this was diagnosed.
                "https://chatgpt.com/backend-api/codex/usage",
                &credentials.access_token,
                &[("chatgpt-account-id", credentials.account_id.as_str())],
            )
            .await
            {
                Ok(response) => {
                    let status = response.status;
                    let snapshot = codex_snapshot_from_response(
                        response,
                        &codex_sessions_dir(),
                        last_codex,
                        now,
                        failures.openai,
                    );
                    let diagnostic = usage_diagnostic("usage-fetch-codex", status, &snapshot);
                    (snapshot, diagnostic)
                }
                Err(error) => (
                    providers::codex::latest_rate_limits_from_sessions(&codex_sessions_dir())
                        .map(|value| {
                            providers::codex::parse_rate_limits(
                                &value,
                                now,
                                model::SnapshotState::Stale,
                            )
                        })
                        .unwrap_or_else(|| {
                            poller::retain_last_good(
                                last_codex,
                                now,
                                poller::state_for_failed_refresh(
                                    last_codex,
                                    next_failure_count(failures.openai, false),
                                    providers::state_for_error(&error),
                                ),
                            )
                        }),
                    Some(("usage-fetch-codex", "transport failure".to_string())),
                ),
            },
            Err(_) => (
                poller::retain_last_good(last_codex, now, model::SnapshotState::Error),
                Some(("usage-fetch-codex", "token unavailable".to_string())),
            ),
        };
        let succeeded = snapshot.state == model::SnapshotState::Fresh;
        (
            Some(model::ProviderUsageEvent {
                provider: model::Provider::Openai,
                snapshot,
            }),
            next_failure_count(failures.openai, succeeded),
            diagnostic,
        )
    };
    let (claude, codex) = tokio::join!(claude, codex);
    UsageCycle {
        events: claude.0.into_iter().chain(codex.0).collect(),
        failures: ProviderFailures {
            claude: claude.1,
            openai: codex.1,
        },
        diagnostics: claude.2.into_iter().chain(codex.2).collect(),
    }
}

fn next_failure_count(previous: u32, succeeded: bool) -> u32 {
    if succeeded {
        0
    } else {
        previous.saturating_add(1)
    }
}

/// Resolves a live `chatgpt.com/backend-api/codex/usage` response into the best usage we can
/// honestly show, in descending order of authority: numbers the response actually carried (even
/// on a non-2xx status), then the last numbers a local Codex session recorded, then whatever was
/// last fetched. Nothing is invented — a failed request never asserts a usage figure of its own.
fn codex_snapshot_from_response(
    response: providers::FetchResponse,
    sessions_dir: &std::path::Path,
    last: Option<&model::UsageSnapshot>,
    now: i64,
    previous_failures: u32,
) -> model::UsageSnapshot {
    let status_state = providers::state_for_status(response.status);
    let served = response
        .body
        .as_ref()
        .map(|value| providers::codex::parse_account_usage(value, now, status_state));
    match served {
        // Numbers the response carried win regardless of the status it carried them on.
        Some(snapshot) if !snapshot.windows.is_empty() => snapshot,
        // A clean response that reports no windows is authoritative; resurrecting older numbers
        // from disk would contradict the provider's own answer.
        Some(snapshot) if status_state == model::SnapshotState::Fresh => snapshot,
        _ => {
            if let Some(value) = providers::codex::latest_rate_limits_from_sessions(sessions_dir) {
                return providers::codex::parse_rate_limits(
                    &value,
                    now,
                    model::SnapshotState::Stale,
                );
            }
            poller::retain_last_good(
                last,
                now,
                poller::state_for_failed_refresh(
                    last,
                    next_failure_count(previous_failures, false),
                    status_state,
                ),
            )
        }
    }
}

fn claude_snapshot_for_error(
    last_claude: Option<&model::UsageSnapshot>,
    now: i64,
    error: providers::FetchError,
) -> model::UsageSnapshot {
    poller::retain_last_good(last_claude, now, providers::state_for_error(&error))
}

async fn claude_access_token(
    client: &reqwest::Client,
    path: &std::path::Path,
    now_seconds: i64,
) -> Result<String, providers::FetchError> {
    let contents =
        std::fs::read_to_string(path).map_err(|_| providers::FetchError::Unauthorized)?;
    let credentials =
        creds::claude_oauth_from_str(&contents).map_err(|_| providers::FetchError::Unauthorized)?;
    let now_millis = now_seconds.saturating_mul(1_000);
    if !credentials.needs_refresh(now_millis) {
        return Ok(credentials.access_token);
    }
    let refresh_token = credentials
        .refresh_token
        .as_deref()
        .ok_or(providers::FetchError::Unauthorized)?;
    let refreshed = providers::claude::refresh_access_token(
        client,
        "https://platform.claude.com/v1/oauth/token",
        refresh_token,
    )
    .await?;
    let saved = creds::persist_claude_refresh(path, &refreshed, now_millis)
        .map_err(|_| providers::FetchError::Malformed)?;
    Ok(saved.access_token)
}

fn home() -> std::path::PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_default()
}
fn claude_ide_dir() -> std::path::PathBuf {
    home().join(".claude").join("ide")
}
fn claude_creds_path() -> std::path::PathBuf {
    home().join(".claude").join(".credentials.json")
}
fn codex_auth_path() -> std::path::PathBuf {
    home().join(".codex").join("auth.json")
}
fn codex_sessions_dir() -> std::path::PathBuf {
    home().join(".codex").join("sessions")
}
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::time::Duration;

    #[test]
    fn geometry_request_uses_the_mixed_layout_contract_without_a_legacy_pill_flag() {
        let request: GeometryRequest = serde_json::from_value(serde_json::json!({
            "corner": "bottom-right",
            "preferred": null,
            "layout": "stacked-compact",
            "scale": 1.0,
            "expandedProviderCount": 1,
            "bubbleCount": 1,
            "theme": "frosted",
            "backgroundColor": "#07101f",
            "cardOpacity": 0.98,
            "contentWidth": 326.0,
            "contentHeight": 190.0,
            "regions": []
        }))
        .expect("mixed geometry request should deserialize");

        assert_eq!(request.expanded_provider_count, 1);
        assert_eq!(request.bubble_count, 1);
        assert_eq!(request.content_width, Some(326.0));
        assert!(
            serde_json::from_value::<GeometryRequest>(serde_json::json!({
                "corner": "bottom-right",
                "preferred": null,
                "layout": "stacked-compact",
                "scale": 1.0,
                "providerCount": 2,
                "minimized": true,
                "theme": "frosted",
                "backgroundColor": "#07101f",
                "cardOpacity": 0.98,
                "regions": []
            }))
            .is_err()
        );
    }

    #[test]
    fn empty_geometry_request_returns_ok_without_touching_cached_native_geometry() {
        let cached = RefCell::new(material::NativeWindowState {
            material: None,
            regions: vec![material::CardRegion {
                x: 8,
                y: 8,
                width: 310,
                height: 166,
                radius: 14,
            }],
            size: Some((326, 182)),
        });
        let original = cached.borrow().clone();

        let result = run_geometry_update(0, 0, || {
            cached.borrow_mut().size = Some((1, 1));
            cached.borrow_mut().regions.clear();
            Err("empty update should not run".to_string())
        });

        assert_eq!(result, Ok(()));
        assert_eq!(*cached.borrow(), original);
    }

    #[test]
    fn focus_events_request_a_second_deferred_repair_for_main_and_settings() {
        for focused in [true, false] {
            let event = tauri::WindowEvent::Focused(focused);
            assert_eq!(
                surface_repair_plan_for_event("main", &event),
                Some(window::SurfaceRepairPlan {
                    immediate: true,
                    deferred: true,
                    restore_cached_main_region: false,
                })
            );
            assert_eq!(
                surface_repair_plan_for_event("settings", &event),
                Some(window::SurfaceRepairPlan {
                    immediate: true,
                    deferred: true,
                    restore_cached_main_region: false,
                })
            );
        }
    }

    #[test]
    fn settings_close_attempts_hide_and_restore_after_repair_failure() {
        let calls = RefCell::new(Vec::new());

        let failures = run_settings_close_steps(
            || {
                calls.borrow_mut().push("repair");
                Err("repair failed".to_string())
            },
            || {
                calls.borrow_mut().push("hide");
                Err("hide failed".to_string())
            },
            || {
                calls.borrow_mut().push("restore");
                Err("restore failed".to_string())
            },
        );

        assert_eq!(*calls.borrow(), vec!["repair", "hide", "repair", "restore"]);
        assert_eq!(
            failures
                .iter()
                .map(|failure| failure.operation)
                .collect::<Vec<_>>(),
            vec![
                "settings-repair",
                "settings-hide",
                "settings-close-repair",
                "overlay-restore"
            ]
        );
    }

    #[test]
    fn settings_close_restrips_the_caption_that_hide_reinstates() {
        // tao rewrites GWL_STYLE from its own flag set on every window-flag change, and that set
        // always contains WS_CAPTION | WS_SYSMENU (tao 0.35.3, window_state.rs `to_window_styles`
        // / `apply_diff`). hide() is such a change, so repairing only *before* the hide leaves the
        // hidden window carrying caption styles — the reopen path happens to strip them again
        // before anything is visible, which is exactly what makes this easy to reintroduce.
        let calls = RefCell::new(Vec::new());

        let failures = run_settings_close_steps(
            || {
                calls.borrow_mut().push("repair");
                Ok(())
            },
            || {
                calls.borrow_mut().push("hide");
                Ok(())
            },
            || {
                calls.borrow_mut().push("restore");
                Ok(())
            },
        );

        assert_eq!(*calls.borrow(), vec!["repair", "hide", "repair", "restore"]);
        assert!(failures.is_empty());
    }

    #[test]
    fn a_rate_limited_codex_response_reports_the_usage_its_body_carried() {
        // The reported bug: burning through the quota produced "Usage temporarily unavailable"
        // because the non-2xx body was discarded before anyone looked at it.
        let response = providers::FetchResponse {
            status: 429,
            body: Some(serde_json::json!({
                "rate_limit": {
                    "primary_window": {"used_percent": 100.0, "limit_window_seconds": 18000, "reset_at": 42},
                    "secondary_window": null
                }
            })),
        };

        let snapshot = codex_snapshot_from_response(
            response,
            std::path::Path::new("nonexistent"),
            None,
            200,
            1,
        );

        assert_eq!(snapshot.windows.len(), 1);
        assert_eq!(snapshot.windows[0].used_percent, 100.0);
        assert_eq!(snapshot.windows[0].label, "5 hour");
        assert_eq!(snapshot.state, model::SnapshotState::Stale);
    }

    #[test]
    fn a_bodyless_failure_with_nothing_fetched_yet_stays_pending_rather_than_claiming_unavailable()
    {
        let snapshot = codex_snapshot_from_response(
            providers::FetchResponse {
                status: 429,
                body: None,
            },
            std::path::Path::new("nonexistent"),
            None,
            200,
            1,
        );

        assert!(snapshot.windows.is_empty());
        assert_eq!(snapshot.state, model::SnapshotState::Pending);
    }

    #[test]
    fn a_bodyless_failure_keeps_previously_fetched_numbers_on_screen() {
        let previous = model::UsageSnapshot {
            windows: vec![model::UsageWindow {
                label: "5 hour".into(),
                used_percent: 87.0,
                resets_at: 10,
            }],
            fetched_at: 100,
            state: model::SnapshotState::Fresh,
        };

        let snapshot = codex_snapshot_from_response(
            providers::FetchResponse {
                status: 503,
                body: None,
            },
            std::path::Path::new("nonexistent"),
            Some(&previous),
            200,
            1,
        );

        assert_eq!(snapshot.windows, previous.windows);
        assert_eq!(snapshot.state, model::SnapshotState::Stale);
    }

    #[test]
    fn a_clean_response_is_reported_fresh() {
        let snapshot = codex_snapshot_from_response(
            providers::FetchResponse {
                status: 200,
                body: Some(serde_json::json!({
                    "rate_limit": {"primary_window": {"used_percent": 12.0, "limit_window_seconds": 18000}}
                })),
            },
            std::path::Path::new("nonexistent"),
            None,
            200,
            0,
        );

        assert_eq!(snapshot.state, model::SnapshotState::Fresh);
        assert_eq!(snapshot.windows[0].used_percent, 12.0);
    }

    #[test]
    fn a_clean_response_reporting_no_limits_is_believed_rather_than_overridden() {
        let snapshot = codex_snapshot_from_response(
            providers::FetchResponse {
                status: 200,
                body: Some(serde_json::json!({"rate_limit": {"secondary_window": null}})),
            },
            std::path::Path::new("nonexistent"),
            None,
            200,
            0,
        );

        assert!(snapshot.windows.is_empty());
        assert_eq!(snapshot.state, model::SnapshotState::Fresh);
    }

    #[test]
    fn a_consecutive_failure_count_resets_only_on_a_clean_read() {
        assert_eq!(next_failure_count(0, false), 1);
        assert_eq!(next_failure_count(3, false), 4);
        assert_eq!(next_failure_count(3, true), 0);
        assert_eq!(next_failure_count(u32::MAX, false), u32::MAX);
    }

    fn previous_claude_snapshot() -> model::UsageSnapshot {
        model::UsageSnapshot {
            windows: vec![
                model::UsageWindow {
                    label: "5 hour".into(),
                    used_percent: 42.5,
                    resets_at: 1_234,
                },
                model::UsageWindow {
                    label: "Weekly".into(),
                    used_percent: 18.0,
                    resets_at: 5_678,
                },
            ],
            fetched_at: 100,
            state: model::SnapshotState::Fresh,
        }
    }

    #[test]
    fn transient_claude_token_failure_retains_provider_owned_usage() {
        let previous = previous_claude_snapshot();
        let snapshot =
            claude_snapshot_for_error(Some(&previous), 200, providers::FetchError::Network);

        assert_eq!(snapshot.state, model::SnapshotState::Stale);
        assert_eq!(snapshot.fetched_at, 200);
        assert_eq!(snapshot.windows, previous.windows);
    }

    #[test]
    fn malformed_claude_payload_retains_provider_owned_usage() {
        let previous = previous_claude_snapshot();
        let malformed = serde_json::json!({
            "five_hour": null,
            "seven_day": null,
        });

        let snapshot = match providers::claude::parse_usage_checked(
            &malformed,
            200,
            model::SnapshotState::Fresh,
        ) {
            Ok(snapshot) => snapshot,
            Err(error) => claude_snapshot_for_error(Some(&previous), 200, error),
        };

        assert_eq!(snapshot.windows, previous.windows);
        assert_eq!(snapshot.state, model::SnapshotState::Error);
    }

    #[tokio::test]
    async fn usage_poll_wait_wakes_immediately_when_a_provider_activates() {
        let wake = tokio::sync::Notify::new();
        wake.notify_one();

        tokio::time::timeout(
            Duration::from_millis(100),
            wait_for_usage_poll(&wake, Duration::from_secs(60)),
        )
        .await
        .expect("provider activation should wake the poll immediately");
    }
}
