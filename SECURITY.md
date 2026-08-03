# Security

Please report vulnerabilities privately through GitHub's **Security advisories** page for this repository. Do not include access tokens, credential files, or raw provider API responses in an issue.

## Credential handling

The app reads the credential files the Claude Code and Codex CLIs already maintain in the
current user's home directory, and sends those tokens only to the corresponding provider's own
usage endpoint. Credentials are never copied into the app configuration or written to logs.

`%USERPROFILE%\.claude\.credentials.json` is also **written** to: when the Claude access token
expires, the app performs the standard OAuth refresh and merges the rotated tokens back into
that file atomically, preserving unrelated keys. This is the same refresh the Claude Code CLI
performs, using Claude Code's public OAuth client ID — a public client identifier, not a
secret. `%USERPROFILE%\.codex\auth.json` is only ever read.

No credential is transmitted to any third party, and the app has no server component or
telemetry of its own.
