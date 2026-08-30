# Notifications and Pace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Calculate consumption pace, show pace markers, and deliver restart-safe, deduplicated Windows threshold notifications with configurable sounds and confirmed resets.

**Architecture:** Pure Rust computes expected linear usage and threshold crossings from consecutive samples. A local JSON notification ledger provides atomic restart persistence; Tauri's notification plugin sends Windows toasts, while a Windows sound adapter plays a validated built-in system alias. React renders pace markers and settings but never decides whether a notification is due.

**Tech Stack:** Tauri 2, tauri-plugin-notification 2, Rust/serde, Windows `PlaySoundW`, React + TypeScript, Vitest

---

## Prerequisites and file map

- Branch after merging `codex/feature-react-rendering-foundation`, `codex/feature-display-system`, and `codex/feature-claude-usage-v2`; pace markers reuse the display-system primitives and threshold decisions consume the normalized dynamic Claude windows. The feature remains single-account, with no profile key in config or ledger.
- Add `tauri-plugin-notification = "2"`; initialize `.plugin(tauri_plugin_notification::init())`. Tauri 2 requires the plugin rather than the removed core notification API.
- Extend `windows-sys` features with `Win32_Media_Audio` for built-in sound aliases; do not accept arbitrary file paths.
- Create `src-tauri/src/pace.rs`, `notifications.rs`, `notification_store.rs`, plus focused unit tests in each module.
- Modify `config.rs`, `lib.rs`, `model.rs`, `types.ts`, overlay/settings React components and CSS.

### Task 1: Define pace math and wire representation

**Files:**
- Create: `src-tauri/src/pace.rs`
- Modify: `src-tauri/src/model.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Write failing pace tests**

```rust
#[test] fn computes_expected_marker_and_status() {
  let p = calculate(PaceInput { used_percent: 60.0, sampled_at: 4_500, window_started_at: 0, resets_at: 10_000 }).unwrap();
  assert_eq!(p.expected_percent, 45.0); assert_eq!(p.delta_percent, 15.0); assert_eq!(p.status, PaceStatus::Ahead);
}
#[test] fn rejects_unknown_or_elapsed_windows() {
  assert_eq!(calculate(PaceInput { used_percent: 10.0, sampled_at: 10, window_started_at: 10, resets_at: 0 }), None);
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml pace::tests -- --nocapture`
Expected: FAIL with module `pace` not found.

- [ ] **Step 3: Implement the pure calculation**

```rust
#[derive(Debug, Clone, Copy)] pub struct PaceInput { pub used_percent:f32, pub sampled_at:i64, pub window_started_at:i64, pub resets_at:i64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all="kebab-case")] pub enum PaceStatus { Behind, OnPace, Ahead }
#[derive(Debug, Clone, Copy, PartialEq, Serialize)] #[serde(rename_all="camelCase")] pub struct Pace { pub expected_percent:f32, pub delta_percent:f32, pub status:PaceStatus }
pub fn calculate(i: PaceInput) -> Option<Pace> {
  let duration=i.resets_at.checked_sub(i.window_started_at)?; let elapsed=i.sampled_at.checked_sub(i.window_started_at)?;
  if duration<=0 || elapsed<0 || elapsed>duration { return None; }
  let expected=(elapsed as f32/duration as f32*100.0).clamp(0.0,100.0); let delta=i.used_percent-expected;
  Some(Pace { expected_percent:expected, delta_percent:delta, status:if delta>5.0{PaceStatus::Ahead}else if delta< -5.0{PaceStatus::Behind}else{PaceStatus::OnPace} })
}
```

Add optional `pace` to each serialized `UsageWindow`. Derive `window_started_at` from stable known duration (`5h`, `24h`, `7d`) as `resets_at-duration`; return null for unknown reset/duration. This marker is a linear-budget guide, explicitly not a forecast.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml pace::tests && npm test -- --run src/state.test.ts`
Expected: PASS and older payloads without pace still deserialize in TypeScript fixtures.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/pace.rs src-tauri/src/model.rs src-tauri/src/lib.rs src/types.ts
git commit -m "feat(pace): calculate consumption pace"
```

### Task 2: Render accessible pace markers

**Files:**
- Modify: `src/components/layer.tsx`
- Modify: `src/components/layer.test.tsx`
- Modify: `src/styles/app.css`

- [ ] **Step 1: Write the failing UI test**

```tsx
it("labels the expected-use marker without changing meter value", () => {
 render(<UsageWindowCard window={{...window, pace:{expectedPercent:45,deltaPercent:15,status:"ahead"}}} />);
 expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow","60");
 expect(screen.getByText("15 points ahead of pace")).toBeInTheDocument();
 expect(screen.getByTestId("pace-marker")).toHaveStyle({left:"45%"});
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- --run src/components/layer.test.tsx`
Expected: FAIL because no pace marker/status exists.

- [ ] **Step 3: Render marker and copy**

Inside the meter, add `<span data-testid="pace-marker" className="meter__pace" style={{left: `${pace.expectedPercent}%`}} aria-hidden />`; below it render rounded absolute delta as “N points ahead of pace”, “N points under pace”, or “On pace”. Omit both for null pace. Use a 2px high-contrast marker and never encode status by color alone.

- [ ] **Step 4: Verify green and accessibility**

Run: `npm test -- --run src/components/layer.test.tsx src/a11y.test.ts`
Expected: PASS with no axe violations.

- [ ] **Step 5: Commit**

```bash
git add src/components/layer.tsx src/components/layer.test.tsx src/styles/app.css
git commit -m "feat(pace): show expected-use markers"
```

### Task 3: Add threshold configuration and sanitization

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src/types.ts`
- Modify: `src/components/settings.tsx`
- Modify: `src/components/settings.test.tsx`

- [ ] **Step 1: Write failing config tests**

```rust
#[test] fn notification_defaults_are_safe() { let c=Config::default(); assert_eq!(c.notification_thresholds, vec![75,90,95]); assert_eq!(c.notification_sound,"Default"); }
#[test] fn thresholds_are_sorted_deduped_and_bounded() { let c=Config{notification_thresholds:vec![95,0,75,75,101],..Default::default()}.sanitized(); assert_eq!(c.notification_thresholds,vec![75,95]); }
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config::tests::notification_defaults_are_safe -- --exact`
Expected: FAIL because fields are absent.

- [ ] **Step 3: Add exact config fields**

```rust
#[serde(default="default_true")] pub notifications_enabled: bool,
#[serde(default="default_thresholds")] pub notification_thresholds: Vec<u8>,
#[serde(default="default_notification_sound")] pub notification_sound: String,
fn default_thresholds()->Vec<u8>{vec![75,90,95]}
fn default_notification_sound()->String{"Default".into()}
```

Sanitize by retaining `1..=100`, sorting, deduplicating, restoring defaults if empty, and accepting only `Default|None|Asterisk|Exclamation|Hand`. Settings provide enabled toggle, numeric threshold chips with Add/Remove, validation text, a fixed sound select, and “Test sound”; changes continue through the existing whole-config `set_config` path.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config::tests && npm test -- --run src/components/settings.test.tsx`
Expected: PASS, including migration from old config JSON.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src/types.ts src/components/settings.tsx src/components/settings.test.tsx
git commit -m "feat(notifications): configure thresholds and sounds"
```

### Task 4: Persist a deduplication ledger across restarts

**Files:**
- Create: `src-tauri/src/notification_store.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing persistence tests**

```rust
#[test] fn round_trip_preserves_sent_crossing() { let d=tempdir().unwrap(); let p=d.path().join("notifications.json"); let mut s=NotificationStore::load(&p); s.mark_sent("claude","session_5h",2000,90,1000); s.save(&p).unwrap(); assert!(NotificationStore::load(&p).was_sent("claude","session_5h",2000,90)); }
#[test] fn corrupt_file_is_quarantined_and_starts_empty() {
  let d=tempdir().unwrap(); let p=d.path().join("notifications.json");
  std::fs::write(&p,"{broken").unwrap();
  let s=NotificationStore::load(&p);
  assert!(s.sent.is_empty());
  assert!(d.path().join("notifications.corrupt.json").exists());
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml notification_store::tests -- --nocapture`
Expected: FAIL because module is absent.

- [ ] **Step 3: Implement the versioned ledger**

```rust
#[derive(Default,Serialize,Deserialize)] #[serde(rename_all="camelCase")]
pub struct NotificationStore { pub schema_version:u8, pub sent:Vec<SentThreshold> }
#[derive(Serialize,Deserialize)] #[serde(rename_all="camelCase")]
pub struct SentThreshold { provider:String, window_kind:String, resets_at:i64, threshold:u8, sent_at:i64 }
```

Use key `(provider,window_kind,resets_at,threshold)`, atomic sibling-temp save + rename, quarantine malformed files to `notifications.corrupt.json`, prune entries older than 35 days, and manage `Mutex<NotificationStore>` loaded from `app_config_dir/notifications.json`. Saving failure is diagnostic and suppresses the toast for that cycle: mark and persist before sending, so a crash cannot duplicate.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml notification_store::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/notification_store.rs src-tauri/src/lib.rs
git commit -m "feat(notifications): persist threshold ledger"
```

### Task 5: Detect crossings and send Windows notifications with sound

**Files:**
- Create: `src-tauri/src/notifications.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing crossing tests**

```rust
#[test] fn emits_each_crossed_threshold_in_order() { assert_eq!(crossings(Some(74.0),96.0,&[75,90,95]),vec![75,90,95]); }
#[test] fn first_observation_does_not_backfill_alerts() { assert!(crossings(None,96.0,&[75,90,95]).is_empty()); }
#[test] fn falling_usage_does_not_alert() { assert!(crossings(Some(96.0),20.0,&[75,90,95]).is_empty()); }
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml notifications::tests -- --nocapture`
Expected: FAIL because `crossings` is missing.

- [ ] **Step 3: Implement crossing orchestration and sound adapter**

```rust
pub fn crossings(previous:Option<f32>, current:f32, thresholds:&[u8])->Vec<u8> {
 let Some(previous)=previous else{return vec![]};
 thresholds.iter().copied().filter(|t| previous < *t as f32 && current >= *t as f32).collect()
}
```

For every fresh window, compare to the last accepted sample only when `resets_at` is unchanged. Check ledger, persist, then send `Usage Tracker` / `Claude 5 hour usage reached 90%` using `app.notification().builder().title(...).body(...).show()`. Play the selected system alias after a successful toast with `PlaySoundW(alias, null_mut(), SND_ALIAS|SND_ASYNC|SND_NODEFAULT)`; `Default` lets Windows toast policy sound, `None` is silent. Add command `test_notification_sound(sound)` using the same whitelist. Do not alert on stale/error/signed-out data, first observation, reset change, or falling values.

- [ ] **Step 4: Verify green and regression**

Run: `cargo test --manifest-path src-tauri/Cargo.toml notifications::tests && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/notifications.rs src-tauri/src/lib.rs
git commit -m "feat(notifications): send deduplicated threshold alerts"
```

### Task 6: Add confirmed notification-state reset

**Files:**
- Modify: `src-tauri/src/notification_store.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/settings.tsx`
- Modify: `src/components/settings.test.tsx`

- [ ] **Step 1: Write failing confirmation test**

```tsx
it("requires explicit confirmation before resetting sent alerts", async()=>{ render(<NotificationSettings {...props}/>); await user.click(screen.getByRole("button",{name:"Reset notification history"})); expect(invoke).not.toHaveBeenCalled(); await user.click(screen.getByRole("button",{name:"Confirm reset"})); expect(invoke).toHaveBeenCalledWith("reset_notification_history"); });
```

- [ ] **Step 2: Verify red**

Run: `npm test -- --run src/components/settings.test.tsx`
Expected: FAIL because reset controls are missing.

- [ ] **Step 3: Implement reset**

`reset_notification_history` clears and atomically saves the ledger. UI confirmation copy states that current windows may notify again only after a future below-to-above crossing; cancel and Escape close it, failure remains visible, and success announces through `role="status"`.

- [ ] **Step 4: Final verification**

Run: `npm test && npm run build && cargo fmt --manifest-path src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all commands exit 0.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/notification_store.rs src-tauri/src/lib.rs src/components/settings.tsx src/components/settings.test.tsx
git commit -m "feat(notifications): confirm alert-history reset"
```
