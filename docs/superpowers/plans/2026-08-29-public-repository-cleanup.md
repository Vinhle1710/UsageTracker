# Public Repository Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `codex/integration-all-features` public-ready by removing known dependency advisories, correcting package metadata, and enforcing least-privilege security checks in CI.

**Architecture:** Keep the cleanup forward-only and isolated to metadata, lockfiles, and GitHub Actions. Treat dependency audits as regression tests, stage only explicitly owned files, verify the committed tree in an isolated worktree, and publish only the current integration branch with a normal push.

**Tech Stack:** npm 11, Cargo/Rust, cargo-audit 0.22.2, GitHub Actions, Gitleaks Action v3, PowerShell, Git

---

### Task 1: Correct package metadata and dependency resolutions

**Files:**
- Modify: `src-tauri/Cargo.toml:1-6`
- Modify: `src-tauri/Cargo.lock`
- Modify: `package-lock.json`

- [ ] **Step 1: Reproduce the dependency findings**

Run:

```powershell
npm audit --json
cargo audit --file src-tauri/Cargo.lock
```

Expected: npm reports `nanoid <3.3.18`; cargo-audit reports `RUSTSEC-2026-0258` for `h2 0.4.15`.

- [ ] **Step 2: Replace placeholder Cargo metadata**

Change the top of `src-tauri/Cargo.toml` to:

```toml
[package]
name = "usage-tracker-overlay"
version = "0.1.2"
description = "A Windows Tauri overlay for Claude and Codex usage limits"
authors = ["Le Pham Gia Vinh"]
license = "MIT"
repository = "https://github.com/Vinhle1710/UsageTracker"
edition = "2021"
```

- [ ] **Step 3: Apply narrow lockfile updates**

Run:

```powershell
cargo update --manifest-path src-tauri/Cargo.toml -p h2 --precise 0.4.16
npm audit fix --package-lock-only --ignore-scripts
```

Expected: `src-tauri/Cargo.lock` resolves `h2` to `0.4.16`; `package-lock.json` resolves `nanoid` to at least `3.3.18`; no manifest dependency range is broadened.

- [ ] **Step 4: Inspect only the owned dependency changes**

Run:

```powershell
git diff -- src-tauri/Cargo.toml src-tauri/Cargo.lock package-lock.json
git diff --check -- src-tauri/Cargo.toml src-tauri/Cargo.lock package-lock.json
```

Expected: metadata plus the two targeted transitive resolutions change; no unrelated source file appears in this diff.

- [ ] **Step 5: Verify both dependency findings are removed**

Run:

```powershell
npm audit
cargo audit --file src-tauri/Cargo.lock
```

Expected: npm reports zero vulnerabilities; cargo-audit reports zero vulnerabilities. Non-vulnerability RustSec warnings may remain visible and must not be silently allowlisted.

- [ ] **Step 6: Commit only Task 1 files**

Run:

```powershell
git add -- src-tauri/Cargo.toml src-tauri/Cargo.lock package-lock.json
git diff --cached --check
git diff --cached --name-only
git commit -m "chore: clean public dependency metadata"
```

Expected staged names: exactly `package-lock.json`, `src-tauri/Cargo.lock`, and `src-tauri/Cargo.toml`. Concurrent edits in other files remain unstaged.

### Task 2: Add least-privilege repository security automation

**Files:**
- Modify: `.github/workflows/ci.yml:1-33`

- [ ] **Step 1: Replace the workflow with pinned actions and security jobs**

Use this complete workflow:

```yaml
name: CI

on:
  push:
  pull_request:
    branches:
      - main

permissions:
  contents: read

jobs:
  secret-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7
        with:
          fetch-depth: 0
      - uses: gitleaks/gitleaks-action@e0c47f4f8be36e29cdc102c57e68cb5cbf0e8d1e # v3
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GITLEAKS_ENABLE_UPLOAD_ARTIFACT: "false"

  dependency-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7
      - uses: actions/setup-node@249970729cb0ef3589644e2896645e5dc5ba9c38 # v6
        with:
          node-version: 22
          cache: npm
      - uses: dtolnay/rust-toolchain@4360b52568e2003a75bf9bc1d59f33a8e3fc893c
      - run: npm audit --omit=dev
      - run: cargo install cargo-audit --locked --version 0.22.2
      - run: cargo audit --file src-tauri/Cargo.lock

  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7
      - uses: actions/setup-node@249970729cb0ef3589644e2896645e5dc5ba9c38 # v6
        with:
          node-version: 22
          cache: npm
      - uses: dtolnay/rust-toolchain@4360b52568e2003a75bf9bc1d59f33a8e3fc893c
        with:
          components: rustfmt
      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2
        with:
          workspaces: src-tauri
      - run: npm ci
      - run: npm test -- --run
      - run: npm run build
      - run: cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
      - run: cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 2: Inspect workflow permissions and immutable references**

Run:

```powershell
rg -n "permissions:|contents: read|uses:|fetch-depth|GITLEAKS" .github/workflows/ci.yml
rg -n "uses: [^@]+@(v|stable|main|master)" .github/workflows/ci.yml
git diff --check -- .github/workflows/ci.yml
```

Expected: the first command shows read-only contents permission, full checkout for Gitleaks, and full 40-character action SHAs. The second command returns no matches.

- [ ] **Step 3: Commit only the workflow**

Run:

```powershell
git add -- .github/workflows/ci.yml
git diff --cached --check
git diff --cached --name-only
git commit -m "ci: add public repository security checks"
```

Expected staged name: exactly `.github/workflows/ci.yml`.

### Task 3: Verify the committed cleanup in isolation

**Files:**
- Verify: committed repository tree
- Preserve: all unrelated working-tree modifications

- [ ] **Step 1: Confirm cleanup files are committed and unrelated edits are unstaged**

Run:

```powershell
git status --short --branch
git diff --name-only
git diff HEAD -- .github/workflows/ci.yml package-lock.json src-tauri/Cargo.lock src-tauri/Cargo.toml
```

Expected: unrelated concurrent edits may remain in the first two outputs; the final command produces no output.

- [ ] **Step 2: Create a detached verification worktree at the cleanup commit**

Use the repository's ignored `.worktrees` directory and verify the target before creation:

```powershell
$verificationPath = [System.IO.Path]::GetFullPath((Join-Path (git rev-parse --show-toplevel) '.worktrees/public-cleanup-verification'))
$workspaceRoot = [System.IO.Path]::GetFullPath((git rev-parse --show-toplevel))
if (-not $verificationPath.StartsWith($workspaceRoot, [System.StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe verification path' }
git worktree add --detach $verificationPath HEAD
```

Expected: a detached worktree is created at the cleanup commit without touching the current worktree's unrelated edits.

- [ ] **Step 3: Run the full verification suite in the detached worktree**

Run from `.worktrees/public-cleanup-verification`:

```powershell
npm ci
npm test -- --run
npm run build
npm audit
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo audit --file src-tauri/Cargo.lock
```

Expected: install, tests, build, formatting, and both audits exit successfully. Cargo-audit may print non-vulnerability advisory warnings while still reporting zero vulnerabilities.

- [ ] **Step 4: Re-run the repository history signature scan**

Run from the main worktree:

```powershell
$revs = @(git rev-list --all)
$pattern = '(-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----|(AKIA|ASIA)[0-9A-Z]{16}|AIza[0-9A-Za-z_-]{35}|gh[pousr]_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]{20,}|xox[baprs]-[0-9A-Za-z-]{10,}|[rs]k_live_[0-9A-Za-z]{16,}|npm_[0-9A-Za-z]{30,}|glpat-[0-9A-Za-z_-]{20,})'
$hits = @(git grep -a -l -E -i -e $pattern $revs 2>$null)
if ($hits.Count -ne 0) { throw "Secret signature findings: $($hits.Count)" }
```

Expected: zero findings.

- [ ] **Step 5: Remove only the temporary verification worktree**

Run from the main worktree after confirming the resolved target:

```powershell
$verificationPath = [System.IO.Path]::GetFullPath((Join-Path (git rev-parse --show-toplevel) '.worktrees/public-cleanup-verification'))
$workspaceRoot = [System.IO.Path]::GetFullPath((git rev-parse --show-toplevel))
if (-not $verificationPath.StartsWith($workspaceRoot, [System.StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe verification path' }
git worktree remove $verificationPath
```

Expected: only the detached verification worktree is removed; the primary worktree and branch remain intact.

### Task 4: Publish and verify only the integration branch

**Files:**
- Publish: commits on `codex/integration-all-features`
- Preserve: every other local and remote ref

- [ ] **Step 1: Snapshot all other remote branch refs and push normally**

Run as one PowerShell block:

```powershell
$branch = 'codex/integration-all-features'
$before = @(git ls-remote --heads origin | Where-Object { $_ -notmatch "refs/heads/$([regex]::Escape($branch))$" })
git push origin $branch
if ($LASTEXITCODE -ne 0) { throw 'Integration branch push failed' }
$after = @(git ls-remote --heads origin | Where-Object { $_ -notmatch "refs/heads/$([regex]::Escape($branch))$" })
$changed = @(Compare-Object $before $after)
if ($changed.Count -ne 0) { throw 'A non-target remote branch changed' }
```

Expected: a normal non-force push updates only `origin/codex/integration-all-features`.

- [ ] **Step 2: Verify local and remote integration commits match**

Run:

```powershell
git fetch origin codex/integration-all-features
$localCommit = git rev-parse codex/integration-all-features
$remoteCommit = git rev-parse origin/codex/integration-all-features
if ($localCommit -ne $remoteCommit) { throw 'Local and remote integration refs differ' }
git status --short --branch
```

Expected: local and remote commit IDs match. Concurrent unrelated working-tree edits remain present and unstaged; no cleanup-owned file is modified.
