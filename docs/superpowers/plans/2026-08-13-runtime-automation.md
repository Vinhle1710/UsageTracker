# Runtime Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add opt-in session initialization, wake/network-aware refresh, configurable short polling, launch-at-login, and global popover/refresh/settings shortcuts.

**Architecture:** A Rust automation coordinator consumes explicit system events and emits idempotent actions into the existing poll wake path. Automatic CLI session initialization is off by default and requires a stored acknowledgement of its cost warning; model selection uses a fixed suitability catalog and executes argv without a shell. Windows power/network adapters only signal the coordinator, while Tauri's global-shortcut plugin owns shortcut registration.

**Tech Stack:** Tauri 2, tauri-plugin-global-shortcut 2, Rust/Tokio, Windows system APIs, serde, React + TypeScript, Vitest

---

## Prerequisites and file map

- Branch after merging `codex/feature-react-rendering-foundation` and `codex/feature-popover-i18n`; global shortcut actions reuse the popover window controller instead of creating a competing window path. Retain the existing `startup.rs` Windows HKCU registration instead of adding a second autostart mechanism.
- Add `tauri-plugin-global-shortcut = "2"` and initialize its builder. Add required `windows-sys` features for power-setting and network-list notifications only after compiling the exact adapters.
- Create `automation.rs` (pure policy), `session_init.rs` (safe process execution), `connectivity.rs` and `power.rs` (Windows event sources), `shortcuts.rs` (registration/action routing).
- Modify `config.rs`, `poller.rs`, `lib.rs`, `startup.rs`, settings React UI/types/tests, and capabilities only if JS plugin APIs are used. Prefer Rust plugin registration so no global-shortcut JS permission is exposed.

### Task 1: Extend configuration with safe automation defaults

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src/types.ts`
- Modify: `src/components/settings.tsx`
- Modify: `src/components/settings.test.tsx`

- [ ] **Step 1: Write failing defaults/sanitization tests**

```rust
#[test] fn automation_is_off_by_default() { let c=Config::default(); assert!(!c.auto_initialize_session); assert!(!c.auto_init_cost_warning_accepted); assert_eq!(c.poll_interval_sec,60); }
#[test] fn short_polling_is_bounded() { assert_eq!(Config{poll_interval_sec:2,..Default::default()}.sanitized().poll_interval_sec,15); }
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config::tests::automation_is_off_by_default -- --exact`
Expected: FAIL because new fields do not exist.

- [ ] **Step 3: Add exact fields and defaults**

```rust
#[serde(default)] pub auto_initialize_session: bool,
#[serde(default)] pub auto_init_cost_warning_accepted: bool,
#[serde(default="default_model_task")] pub auto_init_task_kind: String,
#[serde(default)] pub refresh_on_wake: bool,
#[serde(default="default_true")] pub monitor_network: bool,
#[serde(default)] pub shortcut_popover: Option<String>,
#[serde(default)] pub shortcut_refresh: Option<String>,
#[serde(default)] pub shortcut_settings: Option<String>,
fn default_model_task()->String{"light".into()}
```

Set `refresh_on_wake=true` in `Default`; sanitize task kind to `light|standard|reasoning`, polling to `15..=3600`, trim empty shortcuts to `None`, and force `auto_initialize_session=false` whenever warning acceptance is false. Settings describe polling load, launch-at-login, wake/network behavior and shortcut conflicts. Enabling auto-init opens a modal whose checkbox is “I understand this can start a paid API/CLI session”; only confirmation writes both booleans true. Disabling clears only `auto_initialize_session`, keeping acknowledgement auditable.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config::tests && npm test -- --run src/components/settings.test.tsx`
Expected: PASS, including old-config migration.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src/types.ts src/components/settings.tsx src/components/settings.test.tsx
git commit -m "feat(automation): add safe runtime settings"
```

### Task 2: Build a cheapest-suitable model policy without remote model discovery

**Files:**
- Create: `src-tauri/src/session_init.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing policy tests**

```rust
#[test] fn chooses_cheapest_suitable_enabled_model() { let c=fixture_catalog(); assert_eq!(choose_model(TaskKind::Standard,&c).unwrap().id,"gpt-5.6-terra"); }
#[test] fn no_model_means_no_process() { assert_eq!(choose_model(TaskKind::Reasoning,&[]),None); }
#[test] fn argv_never_uses_a_shell() { assert_eq!(session_command("gpt-5.6-terra"), CommandSpec{program:"codex".into(),args:vec!["exec".into(),"--model".into(),"gpt-5.6-terra".into(),"--".into(),"Initialize a usage-tracking session and wait.".into()]}); }
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml session_init::tests -- --nocapture`
Expected: FAIL because module is missing.

- [ ] **Step 3: Implement explicit catalog and safe spawn**

```rust
#[derive(Clone,Copy,PartialEq,Eq,PartialOrd,Ord)] pub enum Capability{Light,Standard,Reasoning}
pub struct ModelChoice{pub id:&'static str,pub capability:Capability,pub relative_cost:u16}
pub const MODELS:&[ModelChoice]=&[
 ModelChoice{id:"gpt-5.6-terra",capability:Capability::Standard,relative_cost:10},
 ModelChoice{id:"gpt-5.6-sol",capability:Capability::Reasoning,relative_cost:20},
];
pub fn choose_model(required:Capability, models:&[ModelChoice])->Option<&ModelChoice>{models.iter().filter(|m|m.capability>=required).min_by_key(|m|m.relative_cost)}
```

Map `light` and `standard` to Standard until a cheaper verified model is explicitly added. Spawn with `std::process::Command::new(spec.program).args(spec.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).creation_flags(CREATE_NO_WINDOW)`; never `cmd /C`. `maybe_initialize` returns early unless both opt-in flags are true, a provider process is active, credentials exist, and no initialization child/live session is known. Persist only `last_auto_init_at` and apply a 30-minute cooldown; do not store prompts or credentials. Failure is diagnostic and never retried more often than cooldown.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml session_init::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/session_init.rs src-tauri/src/lib.rs
git commit -m "feat(automation): initialize opted-in sessions safely"
```

### Task 3: Centralize refresh reasons and configurable polling

**Files:**
- Create: `src-tauri/src/automation.rs`
- Modify: `src-tauri/src/poller.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing coordinator tests**

```rust
#[test] fn online_and_wake_request_one_refresh_each() { let mut c=Coordinator::new(true,true); assert_eq!(c.on_event(Event::NetworkOnline),Action::WakePoller); assert_eq!(c.on_event(Event::NetworkOnline),Action::None); assert_eq!(c.on_event(Event::Wake),Action::WakePoller); }
#[test] fn offline_suppresses_scheduled_fetch() { let mut c=Coordinator::new(true,true); c.on_event(Event::NetworkOffline); assert_eq!(c.on_event(Event::PollDue),Action::Wait); }
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml automation::tests -- --nocapture`
Expected: FAIL because coordinator is absent.

- [ ] **Step 3: Implement coordinator and replace hard-coded delay**

```rust
pub enum Event{PollDue,ManualRefresh,Wake,NetworkOnline,NetworkOffline}
#[derive(PartialEq,Eq)] pub enum Action{WakePoller,FetchNow,Wait,None}
pub struct Coordinator{online:bool,refresh_on_wake:bool,monitor_network:bool}
```

`NetworkOnline` wakes only on offline→online; `Wake` wakes only when enabled; manual refresh always fetches (and reports offline failure); scheduled polling waits offline. Change `retry_delay_seconds(failures, configured)` to return configured for 0/1 failures, `max(configured,120)` for 2, and `max(configured,300)` thereafter. Read sanitized config every loop rather than caching startup values. Add `refresh_usage` command that calls `usage_wake.notify_one()`.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml automation::tests poller::tests`
Expected: PASS; 15-second configured polling remains 15 only on success and backs off on errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/automation.rs src-tauri/src/poller.rs src-tauri/src/lib.rs
git commit -m "feat(automation): coordinate refresh triggers"
```

### Task 4: Add Windows wake and network event adapters

**Files:**
- Create: `src-tauri/src/power.rs`
- Create: `src-tauri/src/connectivity.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing adapter-boundary tests**

```rust
#[test] fn resume_status_maps_to_wake() { assert_eq!(map_power_status(PBT_APMRESUMEAUTOMATIC),Some(SystemEvent::Wake)); }
#[test] fn connectivity_is_edge_triggered() { let mut s=OnlineState::new(false); assert_eq!(s.update(true),Some(SystemEvent::NetworkOnline)); assert_eq!(s.update(true),None); }
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml power::tests connectivity::tests`
Expected: FAIL because adapters do not exist.

- [ ] **Step 3: Implement lifecycle-safe Windows sources**

Use the Tauri run/window event hook to map Windows resume (`PBT_APMRESUMEAUTOMATIC`, `PBT_APMRESUMESUSPEND`) into `Event::Wake`. Register network availability using Windows Network List Manager COM on a dedicated initialized thread; its callback sends booleans through a Tokio mpsc channel, and a drop guard unadvises the connection point and calls `CoUninitialize`. Non-Windows modules expose no-op starters. If COM subscription fails, fall back to a 5-second `reqwest` reachability probe against the already configured provider host, edge-triggered and paused when monitoring is disabled. Neither adapter performs usage HTTP calls directly.

- [ ] **Step 4: Verify green and Windows compile**

Run: `cargo test --manifest-path src-tauri/Cargo.toml power::tests connectivity::tests && cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc`
Expected: tests PASS and Windows target compiles.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/power.rs src-tauri/src/connectivity.rs src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs
git commit -m "feat(automation): refresh on wake and reconnect"
```

### Task 5: Register global shortcuts transactionally

**Files:**
- Create: `src-tauri/src/shortcuts.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/config.rs`

- [ ] **Step 1: Write failing validation tests**

```rust
#[test] fn rejects_duplicates_before_registration() { let s=ShortcutConfig{popover:Some("Ctrl+Shift+U".into()),refresh:Some("Ctrl+Shift+U".into()),settings:None}; assert_eq!(validate(&s),Err(ShortcutError::Duplicate("Ctrl+Shift+U".into()))); }
#[test] fn maps_actions_exactly() { assert_eq!(action_for(ShortcutSlot::Popover),ShortcutAction::TogglePopover); assert_eq!(action_for(ShortcutSlot::Refresh),ShortcutAction::Refresh); }
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shortcuts::tests -- --nocapture`
Expected: FAIL because module is absent.

- [ ] **Step 3: Implement plugin and rollback-safe replacement**

Initialize `tauri_plugin_global_shortcut::Builder::new().with_handler(...)`. On startup and `set_config`, parse each optional shortcut using plugin `Shortcut` parsing. Register the complete new set before unregistering old entries where possible; if any registration fails, unregister newly added entries and keep the old set/config. Handler reacts only to `ShortcutState::Pressed`: popover calls `toggle_overlay_visibility`, refresh notifies `usage_wake`, settings calls `show_settings_window(app,None)`. Return a specific `set_config` error naming the conflicting shortcut; never silently save an inactive binding.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shortcuts::tests && cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/shortcuts.rs src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/src/config.rs
git commit -m "feat(automation): add global runtime shortcuts"
```

### Task 6: Verify launch-at-login and expose runtime status

**Files:**
- Modify: `src-tauri/src/startup.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/settings.tsx`
- Modify: `src/components/settings.test.tsx`

- [ ] **Step 1: Write failing status tests**

```rust
#[test] fn startup_status_reports_disabled_when_value_absent() { assert_eq!(read_registration_with(|_|Ok(None)).unwrap(),false); }
```

```tsx
it("shows offline state and refresh acknowledgement", async()=>{ render(<AutomationSettings status={{online:false,lastRefreshAt:null}}/>); expect(screen.getByText("Offline — automatic polling paused")).toBeInTheDocument(); await user.click(screen.getByRole("button",{name:"Refresh now"})); expect(invoke).toHaveBeenCalledWith("refresh_usage"); });
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml startup::tests && npm test -- --run src/components/settings.test.tsx`
Expected: FAIL because readback/status UI are missing.

- [ ] **Step 3: Implement status without a second startup system**

Add registry readback for the existing exact `RUN_VALUE_NAME`; `set_config` returns an error if requested launch state cannot be applied, rather than saving a lie. Add `get_runtime_status -> {online,lastRefreshAt,launchAtLoginRegistered,autoInitLastAttemptAt}`. Emit `runtime-status-changed` on transitions and completed refreshes. UI shows online/offline, last successful refresh, startup registration mismatch, auto-init cooldown, and a Refresh now button.

- [ ] **Step 4: Final verification**

Run: `npm test && npm run build && cargo fmt --manifest-path src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: every command exits 0; default config launches no session, registers no shortcuts, and preserves 60-second polling.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/startup.rs src-tauri/src/lib.rs src/components/settings.tsx src/components/settings.test.tsx
git commit -m "feat(automation): report runtime and startup status"
```

### Task 7: Document safety and operational boundaries

**Files:**
- Modify: `README.md`

- [ ] Document auto-init as off-by-default, the exact cost warning, fixed model catalog, 30-minute retry cooldown, offline polling behavior, 15-second minimum interval, launch-at-login location, and shortcut conflict behavior.
- [ ] Run: `rg -n "auto.initialize|cost|15.second|offline|shortcut" README.md`
Expected: matches every documented boundary.
- [ ] Commit:

```bash
git add README.md
git commit -m "docs(automation): explain runtime safeguards"
```
