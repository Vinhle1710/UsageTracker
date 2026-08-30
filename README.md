# Usage Tracker Overlay

A small Windows Tauri overlay for Claude and Codex usage limits. It watches for Claude,
ChatGPT, Codex, and their supported VS Code integrations, shows only the active provider
cards, refreshes usage about once per minute, and hides when no supported client is running.

Detection tolerates a short gap before hiding. The CLIs are not daemons — `codex` exits between
invocations and `claude` restarts — so a provider is held on screen for about a minute after its
process was last seen. Without that grace period the overlay blinked out and back as processes
came and went.

## The overlay

The window is frameless, always-on-top, skip-taskbar, keyboard accessible, responsive, and has
no scrollbar or default title controls. Each card shows a usage readout with the percentage, a
live countdown for the 5-hour reset, and a local date/time for the weekly reset. The stacked
compact and provider-columns layouts can be switched instantly.

There are three resting states, each one step smaller:

1. **Cards** — the full readout per active provider.
2. **Bubbles** — the inline arrow on a card minimises it to a provider pill.
3. **Edge tab** — the tuck control at the end of the bubble row collapses everything to a
   single tab against the screen edge, showing no bubbles at all. Clicking the tab restores it.

## Settings

Settings open from the taskbar tray icon only. They appear in a centered popup, save instantly,
list available screens by friendly names, and control corner, layout, scale, and always-on-top
behavior.

**Display** covers layout and scale, the shape of each usage readout, and the card theme.

- **Readout shape** — Ring, Charge, Arc Reactor, Columns, Line, or Semi Circle.
- **Themes** — Translucent gradient, Frosted, Solid, and Neon. Solid is always fully opaque; its
  opacity slider is disabled, because a translucent "solid" card is just Frosted. Neon lights the
  meter stroke, the percentage, and the card edge, leaving labels and reset text unlit.

## Providers

### Claude

Usage is read one of two ways.

**With a claude.ai session key** (preferred) — read directly from
`claude.ai/api/organizations/{org}/usage`. Costs nothing.

**Without one** — the app makes the smallest possible `POST /v1/messages` call (Haiku,
`max_tokens: 1`) and reads the `anthropic-ratelimit-unified-5h/7d-*` response headers. This
spends one token per poll against the very limits it measures, which is the main reason the
session-key path is preferred. A 429 is treated as a valid reading rather than a failure: it
still carries the headers, and it means the account is genuinely at its limit.

`GET /api/oauth/usage` is not used. It now answers 429 to every request regardless of real
usage, which previously left this card stuck on stale desktop-cache numbers indefinitely.

**Extra usage credit** is read from claude.ai's `overage_spend_limit` and
`overage_credit_grant` endpoints and shown as a horizontal bar under the 5 hour and Weekly
meters. It needs the same session key. Without one the bar is simply absent.

### The claude.ai session key

Both of the above are unlocked by one credential: the `sessionKey` cookie claude.ai sets in your
browser. Signing in through **Settings → Account** captures it automatically — the sign-in
webview authenticates against claude.ai, and its cookie jar is read once sign-in completes.
Nothing needs to be pasted.

If that fails (the login window was closed early, or the cookie was not set), the same panel has
a manual field behind **Enter it manually instead**: open claude.ai signed in, press `F12`, go to
**Application → Cookies → `https://claude.ai`**, and copy the `sessionKey` value.

Treat it like a password: it grants access to the account until you sign out of that browser. On
Windows it is stored in Credential Manager under `UsageTracker/anthropic/claude-ai/session`, and
is only ever sent to claude.ai. Usage limits still work without it, via the header probe above.

### Codex / OpenAI

The OpenAI layer displays Codex usage. The ChatGPT desktop app is used as an activation signal,
but its consumer message quota is not exposed by a local or documented API. When the Codex
endpoint is unavailable, the app falls back to the newest local
`~/.codex/sessions/**/*.jsonl` rate-limit record and marks it stale.

### Anthropic Console costs

Console costs use a separate Anthropic Console credential and identity from Claude.ai OAuth.
Spend, prepaid balance, daily, API-key, and model sections are independent and may be unavailable
when the verified source does not expose a contract or the credential lacks a role. Unavailable
is never rendered as zero. Amounts use integer minor units with explicit currency and UTC month
boundaries. Prepaid balance is never inferred from spend or budget; capabilities are enabled only
after an authoritative, redacted fixture records the endpoint and response keys.

## Requirements

Windows 10/11, plus at least one of:

- **Claude** — sign in with the Claude Code CLI (`claude`), which writes
  `%USERPROFILE%\.claude\.credentials.json`, **or** just use the Claude desktop app: it writes
  its own local usage cache at `%APPDATA%\Claude\plan-usage-history.json` independent of the
  CLI, and the overlay falls back to it (marked stale, since it's a periodic local snapshot
  rather than a live read) whenever there's no working CLI session
- **Codex** — sign in with the Codex CLI (`codex`), which writes
  `%USERPROFILE%\.codex\auth.json`

Nothing has to be configured to get started. The overlay discovers whichever credential files
exist under the current Windows user's profile and starts polling that provider. A card whose
provider has never been signed in reads **Not signed in** and names the CLI to run; sign in and
the card picks it up on the next poll without a restart. The claude.ai session key is the one
optional extra, and signing in through Settings captures it for you.

Because every path is resolved from the running user's home directory, the app works
unmodified for any user on any machine — nothing in this repository is tied to a specific
account. The OAuth client ID in `src-tauri/src/providers/claude.rs` is Claude Code's own
public client identifier, which is identical for every installation and is not a secret.

## Credentials

The app reads the credential and cache files the Claude Code CLI, Claude desktop app, and Codex
CLI already maintain:

| File | Access |
| --- | --- |
| `%USERPROFILE%\.claude\.credentials.json` | Read, and rewritten when a token is refreshed |
| `%APPDATA%\Claude\plan-usage-history.json` | Read only — used only when the credentials file above is missing or its request fails |
| `%USERPROFILE%\.codex\auth.json` | Read only |

Two credentials are stored by the app itself rather than read from disk: the claude.ai session
key described above, and any Anthropic Console key entered for Console costs. Both go to Windows
Credential Manager (`UsageTracker/anthropic/...`), never to a file of the app's own.

When the Claude access token is at or near expiry, the app performs the standard OAuth refresh
against `platform.claude.com` and writes the rotated access and refresh tokens back into
`.credentials.json` — the same thing the Claude Code CLI does, using the same public client ID.
The write is atomic (temp file plus rename) and merges into the existing JSON, so unrelated
keys such as `mcpOAuth` are preserved. Codex credentials and the desktop app's usage cache are
only ever read, never written.

No credential is written to logs, copied into the app configuration, or sent anywhere other than
the provider it belongs to.

### Claude usage v2 limitations

Claude per-model limits are a best-effort, fixture-gated adapter for an undocumented provider
contract. Unknown model keys are displayed automatically when a verified response exposes them;
until a redacted response fixture is captured, that section is shown as unavailable, and
unavailable never means zero. Claude service health is fetched separately from the public
`status.claude.com` Statuspage API and never uses Claude credentials.

Two windows are parsed today, `5 hour` and `Weekly`. Newer responses also carry per-model weekly
scopes in a `limits[]` array, which supersedes the older `seven_day_*` fields; those are not read
yet.

## Configuration

The app stores JSON settings in the Tauri app config directory, normally:

`%APPDATA%\com.vinh1.usage-tracker-overlay\config.json`

Delete that file to reset to bottom-right, stacked compact layout, 100% scale, and
always-on-top.

## Runtime automation safety

Runtime automation is opt-in. Automatic session initialization is off by default and can only be
enabled after storing the exact acknowledgement: "I understand this can start a paid API/CLI
session". The app uses a fixed local model catalog (`gpt-5.6-terra` for light/standard work and
`gpt-5.6-sol` for reasoning), never discovers models remotely, never stores prompts or
credentials, and starts CLI processes directly without a shell. Initialization retries have a
30-minute cooldown.

Polling defaults to 60 seconds and is bounded to a minimum of 15 seconds and maximum of 3600
seconds. Scheduled polling pauses while offline; manual refresh reports the current network
state. Wake and network signals only request a refresh and do not perform usage requests
themselves.

On Windows, connectivity monitoring initializes COM on a dedicated observer thread and
subscribes to Network List Manager connection-point events; an advise guard unadvises before COM
is uninitialized. If subscription setup fails, the observer falls back to a bounded five-second
reachability probe against the active provider host. Non-Windows builds use the no-op
portability path. Power resume is edge-mapped to the coordinator and never performs network work
in the adapter.

Launch-at-login retains the existing Windows HKCU
`Software\Microsoft\Windows\CurrentVersion\Run` value (`Usage Tracker Overlay`) and reports
registration status. Global shortcuts are optional; duplicate or conflicting bindings are
rejected transactionally and never saved as inactive shortcuts.

## Local history

Usage samples and explicitly reported billing values are single-account, local-only data stored
at the Tauri `app_data_dir/history.sqlite3`. History is retained for 180 days by default
(configurable within the supported retention range). JSON/CSV exports may contain sensitive
usage and cost metadata; choose a protected destination and handle exported files accordingly.

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
Push-Location src-tauri; cargo test; cargo clippy --all-targets; Pop-Location
```

## Installer

```powershell
npm run tauri build
```

The MSI and setup executable are written under `src-tauri/target/release/bundle/`.

## License

[MIT](LICENSE)
