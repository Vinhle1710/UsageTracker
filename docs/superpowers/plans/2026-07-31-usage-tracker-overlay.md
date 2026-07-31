# Usage Tracker Overlay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Windows desktop overlay that displays live Claude and Codex usage limits, appearing automatically while any AI client is running and hiding when the last one closes.

**Architecture:** A single Tauri v2 process. Rust owns credential reading, HTTP polling, process/lock-file detection, and window placement; it pushes normalized snapshots to a web UI over Tauri events. The web layer is pure presentation with no vendor-specific branching. Both providers normalize into one `UsageSnapshot` shape so the UI renders a list of windows rather than fixed 5-hour/weekly slots.

**Tech Stack:** Tauri v2, Rust (reqwest, serde, sysinfo, tokio), TypeScript, Vite, Vitest, `cargo test` + mockito.

**Spec:** `docs/superpowers/specs/2026-07-31-usage-tracker-overlay-design.md`

---

## Prerequisites

Verified present on the target machine: Node v25.6.0, npm 11.11.0, WebView2 runtime 150.0.4078.105, winget.

Verified **absent** — must be installed before Phase 1:

```bash
winget install --id Rustlang.Rustup -e
```

```bash
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Verify: `rustc --version` and `cargo --version` both succeed in a fresh terminal.

**Phase 0 requires none of this** and can run immediately with Node.

---

## File Structure

| Path | Responsibility |
|---|---|
| `scripts/verify-endpoints.mjs` | Phase 0 spike. Confirms endpoint shapes and whether polling is metered. Throwaway-but-kept. |
| `src-tauri/src/model.rs` | `UsageWindow`, `UsageSnapshot`, `SnapshotState`. No I/O. |
| `src-tauri/src/config.rs` | Settings load/save with defaulting. No I/O beyond its own file. |
| `src-tauri/src/creds.rs` | Reads tokens from disk, read-only. Never logs. |
| `src-tauri/src/detect.rs` | Process + lock-file scan → `ActiveSources`. |
| `src-tauri/src/providers/claude.rs` | Claude fetch + parse → `UsageSnapshot`. |
| `src-tauri/src/providers/codex.rs` | Codex fetch + parse + JSONL fallback → `UsageSnapshot`. |
| `src-tauri/src/poller.rs` | 60s usage tick, 5s detect tick, visibility-gated. |
| `src-tauri/src/window.rs` | Monitor/corner placement, size states. |
| `src/types.ts` | TS mirror of the Rust model. |
| `src/format.ts` | Percent and reset-time formatting. Pure functions. |
| `src/state.ts` | Size-state machine and derived values. Pure. |
| `src/components/layer.ts` | One provider card. |
| `src/components/bubble.ts` | Bubble view. |
| `src/components/controls.ts` | Custom window controls. |
| `src/styles/tokens.css` | Design tokens. |
| `src/styles/app.css` | Layout, container queries, themes. |

Parsing is deliberately separated from fetching in both providers so tests never touch the network.

---

## Phase 0: Verify Endpoints and Metering

Everything downstream depends on two unproven assumptions: the response shapes, and that polling does not consume quota. This phase costs ~10 minutes and de-risks the entire build. **Do not skip it.**

### Task 0.1: Endpoint verification spike

**Files:**
- Create: `scripts/verify-endpoints.mjs`

- [ ] **Step 1: Write the spike script**

```javascript
// scripts/verify-endpoints.mjs
// Confirms usage endpoint shapes and whether polling consumes quota.
// Prints structure only. Never prints tokens.
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const readJson = (p) => JSON.parse(readFileSync(p, "utf8"));

function claudeToken() {
  const raw = readJson(join(homedir(), ".claude", ".credentials.json"));
  const found = JSON.stringify(raw).match(/"accessToken":"([^"]+)"/);
  if (!found) throw new Error("no accessToken in .credentials.json");
  return found[1];
}

function codexToken() {
  return readJson(join(homedir(), ".codex", "auth.json")).tokens.access_token;
}

async function probe(name, url, token, extraHeaders = {}) {
  const res = await fetch(url, {
    headers: { Authorization: `Bearer ${token}`, ...extraHeaders },
  });
  const text = await res.text();
  let body;
  try {
    body = JSON.parse(text);
  } catch {
    body = text.slice(0, 300);
  }
  console.log(`\n=== ${name} :: HTTP ${res.status} ===`);
  console.log(JSON.stringify(body, null, 2).slice(0, 2000));
  return body;
}

// Returns every numeric field whose key hints at consumption.
function usageFingerprint(obj, path = "") {
  const out = {};
  const walk = (o, p) => {
    if (o && typeof o === "object") {
      for (const [k, v] of Object.entries(o)) {
        const next = p ? `${p}.${k}` : k;
        if (typeof v === "number" && /percent|used|utilization|count/i.test(k)) {
          out[next] = v;
        }
        walk(v, next);
      }
    }
  };
  walk(obj, path);
  return out;
}

const targets = [
  {
    name: "claude",
    url: "https://api.anthropic.com/api/oauth/usage",
    token: claudeToken(),
    headers: { "anthropic-beta": "oauth-2025-04-20" },
  },
  {
    name: "codex",
    url: "https://chatgpt.com/backend-api/api/codex/usage",
    token: codexToken(),
    headers: {},
  },
];

for (const t of targets) {
  try {
    const first = await probe(t.name, t.url, t.token, t.headers);
    const before = usageFingerprint(first);
    console.log(`\n--- ${t.name} metering check: 10 polls ---`);
    console.log("before:", JSON.stringify(before));
    let last = first;
    for (let i = 0; i < 9; i++) {
      const res = await fetch(t.url, {
        headers: { Authorization: `Bearer ${t.token}`, ...t.headers },
      });
      last = await res.json();
    }
    const after = usageFingerprint(last);
    console.log("after: ", JSON.stringify(after));
    const moved = Object.keys(before).filter((k) => before[k] !== after[k]);
    console.log(
      moved.length
        ? `METERED? fields changed: ${moved.join(", ")}`
        : "NOT METERED: no usage field moved across 10 polls",
    );
  } catch (err) {
    console.log(`\n=== ${t.name} FAILED: ${err.message} ===`);
  }
}
```

- [ ] **Step 2: Run the spike**

Run: `node scripts/verify-endpoints.mjs`

Expected: two HTTP 200 blocks showing JSON structure, then a `NOT METERED` line for each.

- [ ] **Step 3: Record the outcome**

Append a "Phase 0 results" section to the spec at
`docs/superpowers/specs/2026-07-31-usage-tracker-overlay-design.md`, recording the real
field names for both providers and the metering verdict. Change §2 rows from
**Inferred** to **Verified** where confirmed.

**Decision gate:**
- If HTTP 200 and not metered → continue to Phase 1 unchanged.
- If a field moved → polling is metered. Change §3.6: Codex sources from JSONL only, Claude interval rises to 15 minutes. Update the spec before continuing.
- If HTTP 401 → the header or token location is wrong. Fix here, before any Rust is written.
- If HTTP 404 → the path is wrong. Re-derive from the binary strings and retry.

- [ ] **Step 4: Commit**

```bash
git add scripts/verify-endpoints.mjs docs/superpowers/specs/
git commit -m "test: verify usage endpoint shapes and metering behavior"
```

---

## Phase 1: Project Scaffold

### Task 1.1: Initialize Tauri v2 + Vite + TypeScript

**Files:**
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src/main.ts`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Scaffold**

Run: `npm create tauri-app@latest . -- --template vanilla-ts --manager npm --yes`

If the directory-not-empty check blocks it, scaffold into `.tmp-scaffold/` and move files in,
preserving the existing `docs/`, `scripts/`, and `.gitignore`.

- [ ] **Step 2: Install dependencies**

```bash
npm install
```

```bash
npm install -D vitest @vitest/coverage-v8 jsdom vitest-axe
```

- [ ] **Step 3: Add Rust dependencies**

Edit `src-tauri/Cargo.toml`, adding to `[dependencies]`:

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
tokio = { version = "1", features = ["time", "rt-multi-thread"] }
sysinfo = "0.32"
tauri-plugin-positioner = "2"
directories = "5"

[dev-dependencies]
mockito = "1"
tempfile = "3"
```

- [ ] **Step 4: Verify both sides build**

Run: `npm run build`
Expected: Vite build succeeds.

Run: `cd src-tauri && cargo check`
Expected: `Finished` with no errors. First run compiles many crates and is slow.

- [ ] **Step 5: Configure test scripts**

Add to `package.json` `"scripts"`:

```json
"test": "vitest run",
"test:watch": "vitest",
"coverage": "vitest run --coverage"
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri v2 project with Vite, TypeScript, and Vitest"
```

---

## Phase 2: Core Model and Config

### Task 2.1: Normalized usage model

**Files:**
- Create: `src-tauri/src/model.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/model.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotState {
    Fresh,
    Stale,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageWindow {
    pub label: String,
    pub used_percent: f32,
    pub resets_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub windows: Vec<UsageWindow>,
    pub fetched_at: i64,
    pub state: SnapshotState,
}

/// Converts a window duration in minutes into a display label.
pub fn label_for_minutes(minutes: u32) -> String {
    match minutes {
        m if m % 10080 == 0 => {
            let weeks = m / 10080;
            if weeks == 1 { "Weekly".into() } else { format!("{weeks} weeks") }
        }
        m if m % 1440 == 0 => {
            let days = m / 1440;
            if days == 1 { "Daily".into() } else { format!("{days} days") }
        }
        m if m % 60 == 0 => format!("{} hour", m / 60),
        m => format!("{m} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_weekly_window() {
        assert_eq!(label_for_minutes(10080), "Weekly");
    }

    #[test]
    fn labels_five_hour_window() {
        assert_eq!(label_for_minutes(300), "5 hour");
    }

    #[test]
    fn labels_daily_window() {
        assert_eq!(label_for_minutes(1440), "Daily");
    }

    #[test]
    fn labels_odd_window_in_minutes() {
        assert_eq!(label_for_minutes(45), "45 min");
    }
}
```

- [ ] **Step 2: Register the module**

Add to the top of `src-tauri/src/lib.rs`:

```rust
pub mod model;
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test model::`
Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/model.rs src-tauri/src/lib.rs
git commit -m "feat: add normalized usage model with window labelling"
```

### Task 2.2: Config with defaulting

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/config.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub monitor_id: Option<String>,
    #[serde(default = "default_corner")]
    pub corner: String,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default = "default_size_state")]
    pub size_state: String,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default)]
    pub offscreen_peek: bool,
    #[serde(default = "default_poll")]
    pub poll_interval_sec: u64,
    #[serde(default = "default_detect")]
    pub detect_interval_sec: u64,
}

fn default_corner() -> String { "bottom-right".into() }
fn default_scale() -> f32 { 1.0 }
fn default_size_state() -> String { "compact".into() }
fn default_true() -> bool { true }
fn default_poll() -> u64 { 60 }
fn default_detect() -> u64 { 5 }

impl Default for Config {
    fn default() -> Self {
        Config {
            monitor_id: None,
            corner: default_corner(),
            scale: default_scale(),
            size_state: default_size_state(),
            always_on_top: default_true(),
            offscreen_peek: false,
            poll_interval_sec: default_poll(),
            detect_interval_sec: default_detect(),
        }
    }
}

impl Config {
    /// Loads config, falling back to defaults for a missing or invalid file.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap())
    }

    /// Clamps out-of-range values to supported bounds.
    pub fn sanitized(mut self) -> Self {
        self.scale = self.scale.clamp(0.75, 1.5);
        self.poll_interval_sec = self.poll_interval_sec.max(30);
        self.detect_interval_sec = self.detect_interval_sec.max(1);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempdir().unwrap();
        let cfg = Config::load(&dir.path().join("nope.json"));
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn invalid_json_yields_defaults() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("c.json");
        std::fs::write(&p, "{ not json").unwrap();
        assert_eq!(Config::load(&p), Config::default());
    }

    #[test]
    fn partial_config_fills_missing_fields() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("c.json");
        std::fs::write(&p, r#"{"corner":"top-left"}"#).unwrap();
        let cfg = Config::load(&p);
        assert_eq!(cfg.corner, "top-left");
        assert_eq!(cfg.scale, 1.0);
        assert!(cfg.always_on_top);
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("c.json");
        let mut cfg = Config::default();
        cfg.corner = "top-right".into();
        cfg.save(&p).unwrap();
        assert_eq!(Config::load(&p), cfg);
    }

    #[test]
    fn sanitize_clamps_out_of_range_scale() {
        let cfg = Config { scale: 9.0, ..Default::default() }.sanitized();
        assert_eq!(cfg.scale, 1.5);
    }

    #[test]
    fn sanitize_enforces_minimum_poll_interval() {
        let cfg = Config { poll_interval_sec: 1, ..Default::default() }.sanitized();
        assert_eq!(cfg.poll_interval_sec, 30);
    }
}
```

- [ ] **Step 2: Register the module**

Add to `src-tauri/src/lib.rs`:

```rust
pub mod config;
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test config::`
Expected: 6 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "feat: add config module with defaulting and range clamping"
```

---

## Phase 3: Credentials and Providers

### Task 3.1: Credential reading

**Files:**
- Create: `src-tauri/src/creds.rs`
- Modify: `src-tauri/src/lib.rs`

Tokens must never appear in logs, errors, or `Debug` output. `TokenError` deliberately
carries no token material.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/creds.rs`:

```rust
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum TokenError {
    NotFound,
    Unreadable,
    Malformed,
}

/// Extracts the Claude OAuth access token.
/// Field names confirmed in Phase 0; adjust the pointer list if they differ.
pub fn claude_token_from_str(s: &str) -> Result<String, TokenError> {
    let v: serde_json::Value = serde_json::from_str(s).map_err(|_| TokenError::Malformed)?;
    for ptr in [
        "/claudeAiOauth/accessToken",
        "/accessToken",
        "/tokens/access_token",
    ] {
        if let Some(t) = v.pointer(ptr).and_then(|x| x.as_str()) {
            if !t.is_empty() {
                return Ok(t.to_string());
            }
        }
    }
    Err(TokenError::Malformed)
}

pub fn codex_token_from_str(s: &str) -> Result<String, TokenError> {
    let v: serde_json::Value = serde_json::from_str(s).map_err(|_| TokenError::Malformed)?;
    v.pointer("/tokens/access_token")
        .and_then(|x| x.as_str())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .ok_or(TokenError::Malformed)
}

pub fn read_token(path: &Path, parse: fn(&str) -> Result<String, TokenError>) -> Result<String, TokenError> {
    if !path.exists() {
        return Err(TokenError::NotFound);
    }
    let s = std::fs::read_to_string(path).map_err(|_| TokenError::Unreadable)?;
    parse(&s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn extracts_codex_token() {
        let s = r#"{"tokens":{"access_token":"abc123","refresh_token":"r"}}"#;
        assert_eq!(codex_token_from_str(s).unwrap(), "abc123");
    }

    #[test]
    fn extracts_nested_claude_token() {
        let s = r#"{"claudeAiOauth":{"accessToken":"xyz789"}}"#;
        assert_eq!(claude_token_from_str(s).unwrap(), "xyz789");
    }

    #[test]
    fn rejects_empty_token() {
        let s = r#"{"tokens":{"access_token":""}}"#;
        assert_eq!(codex_token_from_str(s), Err(TokenError::Malformed));
    }

    #[test]
    fn rejects_malformed_json() {
        assert_eq!(codex_token_from_str("{oops"), Err(TokenError::Malformed));
    }

    #[test]
    fn missing_file_reports_not_found() {
        let dir = tempdir().unwrap();
        let r = read_token(&dir.path().join("none.json"), codex_token_from_str);
        assert_eq!(r, Err(TokenError::NotFound));
    }

    #[test]
    fn error_debug_output_contains_no_token_material() {
        let s = r#"{"tokens":{"access_token":"SUPERSECRET"}}"#;
        let token = codex_token_from_str(s).unwrap();
        let err = format!("{:?}", TokenError::Malformed);
        assert!(!err.contains(&token));
        assert!(!err.contains("SUPERSECRET"));
    }
}
```

- [ ] **Step 2: Register the module**

Add to `src-tauri/src/lib.rs`:

```rust
pub mod creds;
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test creds::`
Expected: 6 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/creds.rs src-tauri/src/lib.rs
git commit -m "feat: add read-only credential loading with no token leakage"
```

### Task 3.2: Codex provider parsing

**Files:**
- Create: `src-tauri/src/providers/mod.rs`, `src-tauri/src/providers/codex.rs`
- Modify: `src-tauri/src/lib.rs`

Parsing is split from fetching so these tests never touch the network. The fixture is the
**real payload** captured from the machine on 2026-07-31.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/providers/codex.rs`:

```rust
use crate::model::{label_for_minutes, SnapshotState, UsageSnapshot, UsageWindow};

/// Parses a Codex `rate_limits` object into normalized windows.
/// Absent windows produce no row; a window present at 0% produces a row.
pub fn parse_rate_limits(v: &serde_json::Value, fetched_at: i64, state: SnapshotState) -> UsageSnapshot {
    let mut windows = Vec::new();
    for key in ["primary", "secondary"] {
        let Some(w) = v.get(key) else { continue };
        if w.is_null() {
            continue;
        }
        let (Some(used), Some(mins)) = (
            w.get("used_percent").and_then(|x| x.as_f64()),
            w.get("window_minutes").and_then(|x| x.as_u64()),
        ) else {
            continue;
        };
        windows.push(UsageWindow {
            label: label_for_minutes(mins as u32),
            used_percent: used as f32,
            resets_at: w.get("resets_at").and_then(|x| x.as_i64()).unwrap_or(0),
        });
    }
    UsageSnapshot { windows, fetched_at, state }
}

/// Extracts the last `rate_limits` object from a Codex session JSONL file.
pub fn last_rate_limits_from_jsonl(contents: &str) -> Option<serde_json::Value> {
    contents
        .lines()
        .rev()
        .find_map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).ok()?;
            find_key(&v, "rate_limits")
        })
}

fn find_key(v: &serde_json::Value, key: &str) -> Option<serde_json::Value> {
    match v {
        serde_json::Value::Object(m) => {
            if let Some(found) = m.get(key) {
                return Some(found.clone());
            }
            m.values().find_map(|x| find_key(x, key))
        }
        serde_json::Value::Array(a) => a.iter().find_map(|x| find_key(x, key)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real payload captured from ~/.codex/sessions on 2026-07-31.
    const REAL: &str = r#"{
      "limit_id":"codex","limit_name":null,
      "primary":{"used_percent":25.0,"window_minutes":10080,"resets_at":1785978830},
      "secondary":null,
      "credits":{"has_credits":false,"unlimited":false,"balance":"0"},
      "plan_type":"plus","rate_limit_reached_type":null
    }"#;

    #[test]
    fn parses_real_payload_with_null_secondary() {
        let v: serde_json::Value = serde_json::from_str(REAL).unwrap();
        let s = parse_rate_limits(&v, 100, SnapshotState::Fresh);
        assert_eq!(s.windows.len(), 1);
        assert_eq!(s.windows[0].label, "Weekly");
        assert_eq!(s.windows[0].used_percent, 25.0);
        assert_eq!(s.windows[0].resets_at, 1785978830);
    }

    #[test]
    fn renders_both_windows_when_present() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"primary":{"used_percent":10.0,"window_minutes":300,"resets_at":1},
                "secondary":{"used_percent":40.0,"window_minutes":10080,"resets_at":2}}"#,
        ).unwrap();
        let s = parse_rate_limits(&v, 0, SnapshotState::Fresh);
        assert_eq!(s.windows.len(), 2);
        assert_eq!(s.windows[0].label, "5 hour");
        assert_eq!(s.windows[1].label, "Weekly");
    }

    #[test]
    fn window_at_zero_percent_still_renders() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"primary":{"used_percent":0.0,"window_minutes":300,"resets_at":1}}"#,
        ).unwrap();
        let s = parse_rate_limits(&v, 0, SnapshotState::Fresh);
        assert_eq!(s.windows.len(), 1);
        assert_eq!(s.windows[0].used_percent, 0.0);
    }

    #[test]
    fn absent_window_produces_no_row() {
        let v: serde_json::Value = serde_json::from_str(r#"{"secondary":null}"#).unwrap();
        let s = parse_rate_limits(&v, 0, SnapshotState::Fresh);
        assert!(s.windows.is_empty());
    }

    #[test]
    fn extracts_last_rate_limits_from_jsonl() {
        let jsonl = format!(
            "{}\n{}\n",
            r#"{"msg":{"info":{"rate_limits":{"primary":{"used_percent":5.0,"window_minutes":300,"resets_at":1}}}}}"#,
            r#"{"msg":{"info":{"rate_limits":{"primary":{"used_percent":9.0,"window_minutes":300,"resets_at":2}}}}}"#
        );
        let v = last_rate_limits_from_jsonl(&jsonl).unwrap();
        let s = parse_rate_limits(&v, 0, SnapshotState::Stale);
        assert_eq!(s.windows[0].used_percent, 9.0);
        assert_eq!(s.state, SnapshotState::Stale);
    }

    #[test]
    fn jsonl_without_rate_limits_returns_none() {
        assert!(last_rate_limits_from_jsonl(r#"{"msg":"hello"}"#).is_none());
    }
}
```

- [ ] **Step 2: Register the modules**

Create `src-tauri/src/providers/mod.rs`:

```rust
pub mod claude;
pub mod codex;
```

Add to `src-tauri/src/lib.rs`:

```rust
pub mod providers;
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test codex::`
Expected: 6 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/providers/
git commit -m "feat: add Codex rate limit parsing with JSONL fallback"
```

### Task 3.3: Claude provider parsing

**Files:**
- Create: `src-tauri/src/providers/claude.rs`

Adjust field names to whatever Phase 0 recorded before writing this.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/providers/claude.rs`:

```rust
use crate::model::{SnapshotState, UsageSnapshot, UsageWindow};

/// Parses the Claude usage payload. Each window is optional and absent windows
/// produce no row, matching the Codex behaviour.
pub fn parse_usage(v: &serde_json::Value, fetched_at: i64, state: SnapshotState) -> UsageSnapshot {
    let mut windows = Vec::new();
    for (key, label) in [("five_hour", "5 hour"), ("seven_day", "Weekly")] {
        let Some(w) = v.get(key) else { continue };
        if w.is_null() {
            continue;
        }
        let Some(util) = w.get("utilization").and_then(|x| x.as_f64()) else { continue };
        windows.push(UsageWindow {
            label: label.to_string(),
            used_percent: util as f32,
            resets_at: w.get("resets_at").and_then(|x| x.as_i64()).unwrap_or(0),
        });
    }
    UsageSnapshot { windows, fetched_at, state }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_windows() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"five_hour":{"utilization":12.5,"resets_at":100},
                "seven_day":{"utilization":48.0,"resets_at":200}}"#,
        ).unwrap();
        let s = parse_usage(&v, 0, SnapshotState::Fresh);
        assert_eq!(s.windows.len(), 2);
        assert_eq!(s.windows[0].label, "5 hour");
        assert_eq!(s.windows[0].used_percent, 12.5);
        assert_eq!(s.windows[1].used_percent, 48.0);
    }

    #[test]
    fn absent_five_hour_window_hides_row() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"seven_day":{"utilization":48.0,"resets_at":200}}"#).unwrap();
        let s = parse_usage(&v, 0, SnapshotState::Fresh);
        assert_eq!(s.windows.len(), 1);
        assert_eq!(s.windows[0].label, "Weekly");
    }

    #[test]
    fn five_hour_at_zero_still_renders() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"five_hour":{"utilization":0.0,"resets_at":100}}"#).unwrap();
        let s = parse_usage(&v, 0, SnapshotState::Fresh);
        assert_eq!(s.windows.len(), 1);
        assert_eq!(s.windows[0].used_percent, 0.0);
    }

    #[test]
    fn empty_payload_yields_no_windows() {
        let v: serde_json::Value = serde_json::from_str("{}").unwrap();
        assert!(parse_usage(&v, 0, SnapshotState::Fresh).windows.is_empty());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test claude::`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/providers/claude.rs
git commit -m "feat: add Claude usage parsing with absent-window handling"
```

### Task 3.4: HTTP fetch with error mapping

**Files:**
- Modify: `src-tauri/src/providers/mod.rs`

- [ ] **Step 1: Write the failing test**

Add to `src-tauri/src/providers/mod.rs`:

```rust
pub mod claude;
pub mod codex;

use crate::model::SnapshotState;

#[derive(Debug, PartialEq)]
pub enum FetchError {
    Unauthorized,
    Network,
    Malformed,
}

/// Maps an HTTP status to a fetch outcome. 429 is treated as a network
/// failure so callers retain last-good values rather than erroring out.
pub fn classify_status(status: u16) -> Option<FetchError> {
    match status {
        200..=299 => None,
        401 | 403 => Some(FetchError::Unauthorized),
        _ => Some(FetchError::Network),
    }
}

pub fn state_for_error(err: &FetchError) -> SnapshotState {
    match err {
        FetchError::Unauthorized | FetchError::Malformed => SnapshotState::Error,
        FetchError::Network => SnapshotState::Stale,
    }
}

pub async fn fetch_json(url: &str, token: &str, extra: &[(&str, &str)]) -> Result<serde_json::Value, FetchError> {
    let client = reqwest::Client::new();
    let mut req = client.get(url).bearer_auth(token);
    for (k, v) in extra {
        req = req.header(*k, *v);
    }
    let res = req.send().await.map_err(|_| FetchError::Network)?;
    if let Some(e) = classify_status(res.status().as_u16()) {
        return Err(e);
    }
    res.json().await.map_err(|_| FetchError::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_status_is_not_an_error() {
        assert_eq!(classify_status(200), None);
    }

    #[test]
    fn unauthorized_maps_to_error_state() {
        let e = classify_status(401).unwrap();
        assert_eq!(e, FetchError::Unauthorized);
        assert_eq!(state_for_error(&e), SnapshotState::Error);
    }

    #[test]
    fn rate_limited_maps_to_stale_not_error() {
        let e = classify_status(429).unwrap();
        assert_eq!(state_for_error(&e), SnapshotState::Stale);
    }

    #[test]
    fn server_error_maps_to_stale() {
        let e = classify_status(500).unwrap();
        assert_eq!(state_for_error(&e), SnapshotState::Stale);
    }

    #[tokio::test]
    async fn fetch_returns_unauthorized_on_401() {
        let mut server = mockito::Server::new_async().await;
        let m = server.mock("GET", "/u").with_status(401).create_async().await;
        let r = fetch_json(&format!("{}/u", server.url()), "tok", &[]).await;
        assert_eq!(r.unwrap_err(), FetchError::Unauthorized);
        m.assert_async().await;
    }

    #[tokio::test]
    async fn fetch_returns_malformed_on_bad_body() {
        let mut server = mockito::Server::new_async().await;
        let m = server.mock("GET", "/u").with_status(200).with_body("not json").create_async().await;
        let r = fetch_json(&format!("{}/u", server.url()), "tok", &[]).await;
        assert_eq!(r.unwrap_err(), FetchError::Malformed);
        m.assert_async().await;
    }

    #[tokio::test]
    async fn fetch_parses_successful_body() {
        let mut server = mockito::Server::new_async().await;
        let m = server.mock("GET", "/u").with_status(200)
            .with_body(r#"{"ok":true}"#).create_async().await;
        let v = fetch_json(&format!("{}/u", server.url()), "tok", &[]).await.unwrap();
        assert_eq!(v["ok"], true);
        m.assert_async().await;
    }
}
```

Add `tokio = { version = "1", features = ["macros", "rt"] }` to `[dev-dependencies]`.

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test providers::tests`
Expected: 7 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/
git commit -m "feat: add HTTP fetch with status classification and 429-as-stale"
```

---

## Phase 4: Detection

### Task 4.1: Lock file parsing with liveness

**Files:**
- Create: `src-tauri/src/detect.rs`
- Modify: `src-tauri/src/lib.rs`

The live-PID check is the point of this task. On the target machine 8 of 9 lock files are
stale, so existence alone would pin the Claude layer on forever.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/detect.rs`:

```rust
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActiveSources {
    pub claude: bool,
    pub openai: bool,
}

#[derive(Debug, PartialEq)]
pub struct IdeLock {
    pub pid: u32,
    pub ide_name: String,
}

pub fn parse_lock(contents: &str) -> Option<IdeLock> {
    let v: serde_json::Value = serde_json::from_str(contents).ok()?;
    Some(IdeLock {
        pid: v.get("pid")?.as_u64()? as u32,
        ide_name: v.get("ideName")?.as_str()?.to_string(),
    })
}

/// Returns true if any lock file in `dir` refers to a currently running PID.
pub fn has_live_ide_lock(dir: &Path, live_pids: &HashSet<u32>) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    entries.filter_map(|e| e.ok()).any(|e| {
        if e.path().extension().and_then(|x| x.to_str()) != Some("lock") {
            return false;
        }
        std::fs::read_to_string(e.path())
            .ok()
            .and_then(|s| parse_lock(&s))
            .is_some_and(|l| live_pids.contains(&l.pid))
    })
}

/// Decides layer visibility from process names and IDE lock liveness.
pub fn resolve(process_names: &[String], has_live_lock: bool) -> ActiveSources {
    let has = |n: &str| process_names.iter().any(|p| p.eq_ignore_ascii_case(n));
    ActiveSources {
        claude: has("claude.exe") || has("claude") || has_live_lock,
        openai: has("ChatGPT.exe") || has("ChatGPT")
            || has("codex.exe") || has("codex")
            || has("codex-code-mode-host.exe"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // Real lock file content from the target machine.
    const LOCK: &str = r#"{"pid":11472,"workspaceFolders":["d:\\GitHub\\Job\\superjunior"],
        "ideName":"Visual Studio Code","transport":"ws","runningInWindows":true,
        "authToken":"3c6c9f0f-1832-43d6-a903-734240952ed0"}"#;

    #[test]
    fn parses_real_lock_file() {
        let l = parse_lock(LOCK).unwrap();
        assert_eq!(l.pid, 11472);
        assert_eq!(l.ide_name, "Visual Studio Code");
    }

    #[test]
    fn stale_lock_with_dead_pid_is_not_live() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("1.lock"), LOCK).unwrap();
        let live = HashSet::from([99999u32]);
        assert!(!has_live_ide_lock(dir.path(), &live));
    }

    #[test]
    fn lock_with_running_pid_is_live() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("1.lock"), LOCK).unwrap();
        let live = HashSet::from([11472u32]);
        assert!(has_live_ide_lock(dir.path(), &live));
    }

    #[test]
    fn many_stale_locks_and_one_live_is_live() {
        let dir = tempdir().unwrap();
        for i in 0..8 {
            std::fs::write(
                dir.path().join(format!("{i}.lock")),
                LOCK.replace("11472", &format!("{}", 1000 + i)),
            ).unwrap();
        }
        std::fs::write(dir.path().join("live.lock"), LOCK).unwrap();
        assert!(has_live_ide_lock(dir.path(), &HashSet::from([11472u32])));
    }

    #[test]
    fn missing_directory_is_not_live() {
        assert!(!has_live_ide_lock(Path::new("/nonexistent"), &HashSet::new()));
    }

    #[test]
    fn non_lock_files_are_ignored() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), LOCK).unwrap();
        assert!(!has_live_ide_lock(dir.path(), &HashSet::from([11472u32])));
    }

    #[test]
    fn claude_process_activates_claude_layer_only() {
        let a = resolve(&["claude.exe".into()], false);
        assert!(a.claude && !a.openai);
    }

    #[test]
    fn chatgpt_process_activates_openai_layer_only() {
        let a = resolve(&["ChatGPT.exe".into()], false);
        assert!(a.openai && !a.claude);
    }

    #[test]
    fn live_lock_activates_claude_without_any_process() {
        let a = resolve(&[], true);
        assert!(a.claude);
    }

    #[test]
    fn no_signals_activates_nothing() {
        assert_eq!(resolve(&[], false), ActiveSources::default());
    }
}
```

- [ ] **Step 2: Register the module**

Add to `src-tauri/src/lib.rs`:

```rust
pub mod detect;
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test detect::`
Expected: 10 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/detect.rs src-tauri/src/lib.rs
git commit -m "feat: add process and IDE lock detection with live-PID validation"
```

### Task 4.2: Live process enumeration

**Files:**
- Modify: `src-tauri/src/detect.rs`

- [ ] **Step 1: Add the sysinfo-backed scan**

Append to `src-tauri/src/detect.rs`:

```rust
use sysinfo::System;

/// Snapshot of currently running process names and PIDs.
pub fn scan_processes(sys: &mut System) -> (Vec<String>, HashSet<u32>) {
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut names = Vec::new();
    let mut pids = HashSet::new();
    for (pid, proc_) in sys.processes() {
        names.push(proc_.name().to_string_lossy().to_string());
        pids.insert(pid.as_u32());
    }
    (names, pids)
}
```

- [ ] **Step 2: Write an integration test**

Append to the `tests` module in `src-tauri/src/detect.rs`:

```rust
    #[test]
    fn scan_finds_the_current_process() {
        let mut sys = System::new();
        let (names, pids) = scan_processes(&mut sys);
        assert!(!names.is_empty());
        assert!(pids.contains(&std::process::id()));
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test detect::`
Expected: 11 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/detect.rs
git commit -m "feat: add live process enumeration via sysinfo"
```

---

## Phase 5: Poller and Tauri Wiring

### Task 5.1: Snapshot freshness logic

**Files:**
- Create: `src-tauri/src/poller.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/poller.rs`:

```rust
use crate::model::{SnapshotState, UsageSnapshot};

/// A snapshot older than 3 polling intervals is downgraded to Stale.
pub fn age_state(snap: &UsageSnapshot, now: i64, poll_interval: i64) -> SnapshotState {
    if snap.state == SnapshotState::Error {
        return SnapshotState::Error;
    }
    if now - snap.fetched_at > poll_interval * 3 {
        SnapshotState::Stale
    } else {
        snap.state
    }
}

/// Highest used percentage across all windows, for the bubble view.
pub fn worst_percent(snaps: &[&UsageSnapshot]) -> Option<f32> {
    snaps
        .iter()
        .flat_map(|s| s.windows.iter())
        .map(|w| w.used_percent)
        .fold(None, |acc: Option<f32>, p| Some(acc.map_or(p, |a| a.max(p))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::UsageWindow;

    fn snap(state: SnapshotState, fetched_at: i64, pcts: &[f32]) -> UsageSnapshot {
        UsageSnapshot {
            windows: pcts.iter().map(|p| UsageWindow {
                label: "w".into(), used_percent: *p, resets_at: 0,
            }).collect(),
            fetched_at,
            state,
        }
    }

    #[test]
    fn recent_snapshot_stays_fresh() {
        let s = snap(SnapshotState::Fresh, 1000, &[10.0]);
        assert_eq!(age_state(&s, 1030, 60), SnapshotState::Fresh);
    }

    #[test]
    fn old_snapshot_becomes_stale() {
        let s = snap(SnapshotState::Fresh, 1000, &[10.0]);
        assert_eq!(age_state(&s, 1500, 60), SnapshotState::Stale);
    }

    #[test]
    fn error_state_is_never_downgraded() {
        let s = snap(SnapshotState::Error, 1000, &[]);
        assert_eq!(age_state(&s, 1500, 60), SnapshotState::Error);
    }

    #[test]
    fn worst_percent_spans_providers() {
        let a = snap(SnapshotState::Fresh, 0, &[10.0, 20.0]);
        let b = snap(SnapshotState::Fresh, 0, &[55.0]);
        assert_eq!(worst_percent(&[&a, &b]), Some(55.0));
    }

    #[test]
    fn worst_percent_of_no_windows_is_none() {
        let a = snap(SnapshotState::Fresh, 0, &[]);
        assert_eq!(worst_percent(&[&a]), None);
    }
}
```

- [ ] **Step 2: Register the module**

Add to `src-tauri/src/lib.rs`:

```rust
pub mod poller;
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test poller::`
Expected: 5 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/poller.rs src-tauri/src/lib.rs
git commit -m "feat: add snapshot ageing and worst-percent aggregation"
```

### Task 5.2: Tauri commands and event loop

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Wire the runtime**

Replace the body of `src-tauri/src/lib.rs`'s `run()` with:

```rust
pub mod config;
pub mod creds;
pub mod detect;
pub mod model;
pub mod poller;
pub mod providers;
pub mod window;

use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[derive(Default)]
pub struct AppState {
    pub visible: Mutex<bool>,
}

#[tauri::command]
fn get_config(app: tauri::AppHandle) -> config::Config {
    let path = app.path().app_config_dir().unwrap().join("config.json");
    config::Config::load(&path).sanitized()
}

#[tauri::command]
fn set_config(app: tauri::AppHandle, cfg: config::Config) -> Result<(), String> {
    let path = app.path().app_config_dir().unwrap().join("config.json");
    cfg.sanitized().save(&path).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![get_config, set_config])
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut sys = sysinfo::System::new();
                let lock_dir = dirs_claude_ide();
                loop {
                    let (names, pids) = detect::scan_processes(&mut sys);
                    let live = detect::has_live_ide_lock(&lock_dir, &pids);
                    let active = detect::resolve(&names, live);
                    let _ = handle.emit("sources-changed", active);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn dirs_claude_ide() -> std::path::PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.home_dir().join(".claude").join("ide"))
        .unwrap_or_default()
}
```

Add `Serialize` to `ActiveSources` in `detect.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct ActiveSources {
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: `Finished` with no errors.

- [ ] **Step 3: Run the app**

Run: `npm run tauri dev`
Expected: a window opens; the console logs `sources-changed` events every 5 seconds.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/
git commit -m "feat: wire detection loop and config commands into Tauri runtime"
```

---

## Phase 6: Window Placement

### Task 6.1: Corner math and monitor fallback

**Files:**
- Create: `src-tauri/src/window.rs`

This implements the monitor rule: anchor to the preferred monitor, fall back to the same
corner elsewhere when it is unplugged, and return automatically when it reconnects.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/window.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorInfo {
    pub id: String,
    pub area: Rect,
}

pub const MARGIN: i32 = 12;

/// Top-left position for a window of `size` in `corner` of `area`.
pub fn corner_position(area: Rect, size: (u32, u32), corner: &str) -> (i32, i32) {
    let (w, h) = (size.0 as i32, size.1 as i32);
    let (l, t) = (area.x + MARGIN, area.y + MARGIN);
    let r = area.x + area.w as i32 - w - MARGIN;
    let b = area.y + area.h as i32 - h - MARGIN;
    match corner {
        "top-left" => (l, t),
        "top-right" => (r, t),
        "bottom-left" => (l, b),
        _ => (r, b),
    }
}

/// Chooses the monitor to anchor to. Prefers `preferred_id` when present,
/// otherwise the first available.
pub fn choose_monitor<'a>(monitors: &'a [MonitorInfo], preferred_id: Option<&str>) -> Option<&'a MonitorInfo> {
    if let Some(id) = preferred_id {
        if let Some(m) = monitors.iter().find(|m| m.id == id) {
            return Some(m);
        }
    }
    monitors.first()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect { Rect { x: 0, y: 0, w: 1920, h: 1080 } }

    fn monitors() -> Vec<MonitorInfo> {
        vec![
            MonitorInfo { id: "DISPLAY1".into(), area: area() },
            MonitorInfo { id: "DISPLAY2".into(), area: Rect { x: 1920, y: 0, w: 1920, h: 1080 } },
        ]
    }

    #[test]
    fn positions_in_bottom_right_by_default() {
        assert_eq!(corner_position(area(), (380, 380), "bottom-right"), (1528, 688));
    }

    #[test]
    fn positions_in_top_left() {
        assert_eq!(corner_position(area(), (380, 380), "top-left"), (12, 12));
    }

    #[test]
    fn respects_monitor_offset() {
        let a = Rect { x: 1920, y: 0, w: 1920, h: 1080 };
        assert_eq!(corner_position(a, (380, 380), "top-left"), (1932, 12));
    }

    #[test]
    fn picks_the_preferred_monitor() {
        let m = choose_monitor(&monitors(), Some("DISPLAY2")).unwrap();
        assert_eq!(m.id, "DISPLAY2");
    }

    #[test]
    fn falls_back_when_preferred_monitor_is_unplugged() {
        let only_one = vec![monitors()[0].clone()];
        let m = choose_monitor(&only_one, Some("DISPLAY2")).unwrap();
        assert_eq!(m.id, "DISPLAY1");
    }

    #[test]
    fn returns_to_preferred_monitor_when_reconnected() {
        let m = choose_monitor(&monitors(), Some("DISPLAY2")).unwrap();
        assert_eq!(m.id, "DISPLAY2");
    }

    #[test]
    fn no_monitors_yields_none() {
        assert!(choose_monitor(&[], Some("DISPLAY1")).is_none());
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test window::`
Expected: 7 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/window.rs
git commit -m "feat: add corner placement math with monitor fallback and return"
```

### Task 6.2: Apply placement and window flags

**Files:**
- Modify: `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`

- [ ] **Step 1: Configure the window**

In `src-tauri/tauri.conf.json`, set the main window to:

```json
{
  "label": "main",
  "width": 320,
  "height": 200,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "resizable": false,
  "visible": false,
  "maxWidth": 380,
  "maxHeight": 380
}
```

`maxWidth`/`maxHeight` make fullscreen structurally impossible, per spec §6.

- [ ] **Step 2: Add the placement command**

Add to `src-tauri/src/lib.rs`:

```rust
#[tauri::command]
fn apply_placement(app: tauri::AppHandle, corner: String, preferred: Option<String>) -> Result<(), String> {
    let win = app.get_webview_window("main").ok_or("no main window")?;
    let monitors: Vec<window::MonitorInfo> = win
        .available_monitors().map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| window::MonitorInfo {
            id: m.name().cloned().unwrap_or_default(),
            area: window::Rect {
                x: m.position().x, y: m.position().y,
                w: m.size().width, h: m.size().height,
            },
        })
        .collect();
    let chosen = window::choose_monitor(&monitors, preferred.as_deref())
        .ok_or("no monitors available")?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let (x, y) = window::corner_position(chosen.area, (size.width, size.height), &corner);
    win.set_position(tauri::PhysicalPosition { x, y }).map_err(|e| e.to_string())
}
```

Register it in `invoke_handler`:

```rust
.invoke_handler(tauri::generate_handler![get_config, set_config, apply_placement])
```

- [ ] **Step 3: Verify manually**

Run: `npm run tauri dev`
Expected: the window is frameless, always on top, absent from the taskbar and alt-tab, and
cannot be resized beyond 380x380.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/
git commit -m "feat: apply corner placement and frameless always-on-top window flags"
```

---

## Phase 7: User Interface

### Task 7.1: Formatting helpers

**Files:**
- Create: `src/format.ts`, `src/format.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/format.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { formatPercent, formatReset, formatAge } from "./format";

describe("formatPercent", () => {
  it("rounds to a whole number", () => {
    expect(formatPercent(25.4)).toBe("25%");
  });
  it("renders zero without hiding it", () => {
    expect(formatPercent(0)).toBe("0%");
  });
});

describe("formatReset", () => {
  const now = 1_000_000;
  it("renders minutes under an hour", () => {
    expect(formatReset(now + 1800, now)).toBe("resets in 30m");
  });
  it("renders hours under a day", () => {
    expect(formatReset(now + 7200, now)).toBe("resets in 2h");
  });
  it("renders days beyond 24 hours", () => {
    expect(formatReset(now + 259200, now)).toBe("resets in 3d");
  });
  it("reports an elapsed reset as due", () => {
    expect(formatReset(now - 10, now)).toBe("resetting");
  });
});

describe("formatAge", () => {
  const now = 1_000_000;
  it("treats the last minute as just now", () => {
    expect(formatAge(now - 5, now)).toBe("just now");
  });
  it("renders whole minutes", () => {
    expect(formatAge(now - 120, now)).toBe("2m ago");
  });
  it("renders hours", () => {
    expect(formatAge(now - 7200, now)).toBe("2h ago");
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npm test -- format`
Expected: FAIL, cannot resolve `./format`.

- [ ] **Step 3: Implement**

Create `src/format.ts`:

```typescript
export function formatPercent(p: number): string {
  return `${Math.round(p)}%`;
}

export function formatReset(resetsAt: number, now: number): string {
  const delta = resetsAt - now;
  if (delta <= 0) return "resetting";
  if (delta < 3600) return `resets in ${Math.round(delta / 60)}m`;
  if (delta < 86400) return `resets in ${Math.round(delta / 3600)}h`;
  return `resets in ${Math.round(delta / 86400)}d`;
}

export function formatAge(fetchedAt: number, now: number): string {
  const delta = now - fetchedAt;
  if (delta < 60) return "just now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  return `${Math.floor(delta / 3600)}h ago`;
}
```

- [ ] **Step 4: Run tests**

Run: `npm test -- format`
Expected: 9 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/format.ts src/format.test.ts
git commit -m "feat: add percent, reset, and age formatting helpers"
```

### Task 7.2: Size-state machine

**Files:**
- Create: `src/state.ts`, `src/state.test.ts`, `src/types.ts`

- [ ] **Step 1: Define shared types**

Create `src/types.ts`:

```typescript
export type SnapshotState = "fresh" | "stale" | "error";
export type SizeState = "bubble" | "compact" | "square";

export interface UsageWindow {
  label: string;
  used_percent: number;
  resets_at: number;
}

export interface UsageSnapshot {
  windows: UsageWindow[];
  fetched_at: number;
  state: SnapshotState;
}

export interface ActiveSources {
  claude: boolean;
  openai: boolean;
}
```

- [ ] **Step 2: Write the failing test**

Create `src/state.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { nextSize, worstPercent, visibleLayers } from "./state";
import type { UsageSnapshot } from "./types";

const snap = (pcts: number[]): UsageSnapshot => ({
  windows: pcts.map((p) => ({ label: "w", used_percent: p, resets_at: 0 })),
  fetched_at: 0,
  state: "fresh",
});

describe("nextSize", () => {
  it("cycles compact to square", () => {
    expect(nextSize("compact")).toBe("square");
  });
  it("cycles square back to compact", () => {
    expect(nextSize("square")).toBe("compact");
  });
  it("restores bubble to compact", () => {
    expect(nextSize("bubble")).toBe("compact");
  });
});

describe("worstPercent", () => {
  it("takes the maximum across providers", () => {
    expect(worstPercent([snap([10, 20]), snap([55])])).toBe(55);
  });
  it("returns null when there are no windows", () => {
    expect(worstPercent([snap([])])).toBeNull();
  });
});

describe("visibleLayers", () => {
  it("shows only claude when only claude is active", () => {
    expect(visibleLayers({ claude: true, openai: false })).toEqual(["claude"]);
  });
  it("shows only openai when only openai is active", () => {
    expect(visibleLayers({ claude: false, openai: true })).toEqual(["openai"]);
  });
  it("shows both when both are active", () => {
    expect(visibleLayers({ claude: true, openai: true })).toEqual(["claude", "openai"]);
  });
  it("shows nothing when neither is active", () => {
    expect(visibleLayers({ claude: false, openai: false })).toEqual([]);
  });
});
```

- [ ] **Step 3: Run to verify failure**

Run: `npm test -- state`
Expected: FAIL, cannot resolve `./state`.

- [ ] **Step 4: Implement**

Create `src/state.ts`:

```typescript
import type { ActiveSources, SizeState, UsageSnapshot } from "./types";

export function nextSize(current: SizeState): SizeState {
  return current === "compact" ? "square" : "compact";
}

export function worstPercent(snaps: UsageSnapshot[]): number | null {
  const all = snaps.flatMap((s) => s.windows.map((w) => w.used_percent));
  return all.length ? Math.max(...all) : null;
}

export function visibleLayers(sources: ActiveSources): Array<"claude" | "openai"> {
  const out: Array<"claude" | "openai"> = [];
  if (sources.claude) out.push("claude");
  if (sources.openai) out.push("openai");
  return out;
}
```

- [ ] **Step 5: Run tests**

Run: `npm test -- state`
Expected: 9 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/state.ts src/state.test.ts src/types.ts
git commit -m "feat: add size-state machine and layer visibility logic"
```

### Task 7.3: Layer component with accessibility

**Files:**
- Create: `src/components/layer.ts`, `src/components/layer.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/components/layer.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { renderLayer } from "./layer";
import type { UsageSnapshot } from "../types";

const snap: UsageSnapshot = {
  windows: [
    { label: "5 hour", used_percent: 12, resets_at: 1_003_600 },
    { label: "Weekly", used_percent: 48, resets_at: 1_259_200 },
  ],
  fetched_at: 999_940,
  state: "fresh",
};

describe("renderLayer", () => {
  it("renders one row per window", () => {
    const el = renderLayer("Claude", snap, 1_000_000);
    expect(el.querySelectorAll('[role="progressbar"]')).toHaveLength(2);
  });

  it("gives each bar an accessible value and description", () => {
    const el = renderLayer("Claude", snap, 1_000_000);
    const bar = el.querySelector('[role="progressbar"]')!;
    expect(bar.getAttribute("aria-valuenow")).toBe("12");
    expect(bar.getAttribute("aria-valuemin")).toBe("0");
    expect(bar.getAttribute("aria-valuemax")).toBe("100");
    expect(bar.getAttribute("aria-valuetext")).toBe("12 percent used, resets in 1h");
  });

  it("renders a zero-percent window rather than hiding it", () => {
    const zero: UsageSnapshot = {
      ...snap,
      windows: [{ label: "5 hour", used_percent: 0, resets_at: 1_003_600 }],
    };
    const el = renderLayer("Claude", zero, 1_000_000);
    expect(el.querySelectorAll('[role="progressbar"]')).toHaveLength(1);
    expect(el.textContent).toContain("0%");
  });

  it("shows a no-window message when the provider reports none", () => {
    const empty: UsageSnapshot = { ...snap, windows: [] };
    const el = renderLayer("Claude", empty, 1_000_000);
    expect(el.textContent).toContain("No active window");
  });

  it("marks the layer stale without blanking values", () => {
    const el = renderLayer("Claude", { ...snap, state: "stale" }, 1_000_000);
    expect(el.dataset.state).toBe("stale");
    expect(el.textContent).toContain("48%");
  });

  it("shows a re-auth hint in the error state", () => {
    const el = renderLayer("Claude", { ...snap, state: "error" }, 1_000_000);
    expect(el.textContent).toContain("Re-authenticate");
  });
});
```

- [ ] **Step 2: Configure jsdom**

Create `vitest.config.ts`:

```typescript
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    coverage: { provider: "v8", thresholds: { lines: 80, functions: 80, branches: 80 } },
  },
});
```

- [ ] **Step 3: Run to verify failure**

Run: `npm test -- layer`
Expected: FAIL, cannot resolve `./layer`.

- [ ] **Step 4: Implement**

Create `src/components/layer.ts`:

```typescript
import { formatAge, formatPercent, formatReset } from "../format";
import type { UsageSnapshot } from "../types";

export function renderLayer(name: string, snap: UsageSnapshot, now: number): HTMLElement {
  const root = document.createElement("section");
  root.className = "layer";
  root.dataset.state = snap.state;
  root.setAttribute("aria-labelledby", `layer-${name.toLowerCase()}`);

  const title = document.createElement("h2");
  title.id = `layer-${name.toLowerCase()}`;
  title.className = "layer__title";
  title.textContent = name;
  root.appendChild(title);

  if (snap.windows.length === 0) {
    const empty = document.createElement("p");
    empty.className = "layer__empty";
    empty.textContent = "No active window";
    root.appendChild(empty);
  }

  for (const w of snap.windows) {
    const row = document.createElement("div");
    row.className = "window-row";

    const label = document.createElement("span");
    label.className = "window-row__label";
    label.textContent = w.label;

    const value = document.createElement("span");
    value.className = "window-row__value";
    value.textContent = formatPercent(w.used_percent);

    const bar = document.createElement("div");
    bar.className = "bar";
    bar.setAttribute("role", "progressbar");
    bar.setAttribute("aria-valuenow", String(Math.round(w.used_percent)));
    bar.setAttribute("aria-valuemin", "0");
    bar.setAttribute("aria-valuemax", "100");
    bar.setAttribute(
      "aria-valuetext",
      `${Math.round(w.used_percent)} percent used, ${formatReset(w.resets_at, now)}`,
    );

    const fill = document.createElement("div");
    fill.className = "bar__fill";
    fill.style.width = `${Math.min(100, w.used_percent)}%`;
    bar.appendChild(fill);

    const reset = document.createElement("span");
    reset.className = "window-row__reset";
    reset.textContent = formatReset(w.resets_at, now);

    row.append(label, value, bar, reset);
    root.appendChild(row);
  }

  if (snap.state === "error") {
    const hint = document.createElement("p");
    hint.className = "layer__hint";
    hint.textContent = "Re-authenticate in the CLI";
    root.appendChild(hint);
  }

  const foot = document.createElement("p");
  foot.className = "layer__age";
  foot.textContent = `Updated ${formatAge(snap.fetched_at, now)}`;
  root.appendChild(foot);

  return root;
}
```

- [ ] **Step 5: Run tests**

Run: `npm test -- layer`
Expected: 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/components/layer.ts src/components/layer.test.ts vitest.config.ts
git commit -m "feat: add accessible layer component with progressbar semantics"
```

### Task 7.4: Styles, themes, and responsiveness

**Files:**
- Create: `src/styles/tokens.css`, `src/styles/app.css`

- [ ] **Step 1: Write the tokens**

Create `src/styles/tokens.css`:

```css
:root {
  --bg: oklch(98% 0 0);
  --surface: oklch(100% 0 0);
  --text: oklch(20% 0 0);
  --text-muted: oklch(45% 0 0);
  --border: oklch(88% 0 0);
  --ok: oklch(62% 0.16 150);
  --warn: oklch(72% 0.17 75);
  --crit: oklch(58% 0.21 25);
  --radius: 10px;
  --space: 8px;
  --duration: 180ms;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg: oklch(18% 0 0);
    --surface: oklch(23% 0 0);
    --text: oklch(96% 0 0);
    --text-muted: oklch(70% 0 0);
    --border: oklch(32% 0 0);
  }
}

@media (prefers-reduced-motion: reduce) {
  * { animation: none !important; transition: none !important; }
}
```

- [ ] **Step 2: Write the layout**

Create `src/styles/app.css`:

```css
@import "./tokens.css";

body {
  margin: 0;
  font: 13px/1.4 system-ui, sans-serif;
  color: var(--text);
  background: transparent;
}

#app {
  container-type: inline-size;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space);
  display: flex;
  flex-direction: column;
  gap: var(--space);
}

.layer { display: flex; flex-direction: column; gap: 4px; }
.layer[data-state="stale"] { opacity: 0.6; }
.layer[data-state="error"] { border-left: 2px solid var(--crit); padding-left: 6px; }

.layer__title { font-size: 12px; font-weight: 600; margin: 0; }
.layer__age, .layer__empty, .layer__hint {
  font-size: 11px; color: var(--text-muted); margin: 0;
}

.window-row {
  display: grid;
  grid-template-columns: auto 1fr;
  grid-template-areas: "label value" "bar bar" "reset reset";
  gap: 2px 6px;
  align-items: center;
}
.window-row__label { grid-area: label; color: var(--text-muted); }
.window-row__value { grid-area: value; justify-self: end; font-variant-numeric: tabular-nums; }
.window-row__reset { grid-area: reset; font-size: 11px; color: var(--text-muted); }

.bar {
  grid-area: bar;
  height: 6px;
  background: var(--border);
  border-radius: 3px;
  overflow: hidden;
}
.bar__fill { height: 100%; background: var(--ok); transition: width var(--duration); }
.bar[aria-valuenow^="8"] .bar__fill,
.bar[aria-valuenow^="9"] .bar__fill { background: var(--warn); }

/* Bubble state collapses everything to a single puck. */
[data-size="bubble"] #app { border-radius: 50%; padding: 0; }
[data-size="bubble"] .layer__title,
[data-size="bubble"] .window-row,
[data-size="bubble"] .layer__age { display: none; }

/* Container query, not viewport: the window resizes independently of the screen. */
@container (min-width: 340px) {
  .window-row {
    grid-template-columns: 70px 1fr auto auto;
    grid-template-areas: "label bar value reset";
  }
}

button:focus-visible {
  outline: 2px solid var(--text);
  outline-offset: 2px;
}
```

- [ ] **Step 3: Verify visually**

Run: `npm run tauri dev`
Expected: rows render, bars fill proportionally, and the layout switches to a single line
when the window is widened past 340px.

- [ ] **Step 4: Commit**

```bash
git add src/styles/
git commit -m "feat: add design tokens, themes, and container-query layout"
```

### Task 7.5: Accessibility audit

**Files:**
- Create: `src/a11y.test.ts`

- [ ] **Step 1: Write the audit test**

Create `src/a11y.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { axe } from "vitest-axe";
import { renderLayer } from "./components/layer";
import type { UsageSnapshot } from "./types";

const snap: UsageSnapshot = {
  windows: [
    { label: "5 hour", used_percent: 12, resets_at: 1_003_600 },
    { label: "Weekly", used_percent: 48, resets_at: 1_259_200 },
  ],
  fetched_at: 999_940,
  state: "fresh",
};

describe("accessibility", () => {
  for (const state of ["fresh", "stale", "error"] as const) {
    it(`has no violations in the ${state} state`, async () => {
      const host = document.createElement("main");
      host.appendChild(renderLayer("Claude", { ...snap, state }, 1_000_000));
      document.body.appendChild(host);
      expect(await axe(host)).toHaveNoViolations();
    });
  }

  it("has no violations when a provider reports no windows", async () => {
    const host = document.createElement("main");
    host.appendChild(renderLayer("Codex", { ...snap, windows: [] }, 1_000_000));
    document.body.appendChild(host);
    expect(await axe(host)).toHaveNoViolations();
  });
});
```

- [ ] **Step 2: Run tests**

Run: `npm test -- a11y`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/a11y.test.ts
git commit -m "test: add axe accessibility audit across layer states"
```

---

## Phase 8: Tray, Lifecycle, and Settings

### Task 8.1: Tray icon with show/hide and exit

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the tray**

Add to the `setup` closure in `src-tauri/src/lib.rs`, before `Ok(())`:

```rust
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;

let toggle = MenuItem::with_id(app, "toggle", "Show/Hide", true, None::<&str>)?;
let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
let menu = Menu::with_items(app, &[&toggle, &quit])?;

TrayIconBuilder::new()
    .icon(app.default_window_icon().unwrap().clone())
    .menu(&menu)
    .on_menu_event(|app, event| match event.id.as_ref() {
        "toggle" => {
            if let Some(w) = app.get_webview_window("main") {
                if w.is_visible().unwrap_or(false) {
                    let _ = w.hide();
                } else {
                    let _ = w.show();
                }
            }
        }
        "quit" => app.exit(0),
        _ => {}
    })
    .build(app)?;
```

- [ ] **Step 2: Verify manually**

Run: `npm run tauri dev`
Expected: a tray icon appears; "Show/Hide" toggles the window and "Quit" exits.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add tray icon with show/hide toggle and quit"
```

### Task 8.2: Visibility-gated polling

**Files:**
- Modify: `src-tauri/src/lib.rs`

Polling must stop while hidden, per spec §5, and fetch immediately on becoming visible
rather than waiting a full interval.

- [ ] **Step 1: Add the usage loop**

Add a second spawned task inside `setup`:

```rust
let usage_handle = app.handle().clone();
tauri::async_runtime::spawn(async move {
    loop {
        let visible = usage_handle
            .get_webview_window("main")
            .and_then(|w| w.is_visible().ok())
            .unwrap_or(false);

        if visible {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            if let Ok(tok) = creds::read_token(&claude_creds_path(), creds::claude_token_from_str) {
                let r = providers::fetch_json(
                    "https://api.anthropic.com/api/oauth/usage",
                    &tok,
                    &[("anthropic-beta", "oauth-2025-04-20")],
                ).await;
                let snap = match r {
                    Ok(v) => providers::claude::parse_usage(&v, now, model::SnapshotState::Fresh),
                    Err(e) => model::UsageSnapshot {
                        windows: vec![], fetched_at: now,
                        state: providers::state_for_error(&e),
                    },
                };
                let _ = usage_handle.emit("claude-usage", snap);
            }

            if let Ok(tok) = creds::read_token(&codex_auth_path(), creds::codex_token_from_str) {
                let r = providers::fetch_json(
                    "https://chatgpt.com/backend-api/api/codex/usage", &tok, &[],
                ).await;
                let snap = match r {
                    Ok(v) => providers::codex::parse_rate_limits(&v, now, model::SnapshotState::Fresh),
                    Err(e) => model::UsageSnapshot {
                        windows: vec![], fetched_at: now,
                        state: providers::state_for_error(&e),
                    },
                };
                let _ = usage_handle.emit("codex-usage", snap);
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
});
```

Add the path helpers:

```rust
fn home() -> std::path::PathBuf {
    directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf()).unwrap_or_default()
}
fn claude_creds_path() -> std::path::PathBuf { home().join(".claude").join(".credentials.json") }
fn codex_auth_path() -> std::path::PathBuf { home().join(".codex").join("auth.json") }
```

- [ ] **Step 2: Verify**

Run: `npm run tauri dev`
Expected: usage events arrive while visible and stop while hidden.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add visibility-gated usage polling for both providers"
```

### Task 8.3: Settings panel

**Files:**
- Create: `src/components/controls.ts`, `src/components/controls.test.ts`

- [ ] **Step 1: Write the failing test**

Create `src/components/controls.test.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { renderControls } from "./controls";

describe("renderControls", () => {
  it("renders every control as a real button", () => {
    const el = renderControls({ sizeState: "compact", alwaysOnTop: true }, vi.fn());
    const buttons = el.querySelectorAll("button");
    expect(buttons.length).toBeGreaterThanOrEqual(3);
    buttons.forEach((b) => expect(b.getAttribute("aria-label")).toBeTruthy());
  });

  it("reports the always-on-top state to assistive tech", () => {
    const el = renderControls({ sizeState: "compact", alwaysOnTop: true }, vi.fn());
    const pin = el.querySelector('[data-action="pin"]')!;
    expect(pin.getAttribute("aria-pressed")).toBe("true");
  });

  it("emits the action when a control is activated", () => {
    const onAction = vi.fn();
    const el = renderControls({ sizeState: "compact", alwaysOnTop: false }, onAction);
    el.querySelector<HTMLButtonElement>('[data-action="bubble"]')!.click();
    expect(onAction).toHaveBeenCalledWith("bubble");
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npm test -- controls`
Expected: FAIL, cannot resolve `./controls`.

- [ ] **Step 3: Implement**

Create `src/components/controls.ts`:

```typescript
import type { SizeState } from "../types";

export interface ControlsState {
  sizeState: SizeState;
  alwaysOnTop: boolean;
}

export type ControlAction = "bubble" | "resize" | "pin" | "settings";

export function renderControls(
  state: ControlsState,
  onAction: (a: ControlAction) => void,
): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "controls";
  bar.setAttribute("role", "toolbar");
  bar.setAttribute("aria-label", "Overlay controls");

  const defs: Array<[ControlAction, string, string]> = [
    ["bubble", "Minimize to bubble", "\u2013"],
    ["resize", state.sizeState === "square" ? "Shrink panel" : "Expand panel", "\u25A1"],
    ["pin", "Always on top", "\u25C9"],
    ["settings", "Settings", "\u2699"],
  ];

  for (const [action, label, glyph] of defs) {
    const b = document.createElement("button");
    b.type = "button";
    b.dataset.action = action;
    b.setAttribute("aria-label", label);
    if (action === "pin") b.setAttribute("aria-pressed", String(state.alwaysOnTop));
    b.textContent = glyph;
    b.addEventListener("click", () => onAction(action));
    bar.appendChild(b);
  }

  return bar;
}
```

- [ ] **Step 4: Run tests**

Run: `npm test -- controls`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/controls.ts src/components/controls.test.ts
git commit -m "feat: add accessible custom window controls"
```

---

## Phase 9: Packaging

### Task 9.1: Coverage gate and full suite

- [ ] **Step 1: Run the complete suite**

Run: `npm run coverage`
Expected: all tests pass, coverage at or above 80% for lines, functions, and branches.

Run: `cd src-tauri && cargo test`
Expected: all tests pass.

- [ ] **Step 2: Fix any shortfall**

If coverage is below 80%, add tests for the uncovered branches. Do not lower the threshold.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "test: bring coverage to the 80 percent threshold"
```

### Task 9.2: Build the installer

- [ ] **Step 1: Build**

Run: `npm run tauri build`
Expected: an `.exe` and `.msi` under `src-tauri/target/release/bundle/`.

- [ ] **Step 2: Verify the built app**

Launch the built executable. Confirm: it starts hidden with a tray icon, appears when
Claude or ChatGPT is running, hides when both close, is absent from the taskbar and
alt-tab, stays on top, and restores its corner and monitor after a restart.

- [ ] **Step 3: Add run-at-login**

Add `tauri-plugin-autostart` to `Cargo.toml` and register it:

```rust
.plugin(tauri_plugin_autostart::init(
    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
    None,
))
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add autostart and produce release bundle"
```

### Task 9.3: README

- [ ] **Step 1: Write the README**

Create `README.md` covering: what the app does, the ChatGPT-app limitation from spec §3.3,
prerequisites, `npm run tauri dev`, `npm run tauri build`, where config is stored, and how
to reset it.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with setup and usage"
```

---

## Self-Review

**Spec coverage:**

| Spec section | Task |
|---|---|
| §2 verification | 0.1 |
| §3.1 Claude source | 3.3, 8.2 |
| §3.2 Codex source + JSONL fallback | 3.2, 8.2 |
| §3.3 ChatGPT limitation | 9.3 (documented) |
| §3.4 normalized model | 2.1 |
| §3.5 window rendering rule | 3.2, 3.3, 7.3 |
| §3.6 polling and quota | 0.1 |
| §4 detection | 4.1, 4.2 |
| §5 lifecycle | 8.1, 8.2 |
| §6 size states and placement | 6.1, 6.2, 7.4 |
| §7 configuration | 2.2, 5.2 |
| §8 security | 3.1 |
| §9 error handling | 3.4, 7.3 |
| §10 accessibility | 7.3, 7.5, 8.3 |
| §11 responsiveness | 7.4 |
| §12 testing | every task |

No gaps.

**Placeholder scan:** No TBDs. Every code step contains complete, runnable code.

**Type consistency:** `UsageSnapshot`, `UsageWindow`, `SnapshotState`, and `ActiveSources`
are defined once in `model.rs`/`detect.rs` and mirrored in `types.ts` with matching
snake_case field names, since serde serializes them unchanged across the Tauri boundary.

**Known follow-up:** Task 3.3 and 8.2 assume Claude's field names (`five_hour`,
`seven_day`, `utilization`) and the `anthropic-beta` header. Phase 0 confirms these; adjust
both tasks before implementing if the real payload differs.
