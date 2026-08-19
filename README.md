# Usage Tracker Overlay

A small Windows Tauri overlay for Claude and Codex usage limits. It watches for Claude,
ChatGPT, Codex, and their supported VS Code integrations, shows only the active provider
cards, refreshes usage about once per minute, and hides when no supported client is running.

The window is frameless, always-on-top, skip-taskbar, keyboard accessible, responsive, and
has no scrollbar or default title controls. Each card uses circular meters with the
percentage centered, a live countdown for the 5-hour reset, and a local date/time for the
weekly reset. The stacked compact and provider-columns layouts can be switched instantly.
The small inline arrow minimizes the overlay to a two-dot provider pill.

Settings are opened from the taskbar tray icon only. They appear in a centered popup, save
instantly, list available screens by friendly names, and control corner, layout, scale, and
always-on-top behavior.

Card themes include Translucent gradient, native Windows Acrylic, native Windows Blur, and
Solid. Acrylic and Blur fill the rounded card regions uniformly; they do not add a CSS gradient.

The OpenAI layer displays Codex usage. The ChatGPT desktop app is used as an activation
signal, but its consumer message quota is not exposed by a local or documented API.
When the Codex endpoint is unavailable, the app falls back to the newest local
`~/.codex/sessions/**/*.jsonl` rate-limit record and marks it stale.

## Requirements

Windows 10/11, plus at least one of:

- **Claude** — sign in with the Claude Code CLI (`claude`), which writes
  `%USERPROFILE%\.claude\.credentials.json`, **or** just use the Claude desktop app: it writes
  its own local usage cache at `%APPDATA%\Claude\plan-usage-history.json` independent of the
  CLI, and the overlay falls back to it (marked stale, since it's a periodic local snapshot
  rather than a live read) whenever there's no working CLI session
- **Codex** — sign in with the Codex CLI (`codex`), which writes
  `%USERPROFILE%\.codex\auth.json`

There is nothing to configure and no key to paste. The overlay discovers whichever credential
files exist under the current Windows user's profile and starts polling that provider. A card
whose provider has never been signed in reads **Not signed in** and names the CLI to run; sign
in and the card picks it up on the next poll without a restart.

Because every path is resolved from the running user's home directory, the app works
unmodified for any user on any machine — nothing in this repository is tied to a specific
account. The OAuth client ID in `src-tauri/src/providers/claude.rs` is Claude Code's own
public client identifier, which is identical for every installation and is not a secret.

## Development

Prerequisites: Node.js, Rust/Cargo, WebView2, and the Windows desktop build tools.

```powershell
npm install
npm run dev
npm run tauri dev
```

Checks:

```powershell
npm test
npm run coverage
npm run build
Push-Location src-tauri; cargo test; Pop-Location
```

## Installer

```powershell
npm run tauri build
```

The MSI and setup executable are written under
`src-tauri/target/release/bundle/`.

## Configuration

The app stores JSON settings in the Tauri app config directory, normally:

`%APPDATA%\com.vinh1.usage-tracker-overlay\config.json`

Delete that file to reset to bottom-right, stacked compact layout, 100% scale, and
always-on-top.

## Credentials

The app never asks for a token and never stores one of its own. It reads the credential and
cache files the Claude Code CLI, Claude desktop app, and Codex CLI already maintain:

| File | Access |
| --- | --- |
| `%USERPROFILE%\.claude\.credentials.json` | Read, and rewritten when a token is refreshed |
| `%APPDATA%\Claude\plan-usage-history.json` | Read only — used only when the credentials file above is missing or its request fails |
| `%USERPROFILE%\.codex\auth.json` | Read only |

When the Claude access token is at or near expiry, the app performs the standard OAuth refresh
against `platform.claude.com` and writes the rotated access and refresh tokens back into
`.credentials.json` — the same thing the Claude Code CLI does, using the same public client ID.
The write is atomic (temp file plus rename) and merges into the existing JSON, so unrelated
keys such as `mcpOAuth` are preserved. Codex credentials and the desktop app's usage cache are
only ever read, never written.

No token is written to logs, copied into the app configuration, or sent anywhere other than
the corresponding provider's own usage endpoint.

## License

[MIT](LICENSE)

## Runtime automation safety

Runtime automation is opt-in. Automatic session initialization is off by default and can only be enabled after storing the exact acknowledgement: “I understand this can start a paid API/CLI session”. The app uses a fixed local model catalog (`gpt-5.6-terra` for light/standard work and `gpt-5.6-sol` for reasoning), never discovers models remotely, never stores prompts or credentials, and starts CLI processes directly without a shell. Initialization retries have a 30-minute cooldown.

Polling defaults to 60 seconds and is bounded to a minimum of 15 seconds and maximum of 3600 seconds. Scheduled polling pauses while offline; manual refresh reports the current network state. Wake and network signals only request a refresh and do not perform usage requests themselves.

On Windows, connectivity monitoring initializes COM on a dedicated observer thread and subscribes to Network List Manager connection-point events; an advise guard unadvises before COM is uninitialized. If subscription setup fails, the observer falls back to a bounded five-second reachability probe against the active provider host. Non-Windows builds use the no-op portability path. Power resume is edge-mapped to the coordinator and never performs network work in the adapter.

Launch-at-login retains the existing Windows HKCU `Software\Microsoft\Windows\CurrentVersion\Run` value (`Usage Tracker Overlay`) and reports registration status. Global shortcuts are optional; duplicate or conflicting bindings are rejected transactionally and never saved as inactive shortcuts.
