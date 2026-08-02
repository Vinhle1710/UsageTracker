# Startup Repair and Provider Bubbles Implementation Plan

> **For the implementer:** Follow the `executing-plans`, `test-driven-development`, and `verification-before-completion` skills. Complete each task with a red test, the smallest production change, green verification, and the listed commit before continuing.

**Goal:** Reliably launch the installed listener at Windows sign-in, reveal active providers immediately without a native frame, and support independent Claude and ChatGPT card-to-bubble minimization.

**Architecture:** Keep one tray-resident Tauri process and one transparent overlay WebView. Rust owns startup registration, provider activation, visibility, native style repair, and window geometry. TypeScript owns provider-keyed presentation state and performs identity-stable DOM updates. CSS remains responsible for card-local Translucent Gradient, Frosted, Blur, and Solid materials.

**Stack:** Tauri 2, Rust, TypeScript, Vitest/jsdom/vitest-axe, Win32 `windows-sys`, CSS.

---

## Task 1: Register a valid installed Windows startup command

**Files:**
- Create: `src-tauri/src/startup.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src/main.ts`
- Modify: `package.json`

1. Add Rust unit tests in `startup.rs` for a path containing spaces, embedded quotes/backslashes, no trailing empty argument, and release-only registration. Assert the generated registry command is exactly a quoted executable path.
2. Run `cargo test startup --manifest-path src-tauri/Cargo.toml` and confirm the tests fail.
3. Implement a Windows-only startup module that writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` using an explicit product value name and a properly quoted current executable. Make registration a no-op outside release builds and outside Windows. Never log credentials or environment contents.
4. Call registration during Tauri setup before the listener begins. Remove the frontend `enable()` call and remove `@tauri-apps/plugin-autostart` plus the Rust plugin dependency/configuration.
5. Run `cargo test startup --manifest-path src-tauri/Cargo.toml`, `npm test -- --run`, and `npm run build`.
6. Commit: `fix: register quoted Windows startup command`

## Task 2: Reveal providers immediately and isolate provider identity

**Files:**
- Modify: `src-tauri/src/visibility.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/model.rs`
- Modify: `src/state.ts`
- Modify: `src/state.test.ts`
- Modify: `src/main.ts`

1. Add Rust tests proving an active provider reveals as soon as the WebView is ready, without waiting for network usage readiness, and that provider activation wakes polling immediately. Preserve manual tray-hide semantics and reset the override only after all providers become inactive.
2. Add frontend tests that repeatedly activate, close, reopen, poll, and minimize providers while asserting Claude data/color/logo never appears in ChatGPT and vice versa. Use provider keys, not list position.
3. Run the targeted Rust and Vitest tests and confirm they fail.
4. Split visibility readiness from usage readiness. Show cached data when available and a provider-owned loading/unavailable state otherwise. Trigger an immediate poll on an inactive-to-active transition while retaining the one-second detector.
5. Refactor state reconciliation to keyed `claude` and `openai` records so nodes and snapshots cannot migrate when one provider disappears.
6. Run `cargo test visibility --manifest-path src-tauri/Cargo.toml` and `npm test -- src/state.test.ts`.
7. Commit: `fix: reveal and update providers by identity`

## Task 3: Enforce the borderless contract throughout the window lifecycle

**Files:**
- Modify: `src-tauri/src/window.rs`
- Modify: `src-tauri/src/material.rs`
- Modify: `src-tauri/src/lib.rs`
- Add or extend unit tests beside the affected Rust modules

1. Add tests for frame repair on both `Focused(true)` and `Focused(false)`, for a deferred second repair, and for restoring the main overlay region after repair. Cover both `main` and `settings` windows.
2. Run targeted Rust tests and confirm failure.
3. Centralize overlay/settings surface repair. On Windows strip caption, system menu, thick frame, minimize, and maximize styles; disable native non-client rendering and shadow. Invoke repair on setup, show, focus gain, focus loss, settings close, geometry changes, bubble transitions, and tray restore.
4. Schedule a short deferred repair after focus events so Windows/WebView2 cannot reapply a native frame after the synchronous callback. Reapply the cached clipping region only to the main overlay.
5. Run all Rust tests and `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
6. Commit: `fix: keep tracker windows permanently borderless`

## Task 4: Implement independent accessible provider bubbles

**Files:**
- Modify: `src/types.ts`
- Modify: `src/components/controls.ts`
- Modify: `src/components/controls.test.ts`
- Modify: `src/components/overlay.ts`
- Modify: `src/components/overlay.test.ts`
- Modify: `src/main.ts`
- Modify: `src/a11y.test.ts`

1. Add failing tests for independent Claude/ChatGPT collapse and restore, stable Claude-then-ChatGPT bubble order, one expanded card plus one bubble, both bubbles, provider-specific accessible names, keyboard operation, and focus transfer between minimize control and resulting bubble.
2. Replace the global `minimized` boolean with provider-keyed session state. Bind every action to a provider key.
3. Render a persistent top-corner bubble row. Each 48-pixel bubble uses the supplied provider logo, active theme variables, a native `button`, a minimum 44-pixel hit target, `aria-label="Expand … usage"`, and decorative logo semantics.
4. Keep expanded provider DOM nodes identity-stable. Minimizing one provider must not rebuild or recolor the other provider. Move focus to the bubble after collapse and back to that card's minimize button after expansion.
5. Run `npm test -- src/components/controls.test.ts src/components/overlay.test.ts src/a11y.test.ts`.
6. Commit: `feat: add independent provider usage bubbles`

## Task 5: Measure mixed card-and-bubble geometry and motion

**Files:**
- Modify: `src/geometry.ts`
- Modify: `src/geometry.test.ts`
- Modify: `src/main.ts`
- Modify: `src/styles/app.css`
- Modify: `src/styles/app-css.test.ts`
- Modify: `src-tauri/src/window.rs`

1. Add failing tests for vertical and horizontal layouts with zero, one, or two collapsed providers; selected-corner alignment; overlap avoidance; both-bubble shrink-to-row behavior; card-local clipping; taskbar avoidance; 44-pixel controls; theme inheritance; and reduced-motion behavior.
2. Replace the global geometry `minimized` flag with expanded-provider and bubble counts plus measured content rectangles. Keep the bubble row pinned to the chosen top corner and shift expanded-card minimize controls inward when needed.
3. Update Rust geometry requests and clipping-region construction so unused transparent host pixels are excluded and non-interactive. Do not apply Frosted/Blur material to the native window; keep those effects on full card/bubble surfaces only.
4. Add short collapse/expand/hover/press/focus transitions and disable nonessential movement under `prefers-reduced-motion: reduce`.
5. Run `npm test -- src/geometry.test.ts src/styles/app-css.test.ts`, then the complete frontend and Rust suites.
6. Commit: `feat: support mixed card and bubble layouts`

## Task 6: Release, package, live-test, and publish 0.1.3

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/tauri.conf.json`
- Review: `.gitignore`, tracked files, generated installers, and repository history/diff

1. Set version `0.1.3` consistently and regenerate lockfiles through the package tools.
2. Run `npm test -- --run`, `npm run coverage`, `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, and `npm run tauri build`.
3. Audit `git status`, `git diff --check`, `git ls-files`, `.gitignore`, and staged changes for credentials, local paths, session data, tokens, temporary screenshots, build output, and installers. Keep secrets and machine-local state untracked.
4. Install and launch the packaged executable. Verify the exact Run-registry value is a quoted executable path with no trailing argument and that it launches successfully as a normal user.
5. With live Claude and ChatGPT processes, verify immediate loading/cached reveal, in-place usage updates, correct provider mapping, one-second activation, one expanded card plus one bubble, two bubbles, both layouts, every corner, tray Show/Hide, Settings close, monitor placement, and taskbar avoidance.
6. Repeatedly change focus and sample both windows' Win32 styles across startup, settings close, minimize/restore, and another-app activation. Confirm no caption, system menu, resize frame, default buttons, shadow, flicker, or full-DOM redraw appears.
7. Commit: `chore: release startup repair and provider bubbles as 0.1.3`
8. Review the complete branch diff and rerun the full verification suite after any review fix.
9. Fast-forward or merge into `codex/per-card-native-material`, push the branch, update pull request #2 with the new scope and verification evidence, and wait for required checks to pass.

