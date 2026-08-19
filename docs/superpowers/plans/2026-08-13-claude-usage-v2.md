# Claude Usage V2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show schema-tolerant per-model Claude limits, Claude Extra spending/budget/balance, rate-limit information retained from 429 responses, explicit freshness/error states, and live Claude service status.

**Architecture:** Replace label-only usage windows with a typed Claude usage domain that preserves unknown model identifiers and partial data. A schema-gated adapter parses only fields observed in verified Claude responses/headers; polling retains last-good sections independently. Service health comes from Atlassian Statuspage's public status API and never shares Claude credentials.

**Tech Stack:** Rust, serde/serde_json, reqwest, Tauri events, React + TypeScript, Vitest, mockito.

---

## Prerequisites and source policy

- Start after `codex/feature-react-rendering-foundation` and `codex/feature-secure-auth`.
- Claude.ai usage endpoints and payloads are undocumented. Before implementation, capture redacted real fixtures under `src-tauri/tests/fixtures/claude/`; the parser must be driven by those fixtures. Never invent a `limits[]` member name. This plan uses normalized app-owned names (`model_key`, `display_name`, `utilization_percent`, `resets_at`) only after an adapter maps a verified source field.
- Opus, Sonnet, Design, and Fable are display labels, not an exhaustive enum. Unknown future entries remain visible with the provider label/key.
- Claude Extra monetary fields are optional and independently available. Store integer minor units plus ISO currency; do not use floating-point money.
- Parse standard/provider rate-limit headers from 429 responses only when their exact names and formats exist in captured fixtures. Preserve unknown headers for neither telemetry nor UI.
- Fetch status from `https://status.claude.com/api/v2/status.json` and `.../summary.json`, the public Statuspage API. No authentication/cookies.

## File structure

- Create `src-tauri/src/providers/claude_usage.rs`: normalized types and verified response/header adapters.
- Create `src-tauri/src/providers/claude_status.rs`: Statuspage client/parser.
- Modify `src-tauri/src/model.rs`: provider-specific details on snapshots.
- Modify `src-tauri/src/providers/mod.rs`: response headers and partial errors.
- Modify `src-tauri/src/lib.rs`: independent usage/status polling and caching.
- Modify `src/types.ts`: Claude detail DTOs.
- Create `src/components/claude/ClaudeUsageDetails.tsx` and tests.
- Create `src/components/claude/ClaudeServiceStatus.tsx` and tests.
- Modify `src/components/ProviderCard.tsx`: mount Claude details.
- Create redacted fixtures only from verified responses; do not check in tokens, IDs, cookies, or full headers.

### Task 1: Model dynamic limits, money, and section freshness

**Files:**
- Modify: `src-tauri/src/model.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Write failing round-trip tests**

Require `ClaudeUsageDetails { limits: DataSection<Vec<ClaudeModelLimit>>, extra: DataSection<ClaudeExtra> }`, `ClaudeModelLimit { model_key, display_name, utilization_percent, resets_at }`, `Money { minor_units: i64, currency }`, and `DataSection<T> { value, fetched_at, state, error_code }`. Assert unknown `claude-next-x` survives JSON round-trip.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml model::tests::claude_details_preserve_unknown_models`

Expected: FAIL with missing `ClaudeUsageDetails`.

- [ ] **Step 3: Implement normalized types**

Add `ProviderDetails::Claude(ClaudeUsageDetails)` as optional `details` on `UsageSnapshot` so existing providers remain compatible. Define section states `Fresh`, `Stale`, `Unavailable`, `Error`; allow last-good value with stale/error metadata.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml model::tests && npm test -- --run src/types.test.ts`

Expected: PASS with Rust/TypeScript camelCase agreement.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/model.rs src/types.ts src/types.test.ts
git commit -m "feat(claude): model dynamic usage details"
```

### Task 2: Capture and enforce verified usage fixtures

**Files:**
- Create: `src-tauri/tests/fixtures/claude/README.md`
- Create: `src-tauri/tests/fixtures/claude/usage-success.json`
- Create: `src-tauri/tests/fixtures/claude/usage-partial.json`
- Create: `src-tauri/tests/fixtures/claude/usage-429-headers.json`
- Create: `src-tauri/src/providers/claude_usage.rs`

- [ ] **Step 1: Document fixture provenance and redaction**

The README must record capture date, account feature set, endpoint path, HTTP status, which field names were observed, and redactions. Replace values, never keys; omit all authorization/cookie/request-ID headers. If a required surface cannot be captured, mark that section unavailable in production and omit its parser rather than fabricating a fixture.

- [ ] **Step 2: Write failing fixture parser tests**

Tests load each committed fixture, parse observed `limits[]` entries in source order, map known display labels without filtering, reject non-array limits, reject NaN/out-of-range utilization, parse RFC3339/epoch reset forms only if observed, and parse Extra money into minor units without float arithmetic.

- [ ] **Step 3: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::claude_usage::tests`

Expected: FAIL because the adapter is absent (or because fixtures have not yet been responsibly captured; do not proceed until they are).

- [ ] **Step 4: Implement the fixture-driven adapter**

Define private serde source structs using the exact captured keys and `deny_unknown_fields` only at stable leaf objects. Convert to normalized types; assign known labels through `known_model_label(key)`, default to the non-empty source label and then key. Parse each limits/Extra section independently so one malformed section does not erase the other.

- [ ] **Step 5: Verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::claude_usage::tests`

Expected: PASS for success, partial, malformed, unknown-model, and integer-money cases.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tests/fixtures/claude src-tauri/src/providers/claude_usage.rs src-tauri/src/providers/mod.rs
git commit -m "feat(claude): parse verified usage v2 payloads"
```

### Task 3: Retain usage information from 429 headers

**Files:**
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/providers/claude_usage.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing response/header tests**

Extend `FetchResponse` with an allowlisted, case-insensitive header map. Using the verified 429 fixture, assert `parse_rate_limit_headers(status, headers, now)` returns only fields actually present, clamps no invalid values, handles duplicate/missing headers as malformed/absent, and marks header-derived data stale rather than fresh.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::claude_usage::tests::parses_verified_429_headers`

Expected: FAIL because response headers are discarded.

- [ ] **Step 3: Implement allowlisted capture and merge**

Capture only verified usage header names into `FetchResponse`. On 429, parse body and headers before status classification; merge by stable model key, prefer a valid body field over its equivalent header, and retain last-good values for absent sections. Do not expose raw headers through IPC or diagnostics.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::claude_usage::tests lib::tests::claude_429`

Expected: PASS; a bodyless 429 retains previous values and any verified header update, state `Stale`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/providers/mod.rs src-tauri/src/providers/claude_usage.rs src-tauri/src/lib.rs
git commit -m "fix(claude): preserve 429 usage headers"
```

### Task 4: Preserve last-good sections and explicit failures

**Files:**
- Modify: `src-tauri/src/poller.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing section merge tests**

Test: fresh limits + malformed Extra; fresh Extra + missing limits; network error with prior data; first-load network error; 401 clears no numbers but sets `NeedsReauthentication`; 404/feature absent is `Unavailable`; schema drift is `Error`; stale timestamp ages independently.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml poller::tests::merge_claude_sections`

Expected: FAIL because current retention operates on the whole snapshot.

- [ ] **Step 3: Implement `merge_data_section` and polling integration**

Merge each section independently, cap user-facing error codes to `unauthorized`, `rate-limited`, `network`, `contract-changed`, and `feature-unavailable`, and keep detailed errors only in redacted diagnostics. A successful empty verified `limits[]` is fresh empty data, not malformed.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml poller::tests lib::tests::claude_usage_v2`

Expected: PASS for the full state matrix.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/poller.rs src-tauri/src/lib.rs
git commit -m "feat(claude): retain usage sections independently"
```

### Task 5: Add live Claude service status

**Files:**
- Create: `src-tauri/src/providers/claude_status.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing Statuspage parser/client tests**

Use mockito fixtures matching Statuspage `status.indicator`, `status.description`, components, and unresolved incidents from `/api/v2/summary.json`. Test unknown indicators, timeout, 5xx, malformed payload, incident URL allowlisting to `status.claude.com`, and stale last-good retention.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::claude_status::tests`

Expected: FAIL because `ClaudeServiceStatus` is absent.

- [ ] **Step 3: Implement unauthenticated status client**

Fetch summary on a separate five-minute interval with a ten-second timeout and fixed User-Agent. Normalize indicator to `Operational`, `Degraded`, `PartialOutage`, `MajorOutage`, or `Unknown`; include provider description and active incident summaries only. Never send Anthropic authorization or cookies to the status domain.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml providers::claude_status::tests`

Expected: PASS; mock asserts absence of `Authorization` and `Cookie`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/providers/claude_status.rs src-tauri/src/providers/mod.rs src-tauri/src/lib.rs
git commit -m "feat(claude): fetch live service status"
```

### Task 6: Render dynamic models, Extra, freshness, and status

**Files:**
- Create: `src/components/claude/ClaudeUsageDetails.tsx`
- Create: `src/components/claude/ClaudeUsageDetails.test.tsx`
- Create: `src/components/claude/ClaudeServiceStatus.tsx`
- Create: `src/components/claude/ClaudeServiceStatus.test.tsx`
- Modify: `src/components/ProviderCard.tsx`
- Modify: `src/styles/app.css`

- [ ] **Step 1: Write failing UI tests**

Render Opus/Sonnet/Design/Fable plus unknown future model; current spend/budget/balance with `Intl.NumberFormat`; partial Extra without invented zeroes; stale badges with timestamps; unavailable and error copy; 429 data; all five service states and incident links. Run axe for each partial-state fixture.

- [ ] **Step 2: Verify RED**

Run: `npm test -- --run src/components/claude/ClaudeUsageDetails.test.tsx src/components/claude/ClaudeServiceStatus.test.tsx`

Expected: FAIL because components do not exist.

- [ ] **Step 3: Implement components**

Key rows by `modelKey`, preserve source order, display all entries, clamp only CSS meter width while announcing the real validated percent, omit absent monetary rows, and render section-specific state copy. Service indicator uses text plus icon, not color alone; external incident links use `rel="noreferrer"`.

- [ ] **Step 4: Verify GREEN**

Run: `npm test -- --run src/components/claude && npm run build`

Expected: component tests PASS with no axe violations; TypeScript/Vite build exits 0.

- [ ] **Step 5: Commit**

```bash
git add src/components/claude src/components/ProviderCard.tsx src/styles/app.css
git commit -m "feat(claude): render usage v2 and service health"
```

### Task 7: Full regression and contract-drift verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Document sources and limitations**

Explain that Claude usage/Extra data is best-effort from an undocumented contract, status is public Statuspage data, unknown models display automatically, and unavailable means the provider did not expose a verified field—not zero usage or balance.

- [ ] **Step 2: Run complete verification**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml && npm test && npm run build`

Expected: all commands exit 0; malformed-fixture and secret-header regression tests PASS.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: explain Claude usage data sources"
```

