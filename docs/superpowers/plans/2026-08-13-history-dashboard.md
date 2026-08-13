# History Dashboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist single-account usage and cost samples locally and provide an interactive, exportable history and billing dashboard.

**Architecture:** Rust owns a versioned SQLite database in `app_data_dir`, records only successful provider samples, applies retention, aggregates chart/billing queries, and streams exports to user-selected paths. A dedicated React history window requests typed query results; it never reads credentials or SQLite directly.

**Tech Stack:** Tauri 2, Rust 2021, rusqlite (bundled SQLite), serde/serde_json, csv, React + TypeScript (from the React-rendering foundation), Vitest/jsdom, CSS/SVG

---

## Prerequisites and file map

- Branch after merging `codex/feature-react-rendering-foundation`, `codex/feature-claude-usage-v2`, and `codex/feature-console-costs`; the history schema records their normalized per-model, Extra, prepaid-credit, and API-cost DTOs and must not invent parallel versions of those types.
- Add `rusqlite = { version = "0.32", features = ["bundled"] }` and `csv = "1.3"` to `src-tauri/Cargo.toml`; run `cargo update -p rusqlite -p csv` so `src-tauri/Cargo.lock` is committed.
- Create `src-tauri/src/history.rs`: schema, inserts, retention, range queries and billing aggregation.
- Create `src-tauri/src/export.rs`: JSON/CSV serialization and atomic file writes.
- Modify `src-tauri/src/lib.rs`: managed database, commands, poll-cycle recording, history-window opening.
- Modify `src-tauri/tauri.conf.json`: hidden resizable `history` window (960×680, minimum 760×520).
- Modify `src-tauri/capabilities/settings.json`: include `history` in the existing trusted settings capability; exports remain Rust commands, so no filesystem plugin permission is added.
- Create `src/history/{types,range,HistoryApp,HistoryChart,BillingTable,ExportControls}.tsx` and tests: typed UI boundary, UTC range math, accessible SVG chart, billing table and export actions.
- Modify `src/main.ts`, `src/types.ts`, `src/styles/app.css`, and settings navigation to mount/open the history window.

### Task 1: Create the durable SQLite schema

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Create: `src-tauri/src/history.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing migration test**

```rust
#[test]
fn migration_creates_versioned_sample_and_cost_schema() {
    let db = HistoryDb::open_in_memory().unwrap();
    let names: Vec<String> = db.connection().prepare(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
    ).unwrap().query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect();
    assert!(names.contains(&"usage_samples".into()));
    assert!(names.contains(&"billing_entries".into()));
    assert_eq!(db.connection().pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0)).unwrap(), 1);
}
```

- [ ] **Step 2: Run the focused test and verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml history::tests::migration_creates_versioned_sample_and_cost_schema -- --exact`
Expected: FAIL with unresolved module/type `HistoryDb`.

- [ ] **Step 3: Add dependencies, module declaration, and the complete v1 migration**

```rust
// src-tauri/src/history.rs
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

const SCHEMA_V1: &str = r#"
BEGIN;
CREATE TABLE usage_samples (
  id INTEGER PRIMARY KEY,
  provider TEXT NOT NULL CHECK(provider IN ('claude','openai')),
  window_kind TEXT NOT NULL,
  used_percent REAL NOT NULL CHECK(used_percent >= 0 AND used_percent <= 100),
  resets_at INTEGER NOT NULL,
  sampled_at INTEGER NOT NULL,
  session_id TEXT,
  model TEXT,
  api_calls INTEGER,
  input_tokens INTEGER,
  output_tokens INTEGER,
  estimated_cost_micros INTEGER,
  overage_cost_micros INTEGER,
  UNIQUE(provider, window_kind, sampled_at)
);
CREATE INDEX usage_samples_range ON usage_samples(sampled_at, provider, window_kind);
CREATE TABLE billing_entries (
  id INTEGER PRIMARY KEY,
  provider TEXT NOT NULL CHECK(provider IN ('claude','openai')),
  period_start INTEGER NOT NULL,
  period_end INTEGER NOT NULL,
  amount_micros INTEGER NOT NULL CHECK(amount_micros >= 0),
  currency TEXT NOT NULL DEFAULT 'USD',
  source TEXT NOT NULL CHECK(source IN ('estimated','provider')),
  UNIQUE(provider, period_start, period_end, source)
);
PRAGMA user_version = 1;
COMMIT;
"#;

pub struct HistoryDb { connection: Connection }
impl HistoryDb {
    pub fn open(path: &Path) -> rusqlite::Result<Self> { let c = Connection::open(path)?; c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?; c.execute_batch(SCHEMA_V1)?; Ok(Self { connection: c }) }
    #[cfg(test)] pub fn open_in_memory() -> rusqlite::Result<Self> { let c = Connection::open_in_memory()?; c.execute_batch(SCHEMA_V1)?; Ok(Self { connection: c }) }
    #[cfg(test)] fn connection(&self) -> &Connection { &self.connection }
}
```

Guard `SCHEMA_V1` with `if user_version == 0` before executing it, and reject versions greater than `1` with `rusqlite::Error::InvalidQuery`. In `lib.rs`, add `pub mod history;`, create `<app_data_dir>/history.sqlite3` in setup, and manage `Mutex<HistoryDb>`.

- [ ] **Step 4: Run database tests and verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml history::tests -- --nocapture`
Expected: PASS; migration is idempotent and rejects a future schema version.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/history.rs src-tauri/src/lib.rs
git commit -m "feat(history): add local usage database"
```

### Task 2: Record normalized samples without inventing unavailable cost data

**Files:**
- Modify: `src-tauri/src/history.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/model.rs`

- [ ] **Step 1: Write failing insertion tests**

```rust
#[test]
fn records_fresh_windows_once_and_leaves_unknown_costs_null() {
    let mut db = HistoryDb::open_in_memory().unwrap();
    let event = ProviderUsageEvent { provider: Provider::Claude, snapshot: UsageSnapshot {
        windows: vec![UsageWindow { label: "5 hour".into(), used_percent: 25.0, resets_at: 2000 }],
        fetched_at: 1000, state: SnapshotState::Fresh,
    }};
    assert_eq!(db.record_event(&event).unwrap(), 1);
    assert_eq!(db.record_event(&event).unwrap(), 0);
    let row: (String, Option<i64>) = db.connection.query_row(
      "SELECT window_kind, estimated_cost_micros FROM usage_samples", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    assert_eq!(row, ("session_5h".into(), None));
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml history::tests::records_fresh_windows_once_and_leaves_unknown_costs_null -- --exact`
Expected: FAIL because `record_event` is missing.

- [ ] **Step 3: Implement normalization and transactional inserts**

```rust
fn window_kind(label: &str) -> String {
    match label.trim().to_ascii_lowercase().as_str() {
        "5 hour" => "session_5h".into(), "daily" | "24 hour" => "daily_24h".into(),
        "weekly" | "7 days" => "weekly_7d".into(), other => format!("provider:{}", other.replace(' ', "_")),
    }
}
pub fn record_event(&mut self, event: &ProviderUsageEvent) -> rusqlite::Result<usize> {
    if event.snapshot.state != SnapshotState::Fresh { return Ok(0); }
    let provider = match event.provider { Provider::Claude => "claude", Provider::Openai => "openai" };
    let tx = self.connection.transaction()?;
    let mut inserted = 0;
    for w in &event.snapshot.windows {
        inserted += tx.execute("INSERT OR IGNORE INTO usage_samples
          (provider,window_kind,used_percent,resets_at,sampled_at) VALUES (?1,?2,?3,?4,?5)",
          params![provider, window_kind(&w.label), w.used_percent, w.resets_at, event.snapshot.fetched_at])?;
    }
    tx.commit()?; Ok(inserted)
}
```

Call `record_event` immediately after a cycle is accepted as complete and before `cache_usage`; log a database failure through `report_diagnostic` without withholding live usage. Session/model/API/token/cost columns are nullable because current provider payloads do not supply them; future enrichment writes real values only, never estimates tokens from percentages. Billing rows likewise accept provider-reported or explicitly calculated entries only.

- [ ] **Step 4: Verify green and regression suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml history::tests && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/history.rs src-tauri/src/lib.rs src-tauri/src/model.rs
git commit -m "feat(history): persist successful usage samples"
```

### Task 3: Add ranges, retention, and aggregate query commands

**Files:**
- Modify: `src-tauri/src/history.rs`
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Write failing boundary tests**

```rust
#[test]
fn query_is_half_open_and_retention_keeps_the_cutoff() {
    let mut db = seeded_db(&[100, 200, 300]);
    assert_eq!(db.query(HistoryQuery { from: 200, to: 301, provider: None, window_kind: None }).unwrap().points.len(), 2);
    assert_eq!(db.prune_before(200).unwrap(), 1);
    assert_eq!(db.query(HistoryQuery { from: 0, to: 1000, provider: None, window_kind: None }).unwrap().points.len(), 2);
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml history::tests::query_is_half_open_and_retention_keeps_the_cutoff -- --exact`
Expected: FAIL with missing query types/functions.

- [ ] **Step 3: Implement typed queries and retention**

```rust
#[derive(Debug, Deserialize)] #[serde(rename_all="camelCase", deny_unknown_fields)]
pub struct HistoryQuery { pub from: i64, pub to: i64, pub provider: Option<String>, pub window_kind: Option<String> }
#[derive(Debug, Serialize)] #[serde(rename_all="camelCase")]
pub struct HistoryPoint { pub provider: String, pub window_kind: String, pub sampled_at: i64, pub used_percent: f32, pub model: Option<String>, pub api_calls: Option<i64>, pub estimated_cost_micros: Option<i64>, pub overage_cost_micros: Option<i64> }
#[derive(Debug, Serialize)] #[serde(rename_all="camelCase")]
pub struct HistoryResult { pub points: Vec<HistoryPoint>, pub billing: Vec<BillingEntry> }
```

Validate `from < to`, max span 366 days, providers `claude|openai`, and known/provider-prefixed window kinds. Query with bound parameters and `sampled_at >= ? AND sampled_at < ? ORDER BY sampled_at,id`; billing overlaps when `period_end > from AND period_start < to`. Add `history_retention_days: u16` to Config, default 180, sanitized to `30..=730`; prune once at startup and after each successful insert with cutoff `now - days*86400`. Add commands `query_history(query) -> Result<HistoryResult,String>` and `clear_history()`, registering both in `generate_handler!`.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml history::tests && npm test -- --run src/state.test.ts`
Expected: PASS, including config migration from files missing `historyRetentionDays`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/history.rs src-tauri/src/config.rs src-tauri/src/lib.rs src/types.ts
git commit -m "feat(history): query ranges and enforce retention"
```

### Task 4: Build range selection and accessible interactive SVG charts

**Files:**
- Create: `src/history/types.ts`
- Create: `src/history/range.ts`
- Create: `src/history/range.test.ts`
- Create: `src/history/HistoryChart.tsx`
- Create: `src/history/HistoryChart.test.tsx`

- [ ] **Step 1: Write failing range/chart tests**

```tsx
it.each([["5h",18_000],["24h",86_400],["7d",604_800],["30d",2_592_000]])("maps %s to seconds", (range, seconds) => {
  expect(historyBounds(range as HistoryRange, 3_000_000)).toEqual({ from: 3_000_000-seconds, to: 3_000_001 });
});
it("exposes points to keyboard and tooltip text", () => {
  render(<HistoryChart points={[point]} selectedSeries="session_5h" />);
  expect(screen.getByRole("img", {name:/5 hour usage history/i})).toBeInTheDocument();
  fireEvent.focus(screen.getByRole("button", {name:/25%/i}));
  expect(screen.getByRole("status")).toHaveTextContent("Claude, 25%");
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- --run src/history/range.test.ts src/history/HistoryChart.test.tsx`
Expected: FAIL because the modules do not exist.

- [ ] **Step 3: Implement exact range API and chart contract**

```ts
export type HistoryRange = "5h" | "24h" | "7d" | "30d";
const SECONDS: Record<HistoryRange, number> = { "5h": 18_000, "24h": 86_400, "7d": 604_800, "30d": 2_592_000 };
export const historyBounds = (range: HistoryRange, now: number) => ({ from: now - SECONDS[range], to: now + 1 });
```

`HistoryChart` groups points by `provider/windowKind`, maps time linearly to x and percent `0..100` to y, renders one labeled `<svg role="img">`, a `<path>` per series, and transparent 24px focusable `<button>` overlays per point. Pointer move/focus selects the nearest point and updates a persistent `role="status"`; arrow keys move within the same series. Render explicit empty-state copy when no points exist and use CSS variables for series colors.

- [ ] **Step 4: Verify green**

Run: `npm test -- --run src/history/range.test.ts src/history/HistoryChart.test.tsx`
Expected: all tests PASS with no axe violations.

- [ ] **Step 5: Commit**

```bash
git add src/history/types.ts src/history/range.ts src/history/range.test.ts src/history/HistoryChart.tsx src/history/HistoryChart.test.tsx
git commit -m "feat(history): add interactive range charts"
```

### Task 5: Assemble dashboard, per-model/API/overage summaries, and billing history

**Files:**
- Create: `src/history/HistoryApp.tsx`
- Create: `src/history/HistoryApp.test.tsx`
- Create: `src/history/BillingTable.tsx`
- Modify: `src/main.ts`
- Modify: `src/components/settings.tsx`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/settings.json`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing integration tests**

```tsx
it("reloads on range and series changes and keeps unavailable metrics honest", async () => {
  invoke.mockResolvedValue({ points: [point], billing: [] });
  render(<HistoryApp now={() => 3_000_000} />);
  await userEvent.click(screen.getByRole("button", {name:"30 days"}));
  expect(invoke).toHaveBeenLastCalledWith("query_history", { query: expect.objectContaining({from: 408_000,to: 3_000_001}) });
  expect(screen.getByText("Per-model data unavailable")).toBeInTheDocument();
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- --run src/history/HistoryApp.test.tsx`
Expected: FAIL because `HistoryApp` is missing.

- [ ] **Step 3: Implement the dashboard**

Mount by window label `history`. Add range tabs 5h/24h/7d/30d; provider and series filters; summary cards for session, weekly, API calls, estimated cost, and overage; a per-model table grouped only from non-null `model`; and `BillingTable` columns provider, UTC period, source, amount/currency. Null metrics display “Unavailable from provider” and never `0`. Add `open_history_window` using the existing repair/show/focus pattern, a Settings “History” page/button, and the configured hidden history window. Query cancellation uses an incrementing request id so stale responses cannot replace a newer range.

- [ ] **Step 4: Verify green and build**

Run: `npm test -- --run src/history/HistoryApp.test.tsx && npm run build`
Expected: PASS; TypeScript and Vite build succeed.

- [ ] **Step 5: Commit**

```bash
git add src/history src/main.ts src/components/settings.tsx src-tauri/tauri.conf.json src-tauri/capabilities/settings.json src-tauri/src/lib.rs
git commit -m "feat(history): add dashboard and billing view"
```

### Task 6: Add deterministic JSON/CSV export and confirmed clearing

**Files:**
- Create: `src-tauri/src/export.rs`
- Modify: `src-tauri/src/history.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src/history/ExportControls.tsx`
- Create: `src/history/ExportControls.test.tsx`

- [ ] **Step 1: Write failing serializer tests**

```rust
#[test] fn csv_has_stable_columns_and_blank_nulls() {
  let text = history_csv(&fixture()).unwrap();
  assert_eq!(text.lines().next().unwrap(), "provider,window_kind,sampled_at,used_percent,model,api_calls,estimated_cost_micros,overage_cost_micros");
  assert!(text.lines().any(|line| line == "claude,session_5h,1000,25,,,,"));
}
#[test] fn json_is_versioned() {
  let value: serde_json::Value = serde_json::from_slice(&history_json(&fixture()).unwrap()).unwrap();
  assert_eq!(value["schemaVersion"], 1);
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml export::tests -- --nocapture`
Expected: FAIL because export serializers are absent.

- [ ] **Step 3: Implement serializers and safe command boundary**

JSON shape is `{ "schemaVersion":1, "exportedAt":<unix>, "query":{...}, "points":[...], "billing":[...] }`. CSV uses one RFC-4180 row per point with the exact tested header and blank nullable cells; billing CSV is a second file named `<stem>-billing.csv`. `export_history(query, format, destination)` accepts only `json|csv`, rejects directories and nonmatching `.json|.csv`, writes a sibling `.tmp`, `sync_all`, then renames. The React controls use native save-path input supplied by the existing UI flow, show completion/error text, and require a two-step confirmation (`Clear history` then `Confirm clear`) before invoking `clear_history`; Escape/cancel resets confirmation.

- [ ] **Step 4: Verify all suites**

Run: `cargo test --manifest-path src-tauri/Cargo.toml && npm test && npm run build`
Expected: all Rust/Vitest tests PASS and production build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/export.rs src-tauri/src/history.rs src-tauri/src/lib.rs src/history/ExportControls.tsx src/history/ExportControls.test.tsx
git commit -m "feat(history): export and clear local history"
```

### Task 7: Final verification and documentation

**Files:**
- Modify: `README.md`

- [ ] Document that data is single-account, local-only at `app_data_dir/history.sqlite3`, retained 180 days by default, and exports may contain sensitive usage/cost metadata.
- [ ] Run: `npm test && npm run build && cargo fmt --manifest-path src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: every command exits 0.
- [ ] Commit:

```bash
git add README.md
git commit -m "docs(history): explain retention and exports"
```
