# Security

Console credentials are isolated from Claude.ai OAuth credentials and are read only for the
least-privilege, fixed-origin Console adapter. Account IDs are validated against the secure
account inventory; commands do not accept endpoint URLs, headers, or raw tokens. Redirects are
disabled, request timeout is bounded, and API-key labels are suffix-redacted before IPC/UI.
Secrets are not included in cache keys, events, diagnostics, or fixtures. Billing and prepaid
balance remain unavailable until exact provider contracts are verified in the Console fixture
metadata.

Please report vulnerabilities privately through GitHub's **Security advisories** page for this repository. Do not include access tokens, credential files, or raw provider API responses in an issue.

## Credential handling

The app reads the credential files the Claude Code and Codex CLIs already maintain in the
current user's home directory, and sends those tokens only to the corresponding provider's own
usage endpoint. Credentials are never copied into the app configuration or written to logs.

`%USERPROFILE%\.claude\.credentials.json` is Claude Code-owned. The app reads it for discovery
and may atomically merge refreshed Claude OAuth credentials into it, preserving unrelated keys;
it does not delete the file. App-owned claude.ai and Console credentials use a distinct
secure-store namespace (`UsageTracker/anthropic/<kind>/<account-id>`), with zeroized in-memory
values. Legacy app config keys are migrated only after secure write/read-back verification;
failed verification leaves the source untouched. `%USERPROFILE%\.codex\auth.json` is only ever
read.

Embedded login navigation is restricted to HTTPS Claude/Anthropic hosts, Google Accounts, and the
exact configured callback; other HTTPS URLs are opened externally and non-HTTPS URLs are blocked
except loopback callbacks. IPC returns redacted account summaries only—never tokens or secrets.

No credential is transmitted to any third party, and the app has no server component or
telemetry of its own.
