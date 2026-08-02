pub mod config;
pub mod creds;
pub mod detect;
pub mod material;
pub mod model;
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
    pub native_window: Mutex<material::NativeWindowState>,
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
            native_window: Mutex::new(material::NativeWindowState::default()),
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
pub struct GeometryRequest {
    pub corner: String,
    pub preferred: Option<String>,
    pub layout: String,
    pub scale: f32,
    pub provider_count: usize,
    pub minimized: bool,
    pub theme: String,
    pub background_color: String,
    pub card_opacity: f32,
    #[serde(default)]
    pub regions: Vec<material::LogicalCardRegion>,
    #[serde(default)]
    pub content_height: Option<f64>,
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
fn mark_overlay_ready(app: tauri::AppHandle, state: tauri::State<'_, AppState>) {
    state.webview_ready.store(true, Ordering::Release);
    reconcile_overlay_visibility(&app);
}

#[tauri::command]
fn close_settings(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("settings")
        .ok_or_else(|| "settings window unavailable".to_string())?
        .hide()
        .map_err(|e| e.to_string())?;
    restore_overlay_surface(&app, true);
    Ok(())
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
        request.provider_count,
        request.minimized,
    );
    let scale_factor = webview.scale_factor().map_err(|error| error.to_string())?;
    let size = if request.minimized {
        base_size
    } else {
        let measured_height = request
            .content_height
            .map(|height| (height * scale_factor).round() as u32)
            .unwrap_or(base_size.1)
            .clamp(120, 540);
        (base_size.0, measured_height)
    };
    #[cfg(target_os = "windows")]
    {
        let tint = material::parse_tint(&request.background_color, request.card_opacity)
            .unwrap_or((7, 16, 31, 240));
        let selected = if request.minimized {
            material::Material::Clear
        } else {
            material::material_for_theme(&request.theme)
        };
        let measured_regions = material::physical_card_regions(&request.regions, scale_factor);
        let regions = if measured_regions.is_empty() {
            material::card_regions(
                size,
                &request.layout,
                request.provider_count,
                request.minimized,
                request.scale,
            )
        } else {
            measured_regions
        };
        let app_state = app.state::<AppState>();
        let mut current = app_state
            .native_window
            .lock()
            .map_err(|_| "native window state unavailable".to_string())?;
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
    let (x, y) = window::corner_position(chosen.area, size, &request.corner);
    webview
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            close_settings,
            list_monitors,
            apply_overlay_geometry,
            get_bootstrap,
            mark_overlay_ready
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Focused(true)) {
                restore_overlay_surface(window.app_handle(), true);
            }
        })
        .setup(|app| {
            startup::register_current_executable();

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
                            let _ = window.show();
                            let _ = window.set_focus();
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
                    reconcile_overlay_visibility(&detection_handle);
                    if should_wake {
                        state.usage_wake.notify_one();
                    }
                    if active != previous {
                        let _ = detection_handle.emit("sources-changed", active);
                    }
                    previous = active;
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
                        let events = fetch_usage_cycle(
                            &client,
                            sources,
                            last_claude.as_ref(),
                            last_codex.as_ref(),
                        )
                        .await;
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
                        reconcile_overlay_visibility(&usage_handle);
                    }
                    first = false;
                    let app_state = usage_handle.state::<AppState>();
                    wait_for_usage_poll(&app_state.usage_wake, std::time::Duration::from_secs(60))
                        .await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running usage tracker");
}

fn toggle_overlay_visibility(app: &tauri::AppHandle) {
    dispatch_overlay_visibility_transition(app, true);
}

fn reconcile_overlay_visibility(app: &tauri::AppHandle) {
    dispatch_overlay_visibility_transition(app, false);
}

fn dispatch_overlay_visibility_transition(app: &tauri::AppHandle, toggle: bool) {
    let app = app.clone();
    let callback_app = app.clone();
    let _ = app.run_on_main_thread(move || {
        apply_overlay_visibility_transition(&callback_app, toggle);
    });
}

fn apply_overlay_visibility_transition(app: &tauri::AppHandle, toggle: bool) {
    let state = app.state::<AppState>();
    let Some(window) = app.get_webview_window("main") else {
        return;
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
        return;
    };
    let currently_visible = window.is_visible().unwrap_or(false);
    let mut controller = visibility::VisibilityTransitionController::new(currently_visible);
    match controller.next(
        active,
        state.webview_ready.load(Ordering::Acquire),
        manually_hidden,
    ) {
        visibility::WindowTransition::Show => {
            restore_overlay_surface(app, true);
            let _ = window.show();
            restore_overlay_surface(app, true);
        }
        visibility::WindowTransition::Hide => {
            let _ = window.hide();
        }
        visibility::WindowTransition::Unchanged => {}
    }
    if toggle && !was_visible && window.is_visible().unwrap_or(false) {
        let _ = window.set_focus();
    }
}

async fn wait_for_usage_poll(wake: &tokio::sync::Notify, interval: std::time::Duration) {
    tokio::select! {
        _ = tokio::time::sleep(interval) => {},
        _ = wake.notified() => {},
    }
}

fn restore_overlay_surface(app: &tauri::AppHandle, force_region: bool) {
    #[cfg(target_os = "windows")]
    if let Some(window) = app.get_webview_window("main") {
        let state = app.state::<AppState>();
        if let Ok(current) = state.native_window.lock() {
            let _ = material::restore_window_surface(&window, &current, force_region);
        };
    }
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

async fn fetch_usage_cycle(
    client: &reqwest::Client,
    sources: detect::ActiveSources,
    last_claude: Option<&model::UsageSnapshot>,
    last_codex: Option<&model::UsageSnapshot>,
) -> Vec<model::ProviderUsageEvent> {
    let now = unix_now();
    let claude = async {
        if !sources.claude {
            return None;
        }
        let snapshot = match claude_access_token(client, &claude_creds_path(), now).await {
            Ok(token) => match providers::fetch_json(
                client,
                "https://api.anthropic.com/api/oauth/usage",
                &token,
                &[("anthropic-beta", "oauth-2025-04-20")],
            )
            .await
            {
                Ok(value) => match providers::claude::parse_usage_checked(
                    &value,
                    now,
                    model::SnapshotState::Fresh,
                ) {
                    Ok(snapshot) => snapshot,
                    Err(error) => claude_snapshot_for_error(last_claude, now, error),
                },
                Err(error) => {
                    poller::retain_last_good(last_claude, now, providers::state_for_error(&error))
                }
            },
            Err(error) => claude_snapshot_for_error(last_claude, now, error),
        };
        Some(model::ProviderUsageEvent {
            provider: model::Provider::Claude,
            snapshot,
        })
    };
    let codex = async {
        if !sources.openai {
            return None;
        }
        let snapshot = match creds::read_token(&codex_auth_path(), creds::codex_token_from_str) {
            Ok(token) => match providers::fetch_json(
                client,
                "https://chatgpt.com/backend-api/api/codex/usage",
                &token,
                &[],
            )
            .await
            {
                Ok(value) => providers::codex::parse_rate_limits(
                    value.get("rate_limits").unwrap_or(&value),
                    now,
                    model::SnapshotState::Fresh,
                ),
                Err(error) => {
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
                                providers::state_for_error(&error),
                            )
                        })
                }
            },
            Err(_) => poller::retain_last_good(last_codex, now, model::SnapshotState::Error),
        };
        Some(model::ProviderUsageEvent {
            provider: model::Provider::Openai,
            snapshot,
        })
    };
    let (claude, codex) = tokio::join!(claude, codex);
    claude.into_iter().chain(codex).collect()
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
    use std::time::Duration;

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
