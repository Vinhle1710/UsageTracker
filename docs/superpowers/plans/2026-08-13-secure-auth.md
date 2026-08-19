# Secure Anthropic Authentication Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add automatic Claude Code account discovery, distinct Claude.ai and Anthropic Console sign-in paths, manual credential fallback, and Windows-native encrypted storage with a safe one-time migration.

**Architecture:** Introduce an `auth` module whose account inventory is independent from provider polling. Claude Code discovery imports credentials without altering the CLI file; app-owned secrets are stored through a `SecretStore` abstraction backed by Windows Credential Manager, with DPAPI-encrypted file fallback. OAuth attempts are short-lived in-memory state machines; the embedded webview only navigates an allowlisted Anthropic/Google authentication flow and never receives stored tokens.

**Tech Stack:** Rust, Tauri 2, reqwest/rustls, serde, Windows Credential Manager and DPAPI through `windows-sys`, React + TypeScript (from `codex/feature-react-rendering-foundation`), Vitest, Rust unit tests, mockito.

---

## Prerequisites and data-source decisions

- Start from `codex/feature-react-rendering-foundation`; merge this branch before the usage-v2 and Console-cost branches.
- Treat `~/.claude/.credentials.json` as a Claude Code-owned, undocumented local format. Read it defensively and never rewrite or delete it.
- Keep the existing Claude Code public-client PKCE flow (`claude.ai/oauth/authorize`, `platform.claude.com/v1/oauth/token`) behind `ClaudeAiOAuthClient`; these are not a public third-party integration contract. Put paths and accepted response fields in one module, reject schema drift, and show a manual fallback instead of guessing.
- “Google SSO support” means allowing the user to choose Google on Anthropic's own login page. The app must not request Google credentials, inject scripts, inspect page DOM, or implement a Google OAuth client.
- Anthropic Console is a separate account kind and secret namespace. Do not reuse Claude.ai cookies/tokens or imply that a Claude.ai subscription grants Console access.
- Manual fallback accepts a user-supplied bearer/API credential only into app-owned secure storage. It never writes plaintext into config, logs, diagnostics, frontend state, or Claude Code files.

## File structure

- Create `src-tauri/src/auth/mod.rs`: account types, source priority, redacted summaries.
- Create `src-tauri/src/auth/discovery.rs`: read-only Claude Code credential discovery.
- Create `src-tauri/src/auth/oauth.rs`: PKCE attempt lifecycle and allowlisted navigation policy.
- Create `src-tauri/src/auth/secret_store.rs`: `SecretStore` trait and migration coordinator.
- Create `src-tauri/src/auth/windows.rs`: Credential Manager primary backend and DPAPI fallback.
- Create `src-tauri/src/auth/console.rs`: validation of manually supplied Console credentials.
- Modify `src-tauri/src/creds.rs`: retain parsers but remove app-owned writes to Claude Code files.
- Modify `src-tauri/src/lib.rs`: Tauri auth commands and managed auth state.
- Modify `src-tauri/src/providers/claude.rs`: consume resolved credentials rather than paths.
- Modify `src-tauri/src/providers/mod.rs`: add non-secret authentication errors.
- Modify `src-tauri/Cargo.toml`: Windows credential/cryptography features and zeroization.
- Modify `src/types.ts`: discriminated auth DTOs.
- Create `src/auth-api.ts`: typed invoke boundary.
- Create `src/components/settings/AnthropicAccounts.tsx` and test: discovery, login, manual fallback UI.
- Modify `src/components/SettingsApp.tsx`: mount account settings.
- Modify `src-tauri/capabilities/settings.json`: allow only the new commands in the settings window.

### Task 1: Define account and secret contracts

**Files:**
- Create: `src-tauri/src/auth/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Write failing Rust serialization tests**

Add tests requiring `AccountKind::{ClaudeAi,AnthropicConsole}`, `CredentialSource::{ClaudeCode,SecureStore,Manual}`, and `AccountSummary { id, kind, source, email, status }`; assert serialized output contains no token-like fields.

```rust
let value = serde_json::to_value(AccountSummary::signed_in(
    "claude-code:local", AccountKind::ClaudeAi, CredentialSource::ClaudeCode,
    Some("person@example.com".into()),
)).unwrap();
assert_eq!(value["kind"], "claude-ai");
assert!(value.get("accessToken").is_none());
assert!(value.get("secret").is_none());
```

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::tests::account_summary_never_serializes_secrets`

Expected: FAIL with unresolved module `auth`.

- [ ] **Step 3: Implement the exact contracts**

Define serde kebab-case enums, `AccountStatus::{SignedIn,NeedsReauthentication,Unavailable}`, and `SecretRecord { account_id, kind, secret: zeroize::Zeroizing<String>, expires_at }`. Only `AccountSummary` crosses IPC. Add `mod auth;` to `lib.rs` and matching TypeScript unions.

- [ ] **Step 4: Verify GREEN**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::tests && npm test -- --run src/types.test.ts`

Expected: Rust tests PASS; Vitest confirms representative DTOs narrow by `kind`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/auth/mod.rs src-tauri/src/lib.rs src/types.ts src/types.test.ts
git commit -m "feat(auth): define Anthropic account contracts"
```

### Task 2: Discover Claude Code credentials without taking ownership

**Files:**
- Create: `src-tauri/src/auth/discovery.rs`
- Modify: `src-tauri/src/creds.rs`
- Test: inline Rust tests in both files

- [ ] **Step 1: Write failing discovery tests**

Use `tempfile` fixtures for missing, malformed, valid `claudeAiOauth`, expired, and legacy top-level credentials. Assert `discover_claude_code(path, now)` returns `NotInstalled`, `Invalid`, `SignedIn`, or `Expired`, never mutates bytes, and uses SHA-256 of the canonical path as the stable account suffix rather than token material.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::discovery::tests`

Expected: FAIL because `discover_claude_code` does not exist.

- [ ] **Step 3: Implement read-only discovery**

Add `ClaudeCodeDiscovery` and `discover_claude_code(path, now_ms)`. Reuse `claude_oauth_from_str`; record only expiry and optional organization UUID in the inventory. Deprecate `persist_claude_login`, `persist_claude_refresh`, and `logout_claude` for app-owned login paths; leave compatibility tests until migration is complete.

- [ ] **Step 4: Verify GREEN and immutability**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::discovery::tests creds::tests`

Expected: PASS, including byte-for-byte before/after fixture equality.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/auth/discovery.rs src-tauri/src/auth/mod.rs src-tauri/src/creds.rs
git commit -m "feat(auth): discover Claude Code sessions read-only"
```

### Task 3: Add Windows secure storage and migration

**Files:**
- Create: `src-tauri/src/auth/secret_store.rs`
- Create: `src-tauri/src/auth/windows.rs`
- Modify: `src-tauri/Cargo.toml`
- Test: inline Rust tests with an in-memory backend

- [ ] **Step 1: Write failing backend-contract and migration tests**

Define a fake store and test `put/get/delete`, target names `UsageTracker/anthropic/<kind>/<account-id>`, maximum-length rejection, zeroization wrapper use, and `migrate_legacy_secret`. Migration must write, read-back verify, then remove only the migrated plaintext key; failed verification leaves the source intact; reruns are idempotent.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::secret_store::tests`

Expected: FAIL with missing `SecretStore`.

- [ ] **Step 3: Implement storage backends**

Add `zeroize = "1"`. Enable `Win32_Security_Credentials`, `Win32_Security_Cryptography`, and `Win32_System_Memory`. Implement `CredentialManagerStore` using generic credentials with local-machine persistence. If Credential Manager returns a platform-unavailable error, use `CryptProtectData`/`CryptUnprotectData` with entropy `UsageTracker:anthropic:v1`, store ciphertext only under the app data directory, use atomic replacement, and ACL-compatible user-only location. Never silently fall back for access-denied errors.

- [ ] **Step 4: Implement safe migration coordinator**

Read legacy secrets only from explicitly known app config keys (`claudeAccessToken`, `claudeRefreshToken`, `anthropicApiKey`); do not migrate Claude Code-owned files. After verified secure write, atomically rewrite config without those keys and set non-secret `secretMigrationVersion: 1`. Log only account kind and outcome.

- [ ] **Step 5: Verify GREEN and security regression tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::secret_store::tests auth::windows::tests`

Expected: PASS; serialized config and diagnostics contain none of fixture secrets `sk-ant-test`, `access-secret`, or `refresh-secret`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/auth/secret_store.rs src-tauri/src/auth/windows.rs src-tauri/src/auth/mod.rs
git commit -m "feat(auth): store Anthropic secrets with Windows protection"
```

### Task 4: Implement contained Claude.ai OAuth with Google SSO compatibility

**Files:**
- Create: `src-tauri/src/auth/oauth.rs`
- Modify: `src-tauri/src/providers/claude.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Write failing OAuth lifecycle tests**

Test single-use state, ten-minute expiry, constant-time state comparison, PKCE S256, rejection of callback without matching state, and navigation decisions: allow HTTPS hosts `claude.ai`, `anthropic.com` subdomains, `accounts.google.com`, and exact configured callback; open unrelated HTTPS externally; block non-HTTPS except loopback callback.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::oauth::tests`

Expected: FAIL because `OAuthAttemptStore` and `navigation_policy` are absent.

- [ ] **Step 3: Implement attempt state and exchange adapter**

Move PKCE/state helpers behind `ClaudeAiOAuthClient`. Store verifier/state/created time only in `AppState`; consume before exchange. Parse only the currently verified token fields already represented by `LoginTokenResponse`, persist into `SecretStore`, and return redacted `AccountSummary`. Map any changed/unknown response to `AuthError::ProviderContractChanged`.

- [ ] **Step 4: Implement the embedded auth window**

Create a transient `claude-auth` Tauri webview with isolated data directory, no preload script, no invoke capability, clipboard disabled, navigation policy above, and explicit close/cancel. Google SSO remains hosted entirely on `accounts.google.com`; document that enterprise policy or WebView2 restrictions may require “Open in browser,” which uses the same PKCE attempt.

- [ ] **Step 5: Verify GREEN and leakage tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::oauth::tests providers::claude::tests`

Expected: PASS; mockito asserts exact token request; formatted errors never include authorization code, verifier, or returned token.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/auth/oauth.rs src-tauri/src/providers/claude.rs src-tauri/src/lib.rs src-tauri/tauri.conf.json
git commit -m "feat(auth): add contained Claude.ai PKCE login"
```

### Task 5: Add separate Console credentials and manual fallback

**Files:**
- Create: `src-tauri/src/auth/console.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/settings.json`

- [ ] **Step 1: Write failing validation and IPC tests**

Test trimming, empty rejection, 16 KiB limit, header-control-character rejection, distinct `ClaudeAi`/`AnthropicConsole` namespaces, validation request timeout, 401 mapping, and that command results expose only last four characters as `credentialHint`.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::console::tests`

Expected: FAIL with missing `validate_manual_credential`.

- [ ] **Step 3: Implement commands and validation**

Add `list_anthropic_accounts`, `save_manual_anthropic_credential`, `delete_anthropic_account`, `start_claude_ai_login`, `cancel_claude_ai_login`. Validate Console API keys only against an Anthropic endpoint whose request/response contract is already used by the Console-cost adapter; until that adapter is present, store only after explicit user confirmation and return status `unverified`, never claim successful Console login.

- [ ] **Step 4: Verify GREEN and command scope**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth::console::tests && npm run tauri -- info`

Expected: tests PASS; only the settings capability lists secret-mutating commands.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/auth/console.rs src-tauri/src/lib.rs src-tauri/capabilities/settings.json
git commit -m "feat(auth): separate Console and manual credentials"
```

### Task 6: Build account settings UI

**Files:**
- Create: `src/auth-api.ts`
- Create: `src/auth-api.test.ts`
- Create: `src/components/settings/AnthropicAccounts.tsx`
- Create: `src/components/settings/AnthropicAccounts.test.tsx`
- Modify: `src/components/SettingsApp.tsx`

- [ ] **Step 1: Write failing component tests**

Cover automatic Claude Code detection, separate “Sign in to Claude.ai” and “Add Console credential” actions, Google SSO explanatory copy, password input that clears after submit, unverified/error states, cancel, delete confirmation, keyboard operation, and axe results with no violations.

- [ ] **Step 2: Verify RED**

Run: `npm test -- --run src/components/settings/AnthropicAccounts.test.tsx`

Expected: FAIL because the component does not exist.

- [ ] **Step 3: Implement typed invoke wrapper and component**

`auth-api.ts` exposes only redacted DTOs and accepts secrets as ephemeral function arguments. `AnthropicAccounts` stores manual input in component state, clears it in `finally`, disables duplicate submissions, uses an `aria-live` status, and never places credentials in DOM data attributes, URL parameters, localStorage, sessionStorage, analytics, or thrown errors.

- [ ] **Step 4: Verify GREEN**

Run: `npm test -- --run src/auth-api.test.ts src/components/settings/AnthropicAccounts.test.tsx`

Expected: PASS, including assertion that the submitted secret is absent from rendered HTML after completion.

- [ ] **Step 5: Commit**

```bash
git add src/auth-api.ts src/auth-api.test.ts src/components/settings/AnthropicAccounts.tsx src/components/settings/AnthropicAccounts.test.tsx src/components/SettingsApp.tsx
git commit -m "feat(auth): add Anthropic account settings"
```

### Task 7: Resolve credentials for polling and remove unsafe ownership

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/providers/claude.rs`
- Modify: `src-tauri/src/creds.rs`
- Modify: `SECURITY.md`

- [ ] **Step 1: Write failing resolution tests**

Test priority: selected account, valid Claude Code discovery, app-owned Claude.ai account; Console credentials never satisfy Claude.ai usage polling. Expired secure-store tokens refresh back into the same store. Logout deletes only app-owned records and leaves Claude Code fixture bytes unchanged.

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path src-tauri/Cargo.toml auth_resolution`

Expected: FAIL because polling still reads `claude_creds_path()` directly.

- [ ] **Step 3: Implement resolver integration**

Pass a `ResolvedClaudeAiCredential` into usage fetches. Remove calls that persist login/refresh/logout into `.claude/.credentials.json`; retain its parser solely for discovery. Wake the poller after account changes. Update `SECURITY.md` with storage boundaries, migration rollback behavior, embedded-login host allowlist, and redaction guarantees.

- [ ] **Step 4: Run full verification**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml && npm test && npm run build`

Expected: all commands exit 0; Rust and Vitest suites PASS; TypeScript/Vite build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/providers/claude.rs src-tauri/src/creds.rs SECURITY.md
git commit -m "refactor(auth): resolve provider credentials securely"
```

