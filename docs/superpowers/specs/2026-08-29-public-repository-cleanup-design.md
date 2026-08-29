# Public Repository Cleanup Design

## Goal

Make `codex/integration-all-features` ready for public use without rewriting Git history or changing any other branch.

## Scope

- Patch the known Rust and npm dependency advisories represented by the current lockfiles.
- Replace placeholder Cargo package metadata with the project's real name and description.
- Add automated secret and dependency checks appropriate for a public repository.
- Preserve the existing MIT license, author identity, application identifier, and branding assets.
- Run the existing npm and Rust verification suites plus dependency and secret audits.
- Commit and publish the verified cleanup only to `codex/integration-all-features`.
- Leave `main` and every other local and remote branch unchanged.

## Git Safety

History will not be rewritten because the repository-wide audit found no committed secrets or private artifacts. Force-pushes and branch deletions are prohibited. The only permitted remote change is a normal push of verified cleanup commits to `origin/codex/integration-all-features`.

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

After publishing, fetch the integration ref and verify:

- local `codex/integration-all-features` and `origin/codex/integration-all-features` resolve to the same commit;
- all other local and remote branch refs are unchanged;
- cleanup files have no uncommitted changes;
- the four pre-existing unrelated working-tree edits remain untouched.

## Failure Handling

If a dependency update breaks compilation or tests, stop before publishing and resolve the failure on the integration branch. If the remote rejects a normal push, keep the verified commits locally and report the required GitHub-side action. If remote verification differs from the expected commit, stop without changing any other ref.
