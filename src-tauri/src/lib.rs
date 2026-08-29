pub mod auth;
pub mod automation;
pub mod config;
pub mod connectivity;
pub mod creds;
pub mod detect;
pub mod export;
pub mod history;
pub mod material;
pub mod model;
pub mod native_surface;
pub mod notification_store;
pub mod notifications;
pub mod pace;
pub mod poller;
pub mod popover;
pub mod power;
pub mod providers;
pub mod session_init;
pub mod shortcuts;
pub mod sound;
pub mod startup;
pub mod tray_actions;
pub mod visibility;
pub mod window;

use auth::secret_store::SecretStore;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{Emitter, Listener, Manager};

/// The PKCE verifier and CSRF state generated for one in-flight "Sign in with Claude" attempt.
/// Held only in memory between `start_claude_login` and `finish_claude_login` — never written to
/// disk, and cleared as soon as the login attempt is consumed (success or failure).
struct PendingClaudeLogin {
    verifier: String,
    state: String,
}

pub struct AppState {
    pub history: Mutex<Option<history::HistoryDb>>,
    pub history_error: Mutex<Option<String>>,
    pub manual_hidden: Mutex<bool>,
    /// The overlay is tucked to the screen edge: the cards/bubbles window is hidden and only
    /// the edge tab is on screen. Distinct from `manual_hidden`, which means "show nothing".
    pub tucked: Mutex<bool>,
    pub sources: Mutex<detect::ActiveSources>,
    pub usage: Mutex<Vec<model::ProviderUsageEvent>>,
    pub usage_ready: AtomicBool,
    pub webview_ready: AtomicBool,
    pub usage_wake: tokio::sync::Notify,
    pub online: AtomicBool,
    pub last_refresh_at: Mutex<Option<i64>>,
    pub auto_init_last_attempt_at: Mutex<Option<i64>>,
    pub auto_init_child: Mutex<Option<std::process::Child>>,
    pub manual_refresh_requested: AtomicBool,
    pub network_monitor: std::sync::Arc<AtomicBool>,
    pub network_shutdown: Mutex<Option<std::sync::mpsc::Sender<()>>>,
    pub coordinator: Mutex<automation::Coordinator>,
    pub monitor_network: AtomicBool,
    pub native_surface: native_surface::NativeSurfaceState,
    pending_claude_login: Mutex<Option<PendingClaudeLogin>>,
    pub auth_accounts: Mutex<Vec<auth::AccountSummary>>,
    pub auth_secrets: Mutex<auth::secret_store::MemoryStore>,
}

#[tauri::command]
fn reset_notification_history(app: tauri::AppHandle) -> Result<(), String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("notifications.json");
    let mut store = notification_store::NotificationStore::load(&path);
    store.reset();
    store.save(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn test_notification_sound(sound: String) -> Result<(), String> {
    let selected = sound::Sound::parse(&sound).ok_or_else(|| "unsupported sound".to_string())?;
    selected.play();
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub online: bool,
    pub last_refresh_at: Option<i64>,
    pub launch_at_login_registered: bool,
    pub auto_init_last_attempt_at: Option<i64>,
}

#[tauri::command]
fn refresh_usage(app: tauri::AppHandle) -> Result<(), String> {
    app.state::<AppState>()
        .manual_refresh_requested
        .store(true, Ordering::Release);
    route_automation_event(&app, automation::Event::ManualRefresh);
    Ok(())
}

pub(crate) fn runtime_status(app: tauri::AppHandle) -> RuntimeStatus {
    let state = app.state::<AppState>();
    let launch = startup::registration_state().unwrap_or(false);
    let last_refresh_at = *state
        .last_refresh_at
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let auto_init_last_attempt_at = *state
        .auto_init_last_attempt_at
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    RuntimeStatus {
        online: state.online.load(Ordering::Acquire),
        last_refresh_at,
        launch_at_login_registered: launch,
        auto_init_last_attempt_at,
    }
}
#[tauri::command]
fn get_runtime_status(app: tauri::AppHandle) -> RuntimeStatus {
    runtime_status(app)
}

fn route_automation_event(app: &tauri::AppHandle, event: automation::Event) {
    let state = app.state::<AppState>();
    if matches!(event, automation::Event::NetworkOnline) {
        state.online.store(true, Ordering::Release);
    }
    if matches!(event, automation::Event::NetworkOffline) {
        state.online.store(false, Ordering::Release);
    }
    let action = state
        .coordinator
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .on_event(event);
    if matches!(
        action,
        automation::Action::WakePoller | automation::Action::FetchNow
    ) {
        state.usage_wake.notify_one();
    }
    let _ = app.emit("runtime-status-changed", runtime_status(app.clone()));
}

#[tauri::command]
fn resize_popover(app: tauri::AppHandle, width: f64, height: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("popover")
        .ok_or_else(|| "popover unavailable".to_string())?;
    window
        .set_size(tauri::LogicalSize::new(
            width.clamp(240.0, 480.0),
            height.clamp(120.0, 640.0),
        ))
        .map_err(|_| "popover resize failed".into())
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            history: Mutex::new(None),
            history_error: Mutex::new(None),
            manual_hidden: Mutex::new(false),
            tucked: Mutex::new(false),
            sources: Mutex::new(detect::ActiveSources::default()),
            usage: Mutex::new(Vec::new()),
            usage_ready: AtomicBool::new(false),
            webview_ready: AtomicBool::new(false),
            usage_wake: tokio::sync::Notify::new(),
            online: AtomicBool::new(true),
            last_refresh_at: Mutex::new(None),
            auto_init_last_attempt_at: Mutex::new(None),
            auto_init_child: Mutex::new(None),
            manual_refresh_requested: AtomicBool::new(false),
            network_monitor: std::sync::Arc::new(AtomicBool::new(true)),
            network_shutdown: Mutex::new(None),
            coordinator: Mutex::new(automation::Coordinator::new(true, true)),
            monitor_network: AtomicBool::new(true),
            native_surface: native_surface::NativeSurfaceState::default(),
            pending_claude_login: Mutex::new(None),
            auth_accounts: Mutex::new(Vec::new()),
            auth_secrets: Mutex::new(auth::secret_store::MemoryStore::default()),
        }
    }
}

#[tauri::command]
fn query_history(
    state: tauri::State<'_, AppState>,
    query: history::HistoryQuery,
) -> Result<history::HistoryResult, String> {
    state
        .history
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(|| {
            state
                .history_error
                .lock()
                .ok()
                .and_then(|e| e.clone())
                .unwrap_or_else(|| "history unavailable".to_string())
        })?
        .query(query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_history(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.history.lock().map_err(|e| e.to_string())?;
    let history = guard.as_mut().ok_or_else(|| {
        state
            .history_error
            .lock()
            .ok()
            .and_then(|e| e.clone())
            .unwrap_or_else(|| "history unavailable".to_string())
    })?;
    clear_history_db(history)
}

fn clear_history_db(db: &mut history::HistoryDb) -> Result<(), String> {
    db.clear().map_err(|e| e.to_string())
}

#[tauri::command]
fn query_billing(
    state: tauri::State<'_, AppState>,
    query: history::HistoryQuery,
) -> Result<Vec<history::BillingAggregate>, String> {
    state
        .history
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(|| {
            state
                .history_error
                .lock()
                .ok()
                .and_then(|e| e.clone())
                .unwrap_or_else(|| "history unavailable".into())
        })?
        .aggregate_billing(query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn choose_history_export_path(
    app: tauri::AppHandle,
    format: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    if !matches!(format.as_str(), "json" | "csv") {
        return Err("invalid format".into());
    }
    Ok(app
        .dialog()
        .file()
        .add_filter("History export", &[format.as_str()])
        .blocking_save_file()
        .map(|p| p.to_string()))
}

#[tauri::command]
fn export_history(
    state: tauri::State<'_, AppState>,
    query: history::HistoryQuery,
    format: String,
    destination: String,
) -> Result<(), String> {
    let result = state
        .history
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .ok_or_else(|| "history unavailable".to_string())?
        .query(query.clone())
        .map_err(|e| e.to_string())?;
    export::write_export(
        std::path::Path::new(&destination),
        &format,
        &result,
        &query,
        chrono::Utc::now().timestamp(),
    )
}

#[tauri::command]
fn list_anthropic_accounts(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<auth::AccountSummary>, String> {
    state
        .auth_accounts
        .lock()
        .map(|v| v.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_manual_anthropic_credential(
    state: tauri::State<'_, AppState>,
    credential: String,
) -> Result<auth::AccountSummary, String> {
    let secret = auth::console::validate_manual_credential(&credential).map_err(str::to_string)?;
    let id = format!(
        "console:manual-{}",
        sha2::Sha256::digest(secret.as_bytes())
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let summary = auth::console::manual_summary(id, &secret);
    state
        .auth_secrets
        .lock()
        .map_err(|e| e.to_string())?
        .put(
            &auth::secret_store::target_name(auth::AccountKind::AnthropicConsole, &summary.id),
            secret,
        )
        .map_err(|_| "secure storage unavailable".to_string())?;
    state
        .auth_accounts
        .lock()
        .map_err(|e| e.to_string())?
        .push(summary.clone());
    Ok(summary)
}

#[tauri::command]
fn delete_anthropic_account(
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    state
        .auth_accounts
        .lock()
        .map_err(|e| e.to_string())?
        .retain(|a| a.id != account_id);
    Ok(())
}

/// Secret-store name for the claude.ai browser session key. Distinct from the Code CLI's OAuth
/// token: the extra-credit endpoints live on claude.ai and only accept the cookie.
fn claude_session_target() -> String {
    auth::secret_store::target_name(auth::AccountKind::ClaudeAi, "session")
}

/// Rejects obvious non-keys before storing, so a mistyped paste fails here with a clear message
/// rather than turning into a silent 401 on every poll for the next hour.
fn validated_session_key(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.len() < 20 || trimmed.contains(char::is_whitespace) {
        return Err("That does not look like a session key.".into());
    }
    Ok(trimmed.to_string())
}

#[tauri::command]
fn save_claude_session_key(
    state: tauri::State<'_, AppState>,
    session_key: String,
) -> Result<(), String> {
    let secret = validated_session_key(&session_key)?;
    state
        .auth_secrets
        .lock()
        .map_err(|e| e.to_string())?
        .put(&claude_session_target(), zeroize::Zeroizing::new(secret))
        .map_err(|_| "secure storage unavailable".to_string())
}

#[tauri::command]
fn clear_claude_session_key(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .auth_secrets
        .lock()
        .map_err(|e| e.to_string())?
        .delete(&claude_session_target())
        .map_err(|_| "secure storage unavailable".to_string())
}

/// Whether a key is stored — never the key itself, so the settings UI can show connected state
/// without the secret ever crossing the IPC boundary.
#[tauri::command]
fn has_claude_session_key(state: tauri::State<'_, AppState>) -> bool {
    state
        .auth_secrets
        .lock()
        .ok()
        .and_then(|store| store.get(&claude_session_target()).ok().flatten())
        .is_some()
}

#[tauri::command]
fn start_claude_ai_login(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let pkce = providers::claude::generate_pkce();
    let oauth_state = providers::claude::generate_state();
    let url = providers::claude::build_authorize_url(&pkce.challenge, &oauth_state);
    *state
        .pending_claude_login
        .lock()
        .map_err(|e| e.to_string())? = Some(PendingClaudeLogin {
        verifier: pkce.verifier,
        state: oauth_state,
    });
    let callback = "https://platform.claude.com/oauth/code/callback".to_string();
    tauri::WebviewWindowBuilder::new(
        &app,
        "claude-auth",
        tauri::WebviewUrl::External(url.parse().map_err(|_| "invalid login URL")?),
    )
    .title("Sign in to Claude.ai")
    .inner_size(500.0, 700.0)
    .resizable(true)
    .on_navigation(move |navigation| {
        matches!(
            auth::oauth::navigation_policy(navigation.as_str(), &callback),
            auth::oauth::NavigationDecision::Allow
        )
    })
    .build()
    .map_err(|e| e.to_string())?;
    Ok(url)
}
#[tauri::command]
fn cancel_claude_ai_login() -> Result<(), String> {
    Ok(())
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
    let previous = config::Config::load(&path).sanitized();
    shortcuts::replace(
        &app,
        &shortcuts::from_config(&previous),
        &shortcuts::from_config(&sanitized),
    )?;
    if let Err(error) = startup::set_registration(sanitized.launch_at_startup) {
        let _ = shortcuts::replace(
            &app,
            &shortcuts::from_config(&sanitized),
            &shortcuts::from_config(&previous),
        );
        return Err(error);
    }
    if let Err(error) = sanitized.save(&path) {
        let shortcut_rollback = shortcuts::replace(
            &app,
            &shortcuts::from_config(&sanitized),
            &shortcuts::from_config(&previous),
        );
        let startup_rollback = startup::set_registration(previous.launch_at_startup);
        let mut detail = format!("config save failed: {error}");
        if let Err(rollback) = shortcut_rollback {
            detail.push_str(&format!("; shortcut rollback failed: {rollback}"));
        }
        if let Err(rollback) = startup_rollback {
            detail.push_str(&format!("; startup rollback failed: {rollback}"));
        }
        return Err(detail);
    }
    let state = app.state::<AppState>();
    state
        .monitor_network
        .store(sanitized.monitor_network, Ordering::Release);
    state
        .network_monitor
        .store(sanitized.monitor_network, Ordering::Release);
    if let Ok(mut coordinator) = state.coordinator.lock() {
        coordinator.set_config(sanitized.refresh_on_wake, sanitized.monitor_network);
    }
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
fn set_tray_indicator(
    app: tauri::AppHandle,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > 256
        || height > 256
        || rgba.len() != width as usize * height as usize * 4
    {
        return Err("invalid tray image".into());
    }
    let tray = app
        .tray_by_id("usage")
        .ok_or_else(|| "tray unavailable".to_string())?;
    tray.set_icon(Some(tauri::image::Image::new_owned(rgba, width, height)))
        .map_err(|_| "tray update failed".into())
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

/// Maps a provider key to the CLI binary whose local login state this app is reading, so the
/// re-authenticate hint can launch the exact command that fixes it.
fn cli_binary_for_provider(provider: &str) -> Result<&'static str, String> {
    match provider {
        "claude" => Ok("claude"),
        "openai" => Ok("codex"),
        other => Err(format!("unknown provider: {other}")),
    }
}

#[tauri::command]
fn open_cli_terminal(provider: String) -> Result<(), String> {
    let binary = cli_binary_for_provider(&provider)?;
    #[cfg(target_os = "windows")]
    {
        // `start "" cmd /K <binary>` opens a new console window and leaves it open after the
        // CLI starts, so the user lands in its own sign-in/re-auth flow instead of a window
        // that closes the instant the command exits.
        std::process::Command::new("cmd")
            .args(["/C", "start", "", "cmd", "/K", binary])
            .spawn()
            .map_err(|error| error.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = binary;
    }
    Ok(())
}

/// `rundll32`, not `cmd /C start` — `cmd` re-parses its entire command line through its own
/// shell grammar, where `&` is a command separator. An OAuth authorize URL is nothing but
/// `&`-joined query parameters, so routing it through `cmd` silently truncated it at the first
/// `&` (dropping `client_id` and everything after). `rundll32` hands the URL to `ShellExecute`
/// as a single argv entry with no such re-parsing, so every character survives intact.
fn windows_open_url_command(url: &str) -> std::process::Command {
    let mut command = std::process::Command::new("rundll32");
    command.args(["url.dll,FileProtocolHandler", url]);
    command
}

fn open_url_in_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        windows_open_url_command(url)
            .spawn()
            .map_err(|error| error.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = url;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeAccountInfo {
    pub organization_uuid: Option<String>,
    /// Not present in the local `.credentials.json` — fetched live from the profile endpoint,
    /// so it's `None` whenever that request fails rather than blocking the signed-in state on it.
    pub email: Option<String>,
}

const CLAUDE_PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";

/// Reads the local credential file to establish sign-in state (unchanged from before), then
/// makes a best-effort live call to fetch the account's email — a failure there (offline, scope
/// rejected, endpoint hiccup) degrades to no email rather than to "not signed in".
async fn claude_account_info(
    client: &reqwest::Client,
    path: &std::path::Path,
    profile_url: &str,
) -> Option<ClaudeAccountInfo> {
    let contents = std::fs::read_to_string(path).ok()?;
    let credentials = creds::claude_oauth_from_str(&contents).ok()?;
    let email = match providers::fetch_response(
        client,
        profile_url,
        &credentials.access_token,
        &[("anthropic-beta", "oauth-2025-04-20")],
    )
    .await
    {
        Ok(response) => {
            let email = response
                .body
                .as_ref()
                .and_then(providers::claude::parse_profile_email);
            // `/api/oauth/profile` isn't an officially documented endpoint, so if it ever stops
            // returning `account.email` this is the only signal available to tell why — logged
            // rather than surfaced to the user, since a missing email degrades gracefully to the
            // org-id label.
            if email.is_none() {
                eprintln!(
                    "claude profile fetch: status {} had no account.email",
                    response.status
                );
            }
            email
        }
        Err(error) => {
            let _ = error;
            eprintln!("claude profile fetch failed before a response");
            None
        }
    };
    Some(ClaudeAccountInfo {
        organization_uuid: credentials.organization_uuid,
        email,
    })
}

/// Starts a browser-based "Sign in with Claude" attempt: generates a fresh PKCE pair and CSRF
/// state, remembers them in memory, and opens the real Anthropic consent screen. Nothing is
/// written to disk until `finish_claude_login` completes the exchange — a user who closes the
/// browser without finishing just leaves the pending attempt to be overwritten by the next one.
/// Always returns the authorize URL, even when opening the browser fails — the caller shows it
/// as a copyable fallback link, since a failed `spawn()` here isn't the only way navigation can
/// silently not happen (blocked by security software, wrong default browser, etc.).
#[tauri::command]
fn start_claude_login(
    app: tauri::AppHandle,
    app_state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let pkce = providers::claude::generate_pkce();
    let state = providers::claude::generate_state();
    let url = providers::claude::build_authorize_url(&pkce.challenge, &state);
    *app_state
        .pending_claude_login
        .lock()
        .map_err(|error| error.to_string())? = Some(PendingClaudeLogin {
        verifier: pkce.verifier,
        state,
    });
    if let Err(error) = open_url_in_browser(&url) {
        native_surface::report_diagnostic(&app, "claude-login-browser-open", &error);
    }
    Ok(url)
}

/// Completes the login with whatever `CODE#STATE` (or bare code) the user pasted back from the
/// consent screen. Consumes the pending attempt either way, so a failed exchange requires a
/// fresh `start_claude_login` rather than retrying a code that's already been spent.
#[tauri::command]
async fn finish_claude_login(
    app_state: tauri::State<'_, AppState>,
    pasted: String,
) -> Result<(), String> {
    let pending = app_state
        .pending_claude_login
        .lock()
        .map_err(|error| error.to_string())?
        .take()
        .ok_or_else(|| "No sign-in is in progress.".to_string())?;
    let (code, returned_state) = providers::claude::parse_pasted_code(&pasted);
    if code.is_empty() {
        return Err("That doesn't look like a valid code.".to_string());
    }
    if returned_state.is_some_and(|returned_state| returned_state != pending.state) {
        return Err("Sign-in state did not match — please try again.".to_string());
    }
    let client = reqwest::Client::new();
    let tokens = providers::claude::exchange_code_for_tokens(
        &client,
        providers::claude::LOGIN_TOKEN_URL,
        &code,
        &pending.state,
        &pending.verifier,
    )
    .await
    .map_err(|_| "Sign-in failed — the code may have expired.".to_string())?;
    creds::persist_claude_login(
        &claude_creds_path(),
        &tokens,
        unix_now().saturating_mul(1_000),
    )
    .map_err(|_| "Could not save the signed-in session.".to_string())?;
    // Otherwise the usage poller keeps following whatever backoff it had built up while signed
    // out (up to 5 minutes — see `retry_delay_seconds`) instead of reflecting the new session
    // right away.
    app_state.usage_wake.notify_one();
    Ok(())
}

#[tauri::command]
fn claude_logout(app_state: tauri::State<'_, AppState>) -> Result<(), String> {
    creds::logout_claude(&claude_creds_path()).map_err(|error| format!("{error:?}"))?;
    // Wakes the usage poller immediately so the signed-out state (and the cleared usage numbers
    // that come with it — see `poller::retain_last_good`) reaches the overlay right away instead
    // of lingering until the next scheduled poll.
    app_state.usage_wake.notify_one();
    Ok(())
}

#[tauri::command]
async fn get_claude_account() -> Option<ClaudeAccountInfo> {
    claude_account_info(
        &reqwest::Client::new(),
        &claude_creds_path(),
        CLAUDE_PROFILE_URL,
    )
    .await
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

fn unavailable_console_dashboard(now: i64) -> model::ConsoleCostsDashboard {
    let date = chrono::DateTime::from_timestamp(now, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    let start = date.date_naive().with_day(1).unwrap();
    let end = if start.month() == 12 {
        chrono::NaiveDate::from_ymd_opt(start.year() + 1, 1, 1).unwrap()
    } else {
        chrono::NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1).unwrap()
    };
    let section_money = |reason: &str| model::DataSection {
        value: None,
        fetched_at: now,
        state: model::DataSectionState::Unavailable,
        error_code: Some(reason.into()),
    };
    let section_points = |reason: &str| model::DataSection {
        value: None,
        fetched_at: now,
        state: model::DataSectionState::Unavailable,
        error_code: Some(reason.into()),
    };
    model::ConsoleCostsDashboard {
        period: model::CostPeriod {
            starts_at: format!("{start}T00:00:00Z"),
            ends_at: format!("{end}T00:00:00Z"),
            timezone: "UTC".into(),
        },
        spend: section_money("unsupportedBySource"),
        prepaid_balance: section_money("unsupportedBySource"),
        daily: section_points("unsupportedBySource"),
        by_api_key: section_points("unsupportedBySource"),
        by_model: section_points("unsupportedBySource"),
    }
}

#[tauri::command]
fn get_console_costs(
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<model::ConsoleCostsDashboard, String> {
    let valid = state
        .auth_accounts
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .any(|a| a.id == account_id && matches!(a.kind, auth::AccountKind::AnthropicConsole));
    if !valid {
        return Err("unknown Console account".into());
    }
    Ok(unavailable_console_dashboard(unix_now()))
}

#[tauri::command]
fn refresh_console_costs(
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<model::ConsoleCostsDashboard, String> {
    get_console_costs(state, account_id)
}

#[tauri::command]
fn select_console_account(
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    let valid = state
        .auth_accounts
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .any(|a| a.id == account_id && matches!(a.kind, auth::AccountKind::AnthropicConsole));
    if valid {
        Ok(())
    } else {
        Err("unknown Console account".into())
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

/// Shows and focuses the settings window, repairing its native surface before and after —
/// `show`/`set_focus` restore tao's default caption style through its window-flag diffing, so
/// the repair has to run again afterward or the window briefly shows a title bar it shouldn't
/// have. Shared by the tray menu's "Settings" item and the overlay's in-card sign-in hint, so
/// both take the exact same, already-hardened path onto screen.
fn show_settings_window(app: &tauri::AppHandle, page: Option<&str>) {
    let Some(window) = app.get_webview_window("settings") else {
        return;
    };
    if let Err(error) = repair_window_surface_ordered(app, "settings", false) {
        native_surface::report_diagnostic(app, "settings-repair", &error);
        if let Err(schedule_error) = schedule_deferred_surface_repair(app, "settings", false) {
            native_surface::report_diagnostic(app, "settings-repair-schedule", &schedule_error);
        }
        return;
    }
    if let Err(error) = window.show() {
        native_surface::report_diagnostic(app, "settings-show", &error.to_string());
        return;
    }
    if let Err(error) = window.set_focus() {
        native_surface::report_diagnostic(app, "settings-focus", &error.to_string());
    }
    if let Err(error) = repair_window_surface_ordered(app, "settings", false) {
        native_surface::report_diagnostic(app, "settings-repair", &error);
    }
    // The settings webview is a single persistent instance created at startup and only ever
    // shown/hidden, never reloaded — without this, an account signed in via a separate `claude`
    // CLI session (or any other config change) while the window stayed hidden would never show
    // up after reopening it.
    let _ = app.emit("settings-shown", page);
}

/// Positions the edge tab against the same screen corner the overlay is anchored to, so it
/// appears where the overlay just left rather than at a fixed corner.
fn place_edge_tab(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("edge-tab") else {
        return Ok(());
    };
    let config = config::Config::load(
        &app.path()
            .app_config_dir()
            .map_err(|e| e.to_string())?
            .join("config.json"),
    )
    .sanitized();
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let monitors: Vec<window::MonitorInfo> = window
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
    let chosen = window::choose_monitor(&monitors, config.monitor_id.as_deref())
        .ok_or_else(|| "no monitors available".to_string())?;
    let (x, y) = window::edge_tab_position(chosen.area, (size.width, size.height), &config.corner);
    window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

/// Applies whichever surface `visibility::overlay_surface` says should be showing. Both windows
/// are driven from that one decision so they can never both be visible, or both hidden while
/// the app believes it is on screen.
fn apply_overlay_surface(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
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
    let tucked = state.tucked.lock().map(|value| *value).unwrap_or(false);
    let surface = visibility::overlay_surface(
        active,
        state.webview_ready.load(Ordering::Acquire),
        manually_hidden,
        tucked,
    );

    if let Some(tab) = app.get_webview_window("edge-tab") {
        if surface == visibility::OverlaySurface::EdgeTab {
            place_edge_tab(app)?;
            tab.show().map_err(|e| e.to_string())?;
        } else {
            tab.hide().map_err(|e| e.to_string())?;
        }
    }
    if let Some(main) = app.get_webview_window("main") {
        if surface == visibility::OverlaySurface::Overlay {
            restore_overlay_surface_ordered(app, true)?;
            main.show().map_err(|e| e.to_string())?;
            // show() runs through tao's window-flag diffing, which restores the caption style.
            restore_overlay_surface_ordered(app, false)?;
        } else {
            main.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Tucks the overlay away to a single edge tab, or brings it back. Called from the tab itself
/// and from the overlay's tuck control.
#[tauri::command]
fn set_overlay_tucked(app: tauri::AppHandle, tucked: bool) -> Result<(), String> {
    *app.state::<AppState>()
        .tucked
        .lock()
        .map_err(|e| e.to_string())? = tucked;
    apply_overlay_surface(&app)
}

#[tauri::command]
fn is_overlay_tucked(state: tauri::State<'_, AppState>) -> bool {
    state.tucked.lock().map(|value| *value).unwrap_or(false)
}

#[tauri::command]
fn open_settings_window(app: tauri::AppHandle, page: Option<String>) {
    show_settings_window(&app, page.as_deref());
}

#[tauri::command]
fn open_history_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("history") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(&app, "history", tauri::WebviewUrl::App("index.html".into()))
        .title("Usage History")
        .inner_size(960.0, 680.0)
        .min_inner_size(760.0, 520.0)
        .resizable(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?
        .show()
        .map_err(|e| e.to_string())
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
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|_app, _shortcut, _event| {})
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            set_tray_indicator,
            close_settings,
            list_monitors,
            apply_overlay_geometry,
            get_bootstrap,
            mark_overlay_ready,
            set_morph_region,
            open_cli_terminal,
            start_claude_login,
            finish_claude_login,
            claude_logout,
            get_claude_account,
            list_anthropic_accounts,
            save_manual_anthropic_credential,
            delete_anthropic_account,
            get_console_costs,
            refresh_console_costs,
            select_console_account,
            start_claude_ai_login,
            cancel_claude_ai_login,
            set_overlay_tucked,
            is_overlay_tucked,
            save_claude_session_key,
            clear_claude_session_key,
            has_claude_session_key,
            open_settings_window,
            reset_notification_history,
            test_notification_sound,
            resize_popover,
            refresh_usage,
            get_runtime_status,
            open_history_window,
            query_history,
            query_billing,
            clear_history,
            export_history,
            choose_history_export_path
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
            if let Ok(data_dir) = app.path().app_data_dir() {
                if let Err(error) = std::fs::create_dir_all(&data_dir) {
                    *app.state::<AppState>()
                        .history_error
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = Some(error.to_string());
                    native_surface::report_diagnostic(
                        app.handle(),
                        "history-data-directory",
                        &error.to_string(),
                    );
                } else {
                    match history::HistoryDb::open(&data_dir.join("history.sqlite3")) {
                        Ok(mut db) => {
                            let cfg = app
                                .path()
                                .app_config_dir()
                                .map(|p| config::Config::load(&p.join("config.json")).sanitized())
                                .unwrap_or_default();
                            if let Err(error) = db.prune_retention_once(
                                chrono::Utc::now().timestamp(),
                                cfg.history_retention_days,
                            ) {
                                native_surface::report_diagnostic(
                                    app.handle(),
                                    "history-retention-startup",
                                    &error.to_string(),
                                );
                            }
                            *app.state::<AppState>()
                                .history
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = Some(db);
                        }
                        Err(error) => {
                            *app.state::<AppState>()
                                .history_error
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = Some(error.to_string());
                            native_surface::report_diagnostic(
                                app.handle(),
                                "history-open",
                                &error.to_string(),
                            );
                        }
                    }
                }
            } else if let Err(error) = app.path().app_data_dir() {
                *app.state::<AppState>()
                    .history_error
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(error.to_string());
            }
            if let Ok(log_directory) = app.path().app_log_dir() {
                app.state::<AppState>()
                    .native_surface
                    .initialize_diagnostic_writer(log_directory);
            }
            let launch_at_startup = app
                .path()
                .app_config_dir()
                .map(|p| config::Config::load(&p.join("config.json")))
                .unwrap_or_default()
                .sanitized()
                .launch_at_startup;
            if let Err(error) = startup::set_registration(launch_at_startup) {
                native_surface::report_diagnostic(app.handle(), "startup-registration", &error);
            }
            let initial = app
                .path()
                .app_config_dir()
                .map(|p| config::Config::load(&p.join("config.json")).sanitized())
                .unwrap_or_default();
            let mut host_system = sysinfo::System::new();
            let (host_names, host_pids) = detect::scan_processes(&mut host_system);
            let host_sources = detect::resolve(
                &host_names,
                detect::has_live_ide_lock(&claude_ide_dir(), &host_pids),
            );
            app.state::<AppState>()
                .monitor_network
                .store(initial.monitor_network, Ordering::Release);
            app.state::<AppState>()
                .network_monitor
                .store(initial.monitor_network, Ordering::Release);
            *app.state::<AppState>()
                .auto_init_last_attempt_at
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = initial.last_auto_init_at;
            if let Ok(mut coordinator) = app.state::<AppState>().coordinator.lock() {
                *coordinator =
                    automation::Coordinator::new(initial.refresh_on_wake, initial.monitor_network);
            }
            let (network_tx, mut network_rx) = tokio::sync::mpsc::channel(8);
            let monitor = app.state::<AppState>().network_monitor.clone();
            let (_network_probe, network_shutdown) = connectivity::start(
                if host_sources.claude {
                    "https://api.anthropic.com".into()
                } else {
                    "https://api.openai.com".into()
                },
                monitor,
                network_tx,
            );
            *app.state::<AppState>()
                .network_shutdown
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(network_shutdown);
            let network_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = network_rx.recv().await {
                    route_automation_event(
                        &network_handle,
                        match event {
                            connectivity::SystemEvent::NetworkOnline => {
                                automation::Event::NetworkOnline
                            }
                            connectivity::SystemEvent::NetworkOffline => {
                                automation::Event::NetworkOffline
                            }
                        },
                    );
                }
            });
            let (power_tx, power_rx) = std::sync::mpsc::channel();
            let _power_observer = power::start(power_tx);
            let power_handle = app.handle().clone();
            std::thread::spawn(move || {
                while power_rx.recv().is_ok() {
                    route_automation_event(&power_handle, automation::Event::Wake);
                }
            });
            let shortcut_config = initial.clone();
            if let Err(error) =
                shortcuts::register_all(app.handle(), &shortcuts::from_config(&shortcut_config))
            {
                native_surface::report_diagnostic(app.handle(), "shortcut-registration", &error);
            }
            let toggle_handle = app.handle().clone();
            let _ = app.listen("shortcut-toggle-popover", move |_| {
                toggle_overlay_visibility(&toggle_handle)
            });
            let settings_handle = app.handle().clone();
            let _ = app.listen("shortcut-open-settings", move |_| {
                show_settings_window(&settings_handle, None)
            });
            repair_windows_on_startup(app.handle());

            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let toggle = MenuItem::with_id(app, "toggle", "Show/Hide", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;
            TrayIconBuilder::with_id("usage")
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
                    "settings" => show_settings_window(app, None),
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
                // Every scan goes through the hold, so a provider whose process briefly drops
                // out of the list keeps its card on screen instead of the window being hidden
                // and reshown a second later. The startup scan seeds it so an already-running
                // provider is held from the first tick, not only from the second.
                let mut hold = detect::SourceHold::new(detect::SOURCE_GRACE_TICKS);
                hold.observe(initial_sources);
                loop {
                    let (names, pids) = detect::scan_processes(&mut system);
                    let active = hold.observe(detect::resolve(
                        &names,
                        detect::has_live_ide_lock(&claude_ide_dir(), &pids),
                    ));
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
                let mut process_system = sysinfo::System::new();
                loop {
                    let sources = usage_handle
                        .state::<AppState>()
                        .sources
                        .lock()
                        .map(|value| *value)
                        .unwrap_or_default();
                    let runtime_config = usage_handle
                        .path()
                        .app_config_dir()
                        .map(|p| config::Config::load(&p.join("config.json")).sanitized())
                        .unwrap_or_default();
                    let credentials_available = (sources.claude && claude_creds_path().is_file())
                        || (sources.openai && codex_auth_path().is_file());
                    let _ = detect::scan_processes(&mut process_system);
                    let child_live = {
                        let app_state = usage_handle.state::<AppState>();
                        let mut child = app_state
                            .auto_init_child
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        match child.as_mut() {
                            Some(process) => match process.try_wait() {
                                Ok(Some(_)) => {
                                    *child = None;
                                    false
                                }
                                Ok(None) => true,
                                Err(_) => {
                                    *child = None;
                                    false
                                }
                            },
                            None => false,
                        }
                    };
                    let required = match runtime_config.auto_init_task_kind.as_str() {
                        "reasoning" => session_init::Capability::Reasoning,
                        _ => session_init::Capability::Standard,
                    };
                    if runtime_config.auto_initialize_session {
                        if let Some(model) =
                            session_init::choose_model(required, session_init::MODELS)
                        {
                            let context = session_init::InitContext {
                                enabled: runtime_config.auto_initialize_session,
                                acknowledged: runtime_config.auto_init_cost_warning_accepted,
                                provider_active: sources.claude || sources.openai,
                                credentials_available,
                                child_or_session_live: child_live,
                                now: unix_now(),
                                last_attempt: runtime_config.last_auto_init_at,
                            };
                            let (decision, timestamp, child) =
                                session_init::maybe_initialize_with_child(
                                    &context,
                                    model,
                                    session_init::spawn_session,
                                );
                            if let Some(child) = child {
                                *usage_handle
                                    .state::<AppState>()
                                    .auto_init_child
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner()) = Some(child);
                            }
                            if matches!(
                                decision,
                                session_init::InitDecision::Started
                                    | session_init::InitDecision::Failed
                            ) {
                                let mut next = runtime_config.clone();
                                next.last_auto_init_at = timestamp;
                                let persisted = next.save(
                                    &usage_handle
                                        .path()
                                        .app_config_dir()
                                        .unwrap_or_default()
                                        .join("config.json"),
                                );
                                if persisted.is_ok() {
                                    *usage_handle
                                        .state::<AppState>()
                                        .auto_init_last_attempt_at
                                        .lock()
                                        .unwrap_or_else(|e| e.into_inner()) = timestamp;
                                    let _ = usage_handle.emit(
                                        "runtime-status-changed",
                                        runtime_status(usage_handle.clone()),
                                    );
                                } else {
                                    native_surface::report_diagnostic(
                                        &usage_handle,
                                        "auto-init-persistence",
                                        "could not persist last automatic initialization attempt",
                                    );
                                }
                            }
                        }
                    }
                    let visible = usage_handle
                        .get_webview_window("main")
                        .and_then(|window| window.is_visible().ok())
                        .unwrap_or(false);
                    let online = usage_handle
                        .state::<AppState>()
                        .online
                        .load(Ordering::Acquire);
                    let poll_action = usage_handle
                        .state::<AppState>()
                        .coordinator
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .on_event(automation::Event::PollDue);
                    let manual_refresh = usage_handle
                        .state::<AppState>()
                        .manual_refresh_requested
                        .swap(false, Ordering::AcqRel);
                    if (manual_refresh || online)
                        && (manual_refresh || poll_action != automation::Action::Wait)
                        && (manual_refresh
                            || (first)
                            || visible
                            || !usage_handle
                                .state::<AppState>()
                                .usage_ready
                                .load(Ordering::Acquire))
                    {
                        let cycle = fetch_usage_cycle(
                            &client,
                            sources,
                            last_claude.as_ref(),
                            last_codex.as_ref(),
                            failures,
                            usage_handle
                                .state::<AppState>()
                                .auth_secrets
                                .lock()
                                .ok()
                                .and_then(|s| {
                                    s.values
                                        .iter()
                                        .find(|(name, _)| name.contains("/claude-ai/"))
                                        .map(|(_, v)| v.to_string())
                                }),
                            claude_web_credential(&usage_handle),
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
                        *usage_state
                            .last_refresh_at
                            .lock()
                            .unwrap_or_else(|e| e.into_inner()) = Some(unix_now());
                        let _ = usage_handle.emit(
                            "runtime-status-changed",
                            runtime_status(usage_handle.clone()),
                        );
                        drop(source_guard);
                        let _ = reconcile_overlay_visibility(&usage_handle);
                    }
                    first = false;
                    let app_state = usage_handle.state::<AppState>();
                    // A failing provider is retried sooner than the steady-state interval so a
                    // transient blip clears in seconds rather than lingering for a full minute.
                    let configured = usage_handle
                        .path()
                        .app_config_dir()
                        .map(|p| config::Config::load(&p.join("config.json")))
                        .unwrap_or_default()
                        .sanitized()
                        .poll_interval_sec;
                    let delay = poller::retry_delay_seconds(
                        failures.claude.max(failures.openai),
                        configured,
                    );
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

pub fn toggle_overlay_visibility(app: &tauri::AppHandle) {
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
    // While tucked, the main window stays hidden and the edge tab is the visible surface, so
    // the whole transition is delegated rather than re-deciding it here — otherwise a routine
    // reconcile (a detection tick, a provider appearing) would pop the cards back out from
    // under the tab.
    if state.tucked.lock().map(|value| *value).unwrap_or(false) {
        return apply_overlay_surface(app);
    }
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
    let billing = history_billing_from_events(&events);
    if let Ok(mut history) = state.history.lock() {
        if let Some(db) = history.as_mut() {
            match db.record_poll_cycle(&events, &billing) {
                Err(error) => {
                    native_surface::report_diagnostic(app, "history-record", &error.to_string())
                }
                Ok(inserted) if inserted > 0 => {
                    let days = app
                        .path()
                        .app_config_dir()
                        .map(|p| {
                            config::Config::load(&p.join("config.json"))
                                .sanitized()
                                .history_retention_days
                        })
                        .unwrap_or(180);
                    if let Err(error) =
                        db.prune_retention_once(chrono::Utc::now().timestamp(), days)
                    {
                        native_surface::report_diagnostic(
                            app,
                            "history-retention-insert",
                            &error.to_string(),
                        );
                    }
                }
                Ok(_) => {}
            }
        }
    }
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

fn history_billing_from_events(
    _events: &[model::ProviderUsageEvent],
) -> Vec<history::BillingSample> {
    // Claude Extra.spend is cumulative and has no verified billing-period contract. Never turn
    // it into synthetic periods or deltas that cannot survive restart.
    Vec::new()
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

/// Merges a freshly fetched extra-credit section into the Claude snapshot's details, leaving
/// every other detail section (model limits, service status) exactly as it was.
/// Pairs the org id from the local Claude credentials with the stored claude.ai session key.
/// Both are required — the endpoints are org-scoped and cookie-authenticated — so a missing
/// either means no extra-credit request is made at all.
fn claude_web_credential(app: &tauri::AppHandle) -> Option<(String, String)> {
    let contents = std::fs::read_to_string(claude_creds_path()).ok()?;
    let organization_uuid = creds::claude_oauth_from_str(&contents)
        .ok()?
        .organization_uuid?;
    let session_key = app
        .state::<AppState>()
        .auth_secrets
        .lock()
        .ok()?
        .get(&claude_session_target())
        .ok()
        .flatten()?
        .to_string();
    Some((organization_uuid, session_key))
}

async fn attach_extra_credit(
    client: &reqwest::Client,
    mut snapshot: model::UsageSnapshot,
    organization_uuid: &str,
    session_key: &str,
    now: i64,
) -> model::UsageSnapshot {
    let extra = providers::claude_overage::fetch_extra(
        client,
        providers::claude_overage::CLAUDE_WEB_ORIGIN,
        organization_uuid,
        session_key,
        now,
    )
    .await;
    let details = match snapshot.details.take() {
        Some(model::ProviderDetails::Claude(mut details)) => {
            details.extra = extra;
            details
        }
        None => model::ClaudeUsageDetails {
            limits: providers::claude_usage::unavailable_limits(now),
            extra,
            status: None,
        },
    };
    snapshot.details = Some(model::ProviderDetails::Claude(details));
    snapshot
}

async fn fetch_usage_cycle(
    client: &reqwest::Client,
    sources: detect::ActiveSources,
    last_claude: Option<&model::UsageSnapshot>,
    last_codex: Option<&model::UsageSnapshot>,
    failures: ProviderFailures,
    resolved_claude_token: Option<String>,
    claude_web_credential: Option<(String, String)>,
) -> UsageCycle {
    let now = unix_now();
    let claude = async {
        if !sources.claude {
            return (None, 0, None);
        }
        let token_result = if let Some(token) = resolved_claude_token {
            Ok(token)
        } else {
            claude_access_token(client, &claude_creds_path(), now).await
        };
        let (snapshot, diagnostic) = match token_result {
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
                    claude_desktop_fallback(&claude_desktop_usage_path(), now).unwrap_or_else(
                        || {
                            poller::retain_last_good(
                                last_claude,
                                now,
                                poller::state_for_failed_refresh(
                                    last_claude,
                                    next_failure_count(failures.claude, false),
                                    providers::state_for_error(&error),
                                ),
                            )
                        },
                    ),
                    Some(("usage-fetch-claude", "transport failure".to_string())),
                ),
            },
            Err(error) => (
                claude_desktop_fallback(&claude_desktop_usage_path(), now)
                    .unwrap_or_else(|| claude_snapshot_for_error(last_claude, now, error)),
                Some(("usage-fetch-claude", "token unavailable".to_string())),
            ),
        };
        let succeeded = snapshot.state == model::SnapshotState::Fresh;
        // Extra credit lives on claude.ai behind the session cookie, so it is a separate
        // request from the usage fetch above and is allowed to fail on its own: no key, an
        // expired one, or a bot challenge all leave the section unavailable and the usage
        // windows untouched.
        let snapshot = match claude_web_credential {
            Some((organization_uuid, session_key)) => {
                attach_extra_credit(client, snapshot, &organization_uuid, &session_key, now).await
            }
            None => snapshot,
        };
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
            Err(error) => (
                codex_snapshot_for_token_error(last_codex, now, error),
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

/// An absent `~/.codex/auth.json` means `codex login` has never run on this machine; any other
/// read failure means credentials exist but cannot be used. Collapsing both into `Error` would
/// tell a first-time user to re-authenticate an account they never connected.
fn codex_snapshot_for_token_error(
    last: Option<&model::UsageSnapshot>,
    now: i64,
    error: creds::TokenError,
) -> model::UsageSnapshot {
    let state = match error {
        creds::TokenError::NotFound => model::SnapshotState::SignedOut,
        creds::TokenError::Unreadable | creds::TokenError::Malformed => model::SnapshotState::Error,
    };
    poller::retain_last_good(last, now, state)
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
    // Checked before reading so a fresh install — where this file has never existed — is told to
    // sign in rather than to re-authenticate credentials it does not have.
    if !path.exists() {
        return Err(providers::FetchError::SignedOut);
    }
    let contents =
        std::fs::read_to_string(path).map_err(|_| providers::FetchError::Unauthorized)?;
    let credentials = creds::claude_oauth_from_str(&contents).map_err(|error| match error {
        // A file that exists but has no `claudeAiOauth` key at all — e.g. right after logging
        // out through this app — is "never (or no longer) signed in", not a rejected token.
        creds::TokenError::NotFound => providers::FetchError::SignedOut,
        creds::TokenError::Unreadable | creds::TokenError::Malformed => {
            providers::FetchError::Unauthorized
        }
    })?;
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
fn claude_desktop_usage_path() -> std::path::PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| {
            dirs.data_dir()
                .join("Claude")
                .join("plan-usage-history.json")
        })
        .unwrap_or_default()
}

/// Falls back to the Claude Desktop app's own local usage cache when the Code CLI's
/// OAuth-backed fetch can't produce a result — either because `.credentials.json` doesn't exist
/// at all (a desktop-only user has never run `claude`) or because the live request failed. The
/// desktop app writes this file on its own schedule, so a value read from it is inherently a
/// little behind, hence `Stale` rather than `Fresh`.
fn claude_desktop_fallback(path: &std::path::Path, now: i64) -> Option<model::UsageSnapshot> {
    let value = providers::claude::read_desktop_usage_history(path)?;
    providers::claude::parse_desktop_usage_history(&value, now, model::SnapshotState::Stale)
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

    #[tokio::test]
    async fn a_user_who_never_signed_in_is_reported_as_signed_out_not_unauthorized() {
        // A fresh install has no ~/.claude/.credentials.json at all. That is the single most
        // likely first-run state for anyone downloading this, and it needs its own copy.
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            claude_access_token(
                &reqwest::Client::new(),
                &directory.path().join(".credentials.json"),
                0,
            )
            .await
            .unwrap_err(),
            providers::FetchError::SignedOut
        );
    }

    #[tokio::test]
    async fn a_present_but_unusable_credential_file_is_not_mistaken_for_being_signed_out() {
        // The file existing means the user did sign in at some point; a corrupt or incomplete
        // file is a re-authentication problem, not a "you have never signed in" problem.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".credentials.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(
            claude_access_token(&reqwest::Client::new(), &path, 0)
                .await
                .unwrap_err(),
            providers::FetchError::Unauthorized
        );
    }

    #[test]
    fn cli_binary_for_provider_maps_each_provider_to_its_own_cli() {
        assert_eq!(cli_binary_for_provider("claude"), Ok("claude"));
        assert_eq!(cli_binary_for_provider("openai"), Ok("codex"));
    }

    #[test]
    fn cli_binary_for_provider_rejects_an_unknown_provider() {
        assert!(cli_binary_for_provider("bogus").is_err());
    }

    #[test]
    fn a_desktop_only_user_gets_stale_desktop_usage_instead_of_nothing() {
        // No Code CLI credentials on this machine at all, but the desktop app's own cache has
        // usage in it: that's the exact case this fallback exists for.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("plan-usage-history.json");
        std::fs::write(
            &path,
            r#"{"version":2,"samples":[{"t":1,"org":"x","u":{"fh":11,"sd":40}}]}"#,
        )
        .unwrap();
        let snapshot = claude_desktop_fallback(&path, 100).unwrap();
        assert_eq!(snapshot.state, model::SnapshotState::Stale);
        assert_eq!(snapshot.windows[0].used_percent, 11.0);
    }

    #[test]
    fn opens_the_url_via_rundll32_as_its_own_argument_rather_than_through_a_shell() {
        // rundll32 receives argv directly with no textual re-parsing, so an authorize URL full
        // of `&`-joined query parameters can't be truncated the way `cmd /C start` truncated it.
        let url = "https://claude.ai/oauth/authorize?code=true&client_id=abc&state=xyz";
        let command = windows_open_url_command(url);
        assert_eq!(command.get_program(), "rundll32");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, ["url.dll,FileProtocolHandler", url]);
    }

    #[tokio::test]
    async fn claude_account_info_includes_the_email_the_profile_endpoint_reports() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".credentials.json");
        std::fs::write(&path, r#"{"claudeAiOauth":{"accessToken":"tok-1"}}"#).unwrap();
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/oauth/profile")
            .match_header("anthropic-beta", "oauth-2025-04-20")
            .with_status(200)
            .with_body(r#"{"account":{"email":"person@example.com"}}"#)
            .create_async()
            .await;

        let account = claude_account_info(
            &reqwest::Client::new(),
            &path,
            &format!("{}/api/oauth/profile", server.url()),
        )
        .await
        .expect("credentials exist, so this is signed in");

        assert_eq!(account.email.as_deref(), Some("person@example.com"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn claude_account_info_still_reports_signed_in_when_the_profile_request_fails() {
        // The local credential file is the source of truth for "signed in" — a broken or
        // unreachable profile endpoint should only cost the email, not the whole account.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(".credentials.json");
        std::fs::write(
            &path,
            r#"{"claudeAiOauth":{"accessToken":"tok-1"},"organizationUuid":"org-1"}"#,
        )
        .unwrap();
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/api/oauth/profile")
            .with_status(500)
            .create_async()
            .await;

        let account = claude_account_info(
            &reqwest::Client::new(),
            &path,
            &format!("{}/api/oauth/profile", server.url()),
        )
        .await
        .expect("credentials exist, so this is signed in");

        assert_eq!(account.email, None);
        assert_eq!(account.organization_uuid.as_deref(), Some("org-1"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn claude_account_info_is_nothing_when_never_signed_in() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.json");
        assert!(
            claude_account_info(&reqwest::Client::new(), &missing, "http://unused")
                .await
                .is_none()
        );
    }

    #[test]
    fn claude_desktop_fallback_reports_nothing_when_the_cache_file_is_absent() {
        let directory = tempfile::tempdir().unwrap();
        assert!(claude_desktop_fallback(&directory.path().join("missing.json"), 100).is_none());
    }

    #[test]
    fn a_missing_codex_auth_file_reports_signed_out_while_a_broken_one_reports_an_error() {
        assert_eq!(
            codex_snapshot_for_token_error(None, 100, creds::TokenError::NotFound).state,
            model::SnapshotState::SignedOut
        );
        assert_eq!(
            codex_snapshot_for_token_error(None, 100, creds::TokenError::Malformed).state,
            model::SnapshotState::Error
        );
    }

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
            headers: std::collections::BTreeMap::new(),
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
                headers: std::collections::BTreeMap::new(),
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
                pace: None,
            }],
            fetched_at: 100,
            state: model::SnapshotState::Fresh,
            details: None,
        };

        let snapshot = codex_snapshot_from_response(
            providers::FetchResponse {
                status: 503,
                body: None,
                headers: std::collections::BTreeMap::new(),
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
                headers: std::collections::BTreeMap::new(),
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
                headers: std::collections::BTreeMap::new(),
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
                    pace: None,
                },
                model::UsageWindow {
                    label: "Weekly".into(),
                    used_percent: 18.0,
                    resets_at: 5_678,
                    pace: None,
                },
            ],
            fetched_at: 100,
            state: model::SnapshotState::Fresh,
            details: None,
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

    #[test]
    fn cumulative_claude_spend_is_not_invented_or_persisted() {
        let make = |at, amount| model::ProviderUsageEvent {
            provider: model::Provider::Claude,
            snapshot: model::UsageSnapshot {
                windows: vec![],
                fetched_at: at,
                state: model::SnapshotState::Fresh,
                details: Some(model::ProviderDetails::Claude(model::ClaudeUsageDetails {
                    limits: model::DataSection {
                        value: Some(vec![]),
                        fetched_at: at,
                        state: model::DataSectionState::Fresh,
                        error_code: None,
                    },
                    extra: model::DataSection {
                        value: Some(model::ClaudeExtra {
                            spend: Some(model::Money {
                                minor_units: amount,
                                currency: "USD".into(),
                            }),
                            ..Default::default()
                        }),
                        fetched_at: at,
                        state: model::DataSectionState::Fresh,
                        error_code: None,
                    },
                    status: None,
                })),
            },
        };
        let event = make(1, 100);
        assert!(history_billing_from_events(std::slice::from_ref(&event)).is_empty());
        let mut db = history::HistoryDb::open_in_memory().unwrap();
        db.record_poll_cycle(
            std::slice::from_ref(&event),
            &history_billing_from_events(std::slice::from_ref(&event)),
        )
        .unwrap();
        assert_eq!(
            db.connection()
                .query_row("SELECT count(*) FROM billing_entries", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn clear_command_helper_propagates_database_errors() {
        let mut db = history::HistoryDb::open_in_memory().unwrap();
        db.connection().execute("INSERT INTO usage_samples(provider,window_kind,used_percent,resets_at,sampled_at) VALUES ('claude','session_5h',1,0,1)", []).unwrap();
        db.connection().execute("INSERT INTO billing_entries(provider,period_start,period_end,amount_micros,currency,source) VALUES ('claude',0,1,1,'USD','provider')", []).unwrap();
        db.connection().execute("CREATE TRIGGER reject_clear BEFORE DELETE ON billing_entries BEGIN SELECT RAISE(ABORT, 'reject'); END", []).unwrap();
        assert!(clear_history_db(&mut db).is_err());
        assert_eq!(
            db.connection()
                .query_row("SELECT count(*) FROM usage_samples", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.connection()
                .query_row("SELECT count(*) FROM billing_entries", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
