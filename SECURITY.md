# Security

Console credentials are isolated from Claude Code OAuth credentials and are read only for the
fixed-origin Console adapter. Commands do not accept endpoint URLs, headers, or raw tokens.
Redirects are disabled, request timeout is bounded, and response bodies are capped while
streaming before JSON parsing.
Secrets are not included in cache keys, events, diagnostics, or fixtures. Billing and prepaid
balance remain unavailable until exact provider contracts are verified in the Console fixture
metadata.

Please report vulnerabilities privately through GitHub's **Security advisories** page for this repository. Do not include access tokens, credential files, or raw provider API responses in an issue.

## Credential handling

The app reads the credential files the Claude Code and Codex CLIs already maintain in the
current user's home directory, and sends those tokens only to the corresponding provider's own
usage endpoint. Credentials are never copied into the app configuration or written to logs.

`%USERPROFILE%\.claude\.credentials.json` is Claude Code-owned and read-only to Usage Tracker.
The app does not initiate Claude OAuth, refresh its tokens, or sign the CLI out. App-owned
claude.ai and Console session cookies use a distinct secure-store namespace
(`UsageTracker/anthropic/<kind>/session`), with zeroized in-memory values. Windows Credential
Manager is primary; a DPAPI-encrypted file under the user's local data directory is the fallback.
Obsolete plaintext credential keys from pre-1.0 app config are deleted at startup because the
features that owned them no longer exist. `%USERPROFILE%\.codex\auth.json` is also read-only.

The app is bundled for Windows only. IPC reports only whether app-owned session cookies exist and
returns Claude account display metadata—never tokens or secrets.

No credential is transmitted to any third party, and the app has no server component or
telemetry of its own.
