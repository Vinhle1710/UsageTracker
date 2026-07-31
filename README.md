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

The OpenAI layer displays Codex usage. The ChatGPT desktop app is used as an activation
signal, but its consumer message quota is not exposed by a local or documented API.
When the Codex endpoint is unavailable, the app falls back to the newest local
`~/.codex/sessions/**/*.jsonl` rate-limit record and marks it stale.

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

`%APPDATA%\com.vinh1.usage-tracker-overlay\config\config.json`

Delete that file to reset to bottom-right, stacked compact layout, 100% scale, and
always-on-top.

Tokens are read-only from `%USERPROFILE%\.claude\.credentials.json` and
`%USERPROFILE%\.codex\auth.json`; they are never logged, persisted, or refreshed.
