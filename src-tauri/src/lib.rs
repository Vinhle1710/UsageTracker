pub mod config;
pub mod creds;
pub mod detect;
pub mod model;
pub mod poller;
pub mod providers;
pub mod visibility;
pub mod window;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[derive(Default)]
pub struct AppState {
    pub manual_hidden: Mutex<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorOption {
    pub id: String,
    pub label: String,
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
                x: monitor.position().x,
                y: monitor.position().y,
                w: monitor.size().width,
                h: monitor.size().height,
            },
        })
        .collect();
    let chosen = window::choose_monitor(&monitors, request.preferred.as_deref())
        .ok_or_else(|| "no monitors available".to_string())?;
    let size = window::overlay_size(
        &request.layout,
        request.scale,
        request.provider_count,
        request.minimized,
    );
    webview
        .set_size(tauri::PhysicalSize::new(size.0, size.1))
        .map_err(|e| e.to_string())?;
    let (x, y) = window::corner_position(chosen.area, size, &request.corner);
    webview
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            list_monitors,
            apply_overlay_geometry
        ])
        .setup(|app| {
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
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                if let Ok(mut hidden) = app.state::<AppState>().manual_hidden.lock()
                                {
                                    *hidden = true;
                                }
                                let _ = window.hide();
                            } else {
                                if let Ok(mut hidden) = app.state::<AppState>().manual_hidden.lock()
                                {
                                    *hidden = false;
                                }
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
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

            let detection_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut system = sysinfo::System::new();
                loop {
                    let (names, pids) = detect::scan_processes(&mut system);
                    let active = detect::resolve(
                        &names,
                        detect::has_live_ide_lock(&claude_ide_dir(), &pids),
                    );
                    let visible = active.claude || active.openai;
                    if let Some(window) = detection_handle.get_webview_window("main") {
                        let manually_hidden = detection_handle
                            .state::<AppState>()
                            .manual_hidden
                            .lock()
                            .map(|hidden| *hidden)
                            .unwrap_or(false);
                        if !visible {
                            if let Ok(mut hidden) =
                                detection_handle.state::<AppState>().manual_hidden.lock()
                            {
                                *hidden = false;
                            }
                            let _ = window.hide();
                        } else if visibility::should_display(visible, manually_hidden) {
                            let _ = window.show();
                        }
                    }
                    let _ = detection_handle.emit("sources-changed", active);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            });

            let usage_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut last_claude: Option<model::UsageSnapshot> = None;
                let mut last_codex: Option<model::UsageSnapshot> = None;
                loop {
                    let visible = usage_handle
                        .get_webview_window("main")
                        .and_then(|window| window.is_visible().ok())
                        .unwrap_or(false);
                    if visible {
                        let now = unix_now();
                        if let Ok(token) =
                            creds::read_token(&claude_creds_path(), creds::claude_token_from_str)
                        {
                            let snapshot = match providers::fetch_json(
                                "https://api.anthropic.com/api/oauth/usage",
                                &token,
                                &[("anthropic-beta", "oauth-2025-04-20")],
                            )
                            .await
                            {
                                Ok(value) => providers::claude::parse_usage(
                                    &value,
                                    now,
                                    model::SnapshotState::Fresh,
                                ),
                                Err(error) => poller::retain_last_good(
                                    last_claude.as_ref(),
                                    now,
                                    providers::state_for_error(&error),
                                ),
                            };
                            last_claude = Some(snapshot.clone());
                            let _ = usage_handle.emit("claude-usage", snapshot);
                        }
                        if let Ok(token) =
                            creds::read_token(&codex_auth_path(), creds::codex_token_from_str)
                        {
                            let snapshot = match providers::fetch_json(
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
                                Err(error) => providers::codex::latest_rate_limits_from_sessions(
                                    &codex_sessions_dir(),
                                )
                                .map(|value| {
                                    providers::codex::parse_rate_limits(
                                        &value,
                                        now,
                                        model::SnapshotState::Stale,
                                    )
                                })
                                .unwrap_or_else(|| {
                                    poller::retain_last_good(
                                        last_codex.as_ref(),
                                        now,
                                        providers::state_for_error(&error),
                                    )
                                }),
                            };
                            last_codex = Some(snapshot.clone());
                            let _ = usage_handle.emit("codex-usage", snapshot);
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running usage tracker");
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
