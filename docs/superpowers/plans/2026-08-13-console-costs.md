# Anthropic Console Costs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an Anthropic Console dashboard for current-month API spend, prepaid credit balance, and daily/per-key/per-model breakdowns whenever the authenticated Console surface exposes them, with honest partial and unavailable states.

**Architecture:** A Console billing adapter consumes only verified, redacted response fixtures and normalizes monetary values to integer minor units. Each dashboard section has independent provenance/freshness so missing permissions or unsupported breakdown dimensions do not erase available totals. Console authentication remains completely separate from Claude.ai subscription usage.

**Tech Stack:** Rust, serde, reqwest, Tauri commands/events, React + TypeScript, Vitest, mockito.

---

## Prerequisites and source policy

- Start after `codex/feature-react-rendering-foundation`, `codex/feature-secure-auth`, and `codex/feature-claude-usage-v2` (for `DataSection`, money formatting, and stale-state conventions).
- Anthropic's public API documents usage/cost reporting for eligible organization/admin credentials, while consumer Console web endpoints and prepaid-balance surfaces may be undocumented or permission-gated. Use an official documented API where the account credential and endpoint support it. Any Console-web adapter must be fixture-driven, isolated, labeled undocumented, and disabled when its contract is unverified.
- Do not claim prepaid balance can be derived as budget minus spend. Balance, monthly spend, and budget are distinct optional facts.
- Never invent response field names. Before adding a parser, save a redacted fixture that retains original keys and record endpoint, status, credential type, required role, API version/beta headers, capture date, and redactions.
- Currency amounts use integer minor units plus ISO 4217 currency. Reject mixed-currency aggregation; display separate totals if a verified source returns multiple currencies.

## File structure

- Create `src-tauri/src/providers/console_costs.rs`: source DTO adapters and normalized aggregation.
- Create `src-tauri/src/providers/console_client.rs`: documented and optional fixture-gated requests.
- Create `src-tauri/tests/fixtures/console/README.md` and verified redacted fixtures.
- Modify `src-tauri/src/model.rs`: Console cost dashboard DTOs.
- Modify `src-tauri/src/lib.rs`: commands, cache, polling, and account selection.
- Modify `src/types.ts`: dashboard types.
- Create `src/console-costs-api.ts`: typed invoke API.
- Create `src/components/console/ConsoleCostsDashboard.tsx` and tests.
- Create `src/components/console/CostBreakdownTable.tsx` and tests.
- Modify `src/components/SettingsApp.tsx`: Console dashboard route/account prompt.

### Task 1: Define money-safe dashboard contracts

**Files:**
- Modify: `src-tauri/src/model.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Write failing serialization tests**

Require `ConsoleCostsDashboard { period, spend, prepaid_balance, daily, by_api_key, by_model }`, `CostPeriod { starts_at, ends_at, timezone }`, `CostPoint { key, label, amount }`, and independent `DataSection<T>`. Test a value larger than JavaScript's safe integer serializes `minorUnits` as a decimal string.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml model::tests::console_money_serializes_losslessly`

Expected: FAIL because Console cost types are missing.

- [ ] **Step 3: Implement contracts**

Use Rust `i128` internally and serialize decimal strings; TypeScript uses branded `MoneyMinorUnits = string`. `CostPoint.key` is a stable opaque source identifier, `label` may be redacted (“Key …A1B2”), and all section values are optional. Add `UnavailableReason::{NoCredential,InsufficientRole,UnsupportedBySource,ProviderUnavailable}`.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml model::tests && npm test -- --run src/types.test.ts`

Expected: PASS with exact Rust/TypeScript JSON agreement.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/model.rs src/types.ts src/types.test.ts
git commit -m "feat(console): define cost dashboard contracts"
```

### Task 2: Capture verified billing fixtures and implement exact adapters

**Files:**
- Create: `src-tauri/tests/fixtures/console/README.md`
- Create: `src-tauri/tests/fixtures/console/cost-report.json`
- Create: `src-tauri/tests/fixtures/console/credit-balance.json`
- Create: `src-tauri/tests/fixtures/console/partial-permissions.json`
- Create: `src-tauri/src/providers/console_costs.rs`

- [ ] **Step 1: Record authoritative contract metadata**

For every fixture, record the official documentation URL if public, endpoint path (no host secrets/query identifiers), method, API version/beta headers, credential type, organization role, pagination semantics, status, and capture date. Redact values only. If prepaid balance or a breakdown dimension has no verified endpoint, omit that fixture and configure the corresponding adapter capability `false`.

- [ ] **Step 2: Write failing parser tests**

Load each available fixture and assert exact source-key parsing, decimal-to-minor-unit conversion, UTC date boundaries, pagination cursor handling, duplicate bucket aggregation, API-key label redaction, unknown-model preservation, mixed-currency rejection, negative/overflow rejection, and partial-permission mapping.

- [ ] **Step 3: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::console_costs::tests`

Expected: FAIL because `console_costs` is absent (or because required evidence is unavailable; leave that capability disabled rather than creating a fictional fixture).

- [ ] **Step 4: Implement fixture-driven adapters**

Create source-specific private serde structs with exact verified keys. Normalize through checked decimal arithmetic; aggregate by UTC day, opaque key ID, and model ID only where the source returns that dimension. `source_capabilities()` reports each surface independently and determines whether the client makes that request.

- [ ] **Step 5: Verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::console_costs::tests`

Expected: PASS for full, partial, malformed, paginated, overflow, and unsupported-capability fixtures.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tests/fixtures/console src-tauri/src/providers/console_costs.rs src-tauri/src/providers/mod.rs
git commit -m "feat(console): parse verified billing data"
```

### Task 3: Implement least-privilege Console client

**Files:**
- Create: `src-tauri/src/providers/console_client.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/auth/console.rs`

- [ ] **Step 1: Write failing HTTP contract tests**

For each enabled source capability, mock the exact documented/verified method, path, query, version headers, auth header, pagination cursor, timeout, and maximum pages. Test 401, 403, 404, 429 with Retry-After, 5xx, malformed content type, cross-origin redirect rejection, and response-size limit.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::console_client::tests`

Expected: FAIL because `ConsoleClient` does not exist.

- [ ] **Step 3: Implement client and credential validation**

Build a dedicated reqwest client with ten-second timeout, redirects disabled, fixed Anthropic API origin, 2 MiB response cap, and no cookies. Retrieve Console credentials just-in-time from `SecretStore`, apply only the exact verified auth/version headers, zeroize local copies, and validate a saved credential through the cheapest enabled read endpoint. Never send Claude.ai OAuth tokens to Console endpoints.

- [ ] **Step 4: Verify GREEN and secret redaction**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::console_client::tests auth::console::tests`

Expected: PASS; error/debug strings and diagnostics do not contain fixture key `sk-ant-admin-test-secret`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/providers/console_client.rs src-tauri/src/providers/mod.rs src-tauri/src/auth/console.rs
git commit -m "feat(console): add least-privilege billing client"
```

### Task 4: Fetch and merge partial dashboard sections

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/poller.rs`

- [ ] **Step 1: Write failing orchestration tests**

Test parallel independent requests; spend succeeds while balance is 403; balance succeeds while breakdown is unsupported; page two fails after page one; 429 retains last-good data and honors bounded Retry-After; switching Console accounts clears the prior account cache; first-load failure produces explicit unavailable sections.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml lib::tests::console_cost_cycle`

Expected: FAIL because no Console cost cycle exists.

- [ ] **Step 3: Implement `fetch_console_cost_cycle`**

Run only capability-enabled requests using `tokio::join!`, merge each `DataSection` independently, cache by non-secret account ID and calendar month, and emit `console-costs-changed`. Poll every fifteen minutes while the dashboard is open and on explicit refresh; pause when no Console account exists. Retry delays are capped and jittered without blocking Claude usage polling.

- [ ] **Step 4: Add Tauri commands**

Add `get_console_costs(account_id)`, `refresh_console_costs(account_id)`, and `select_console_account(account_id)`. Validate IDs against the secure account inventory; never accept endpoint URLs, header names, or raw credentials over these commands.

- [ ] **Step 5: Verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml lib::tests::console_cost_cycle`

Expected: PASS for all partial-state and account-isolation tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/poller.rs
git commit -m "feat(console): poll cost sections independently"
```

### Task 5: Add typed frontend API and dashboard state

**Files:**
- Create: `src/console-costs-api.ts`
- Create: `src/console-costs-api.test.ts`
- Create: `src/console-costs-state.ts`
- Create: `src/console-costs-state.test.ts`

- [ ] **Step 1: Write failing API/state tests**

Test bootstrap, event replacement only for matching account, refresh-in-flight deduplication, stale data retention, account switch reset, decimal-string money parsing with `BigInt`, and malformed IPC rejection without crashing settings.

- [ ] **Step 2: Verify RED**

Run: `npm test -- --run src/console-costs-api.test.ts src/console-costs-state.test.ts`

Expected: FAIL because modules do not exist.

- [ ] **Step 3: Implement API and reducer**

Wrap invokes/listen in `console-costs-api.ts`; validate discriminants and decimal strings at the boundary. Reducer actions are `loadStarted`, `snapshotReceived`, `loadFailed`, and `accountChanged`; retain section values supplied with stale/error state and never turn missing values into zero.

- [ ] **Step 4: Verify GREEN**

Run: `npm test -- --run src/console-costs-api.test.ts src/console-costs-state.test.ts`

Expected: PASS, including values above `Number.MAX_SAFE_INTEGER`.

- [ ] **Step 5: Commit**

```bash
git add src/console-costs-api.ts src/console-costs-api.test.ts src/console-costs-state.ts src/console-costs-state.test.ts
git commit -m "feat(console): add typed cost dashboard state"
```

### Task 6: Render totals and available breakdowns

**Files:**
- Create: `src/components/console/ConsoleCostsDashboard.tsx`
- Create: `src/components/console/ConsoleCostsDashboard.test.tsx`
- Create: `src/components/console/CostBreakdownTable.tsx`
- Create: `src/components/console/CostBreakdownTable.test.tsx`
- Modify: `src/components/SettingsApp.tsx`
- Modify: `src/styles/app.css`

- [ ] **Step 1: Write failing dashboard tests**

Cover current-month spend, balance, period/timezone, daily rows, per-key redacted labels, per-model unknown labels, separate currencies, no-credential prompt, insufficient-role copy, unsupported breakdown omission with explanation, partial success, stale timestamp, retry, loading, empty verified data, keyboard table navigation, and axe checks.

- [ ] **Step 2: Verify RED**

Run: `npm test -- --run src/components/console/ConsoleCostsDashboard.test.tsx src/components/console/CostBreakdownTable.test.tsx`

Expected: FAIL because components do not exist.

- [ ] **Step 3: Implement dashboard**

Use `Intl.NumberFormat` after converting minor units with currency exponent logic that supports zero-, two-, and three-decimal currencies without lossy total arithmetic. Render a total card only when that source value exists. Tables use semantic captions/headers, sort by amount descending only for display, retain opaque stable keys, and show per-section state adjacent to its heading.

- [ ] **Step 4: Integrate Console account selection**

Mount under a distinct “Console costs” settings page. If no verified credential is selected, link to the separate Console credential panel from secure-auth; never offer Claude.ai login as the remedy for missing Console access.

- [ ] **Step 5: Verify GREEN**

Run: `npm test -- --run src/components/console && npm run build`

Expected: tests PASS with no axe violations; build exits 0.

- [ ] **Step 6: Commit**

```bash
git add src/components/console src/components/SettingsApp.tsx src/styles/app.css
git commit -m "feat(console): render cost and credit dashboard"
```

### Task 7: Security, unavailability, and full regression verification

**Files:**
- Modify: `SECURITY.md`
- Modify: `README.md`

- [ ] **Step 1: Add end-to-end failure tests**

Add Rust tests proving cross-origin redirects are blocked, raw key IDs are redacted, secrets never enter events/cache/diagnostics, partial 403 does not suppress spend, unsupported endpoints are never requested, and mixed currencies are not summed. Add UI tests proving unavailable is not rendered as `$0.00`.

- [ ] **Step 2: Document behavior**

Document credential/role prerequisites, separate Claude.ai versus Console identities, refresh cadence, UTC month boundaries, source-specific availability, and the fact that prepaid credit/breakdowns appear only when an enabled verified provider contract exposes them.

- [ ] **Step 3: Run complete verification**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml && npm test && npm run build`

Expected: all commands exit 0; HTTP, money, partial-state, accessibility, and secret-redaction suites PASS.

- [ ] **Step 4: Commit**

```bash
git add SECURITY.md README.md src-tauri/src src/components/console
git commit -m "docs(console): document cost data boundaries"
```

