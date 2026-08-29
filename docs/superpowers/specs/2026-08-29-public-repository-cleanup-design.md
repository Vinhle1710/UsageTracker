# Public Repository Cleanup Design

## Goal

Make the repository ready for public use without rewriting Git history. Apply security and release-hygiene fixes once on `codex/integration-all-features`, fast-forward `main`, verify the result, and remove obsolete branches.

## Scope

- Patch the known Rust and npm dependency advisories represented by the current lockfiles.
- Replace placeholder Cargo package metadata with the project's real name and description.
- Add automated secret and dependency checks appropriate for a public repository.
- Preserve the existing MIT license, author identity, application identifier, and branding assets.
- Run the existing npm and Rust verification suites plus dependency and secret audits.
- Publish the verified integration commit to `main` using a fast-forward update only.
- Remove every local and remote branch except `main` after the remote `main` ref is verified.

## Git Safety

History will not be rewritten because the repository-wide audit found no committed secrets or private artifacts. The update to `main` must be a fast-forward; force-pushes are prohibited.

Before deleting branches, verify that all feature and fix branches are ancestors of the cleaned integration commit. Seven existing Dependabot branches each contain one isolated upgrade commit. They may be deleted without merging because the cleanup will regenerate and verify the dependency lockfiles directly.

Remote branches are deleted only after all of the following are true:

1. The cleanup commit exists on `origin/codex/integration-all-features`.
2. `origin/main` points to the same verified commit.
3. The remote default branch remains `main`.
4. The worktree is clean.

After remote verification, switch the worktree to `main`, fast-forward the local branch, and delete every other local and remote branch. The intended final branch set is only `main` and `origin/main`.

## Dependency Cleanup

Update dependency resolution narrowly enough to remove the identified `h2` and `nanoid` advisories. Avoid unrelated major-version upgrades. Regenerate only the existing npm and Cargo lockfiles, then confirm production npm dependencies and Rust dependencies have no known vulnerability findings.

Advisory warnings that have no compatible upstream remediation must be reported explicitly rather than hidden or allowlisted silently.

## Repository Automation

Keep the existing least-privilege CI permissions. Add automated checks that:

- scan the full checkout for committed secrets;
- audit production npm dependencies;
- audit the Rust lockfile;
- fail clearly when a verified vulnerability is found.

Third-party workflow actions must be pinned to immutable commit SHAs. Automation must not receive repository write permissions or secrets on pull requests.

## Metadata Cleanup

Replace the Cargo manifest's placeholder author and description values with metadata consistent with the MIT license and README. Do not alter the application bundle identifier or remove the author's public identity.

## Verification

The cleanup is ready to publish only when these checks pass from a clean worktree:

- `npm ci`
- `npm test -- --run`
- `npm run build`
- `npm audit --omit=dev`
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo audit --file src-tauri/Cargo.lock`
- the repository secret scan

After publishing and pruning, fetch with pruning enabled and verify:

- local `main` and `origin/main` resolve to the same commit;
- no other local or remote branch remains;
- `git status --short --branch` is clean and synchronized.

## Failure Handling

If a dependency update breaks compilation or tests, stop before publishing and resolve the failure on the integration branch. If branch protection rejects the fast-forward, retain all branches and report the required GitHub-side action. If any remote verification differs from the expected commit, do not delete branches.
