# Usage Tracker Overlay — Design

**Date:** 2026-07-31
**Status:** Approved design, pending implementation plan

## 1. Purpose

An always-available desktop overlay for Windows that shows current Claude and OpenAI
(Codex) usage limits. It appears automatically while any relevant AI client is running
and hides when the last one closes, so quota state is glanceable without opening a CLI
or switching apps.

### Goals

- Show usage windows (percent used, reset time) for Claude and Codex.
- Refresh every 60 seconds with a visible "last updated" indicator.
- Appear/disappear automatically based on which AI clients are running.
- Sit unobtrusively in a chosen corner of a chosen monitor, always on top.
- Be fully responsive and accessible.

### Non-goals

- Showing ChatGPT desktop app message quotas. No local file or documented endpoint
  exposes them (see §3.3). The OpenAI layer reports Codex usage only.
- Historical charts, cost tracking, or per-project breakdowns.
- Refreshing or mutating authentication tokens (see §8).
- Supporting platforms other than Windows.

## 2. Verified environment findings

These were confirmed on the target machine on 2026-07-31 and drive the design. The
distinction between **verified** and **inferred** matters: inferred items are the first
thing implementation must confirm.

| Finding | Status |
|---|---|
| `~/.claude/.credentials.json` exists | Verified |
| `~/.codex/auth.json` has `tokens.access_token`, `refresh_token`, `account_id` | Verified |
| Claude binary contains `/api/oauth/usage` | Verified (string present) |
| Claude binary contains `five_hour`, `seven_day`, `utilization`, `resets_at` | Verified (strings present) |
| Exact JSON response shape of the Claude usage endpoint | **Inferred** |
| Codex binary contains `/backend-api/api/codex/usage` | Verified (string present) |
| Exact JSON response shape of the Codex usage endpoint | **Inferred** |
| Codex session JSONL contains a live `rate_limits` object | **Verified (real data read)** |
| `~/.claude/ide/*.lock` contains `pid`, `workspaceFolders`, `ideName` | Verified |
| 8 of 9 lock files are stale (dating to Jul 6) | Verified |

Observed Codex payload, used as the parser fixture:

```json
"rate_limits": {
  "limit_id": "codex",
  "limit_name": null,
  "primary": { "used_percent": 25.0, "window_minutes": 10080, "resets_at": 1785978830 },
  "secondary": null,
  "credits": { "has_credits": false, "unlimited": false, "balance": "0" },
  "plan_type": "plus",
  "rate_limit_reached_type": null
}
```

Note `secondary` is `null` and the only window is weekly (10080 minutes). The UI must not
assume a 5-hour window exists.

## 3. Data sources

### 3.1 Claude

`GET https://api.anthropic.com/api/oauth/usage`, `Authorization: Bearer <token>` read from
`~/.claude/.credentials.json`. Expected to yield 5-hour and 7-day windows plus separate
Opus tracking.

### 3.2 Codex

`GET https://chatgpt.com/backend-api/api/codex/usage`, bearer token from
`~/.codex/auth.json` (`tokens.access_token`).

**Fallback:** if the request fails, read the newest file under `~/.codex/sessions/**/*.jsonl`
and take the last `rate_limits` object. This is verified to contain valid data and requires
no network access. Values sourced this way are marked `Stale`.

### 3.3 ChatGPT desktop app

No data source exists. The app remains a *trigger* that causes the OpenAI layer to show,
but the numbers displayed are Codex's. This is a deliberate, user-approved limitation.

### 3.4 Normalized model

Both providers normalize to one shape so the UI never branches on vendor:

```rust
enum SnapshotState { Fresh, Stale, Error }

struct UsageWindow {
    label: String,       // derived from window_minutes, e.g. "5 hour", "Weekly"
    used_percent: f32,
    resets_at: i64,      // unix seconds
}

struct UsageSnapshot {
    windows: Vec<UsageWindow>,
    fetched_at: i64,
    state: SnapshotState,
}
```

`windows` is a list, never fixed fields.

### 3.5 Window rendering rule

Both providers support a **5-hour** and a **weekly** window. Neither is assumed to exist.

| Case | Render |
|---|---|
| Window present with a value | Show the row, including at 0% |
| Window absent from the response | Hide the row entirely |
| Both present | Show both rows |
| Neither present | Show "no active window" for that layer |

The distinction between *absent* and *zero* is deliberate. A Claude 5-hour window that has
not started yet is absent and hides; a 5-hour window that has started but consumed nothing
reports 0% and shows. The layer never reserves an empty placeholder slot, so the panel
grows and shrinks with whatever the account actually reports.

### 3.6 Polling and quota

Usage endpoints report the meter; they do not perform inference and therefore must not
consume 5-hour or weekly quota. **This is an assumption until proven.** The first
implementation task is to poll each endpoint ~10 times consecutively and confirm
`used_percent` does not advance.

If polling proves to be metered, the fallback is to source Codex from session JSONL (zero
network) and lengthen the Claude interval, surfacing the change to the user rather than
silently burning quota.

Endpoints may separately return HTTP 429. One request per 60 seconds, only while visible,
is far below any plausible threshold. A 429 is treated as a network failure (§9).

## 4. Detection

| Source | Signal | Layer |
|---|---|---|
| Claude desktop app | `claude` process with non-empty main window title | Claude |
| Claude Code CLI | `claude` process | Claude |
| Claude VSCode extension | `~/.claude/ide/*.lock` with a **live PID**, and `Code` running | Claude |
| ChatGPT app | `ChatGPT` process | OpenAI |
| Codex CLI | `codex` process | OpenAI |
| Codex VSCode extension | `codex-code-mode-host` process | OpenAI |

Rules:

- The Claude layer shows if any Claude signal is true.
- The OpenAI layer shows if any OpenAI signal is true.
- The window hides entirely when no signal is true.
- Detection runs every 5 seconds (cheaper than the usage poll; drives show/hide latency).

**The live-PID check is mandatory.** Lock files are not cleaned up on exit; treating
file existence as presence would pin the Claude layer on permanently.

**Open verification item:** it is not yet established whether the `claude` process with
window title "Claude" is the desktop app or a titled CLI window. Implementation must
determine this first. If the two cannot be distinguished reliably, both collapse into a
single "Claude active" signal, which changes no user-visible behavior since both map to
the same layer.

## 5. Lifecycle

One process. Starts at login, idles in the tray. The overlay window is shown or hidden;
it is never spawned or killed per-event.

- Usage polling runs **only while visible**, so no quota or network is consumed when hidden.
- On becoming visible, fetch immediately rather than waiting for the next tick.
- "Closing" the overlay hides it; exit is available only from the tray menu.

## 6. Window behavior

### Size states

| State | Size | Content |
|---|---|---|
| Bubble | 56 x 56 px | Worst-case percentage across visible layers |
| Compact | 320 x auto | One row per layer: name, worst window, reset |
| Square | 380 x 380 px (max) | All windows per layer, reset times, last-updated |

Square is a hard maximum, so fullscreen is structurally impossible. The window is
frameless with no default controls; all controls are custom.

### Placement

- User selects a **monitor** and a **corner**.
- The preferred monitor is stored by stable ID.
- If that monitor is disconnected, the window re-anchors to the **same corner** of any
  available monitor.
- When the preferred monitor reconnects, the window **returns to it automatically**.
- Scale is adjustable from 75% to 150%.

### Other

- Always-on-top toggle.
- Offscreen peek: slides mostly off the chosen edge, leaving a grab sliver.
- Visible on all virtual desktops.
- `skip_taskbar` is set, so no taskbar button and no alt-tab slot. The tray icon is
  present only while the app is active.

## 7. Configuration

Persisted as JSON in the app config directory:

```json
{
  "monitorId": "\\\\.\\DISPLAY1",
  "corner": "bottom-right",
  "scale": 1.0,
  "sizeState": "compact",
  "alwaysOnTop": true,
  "offscreenPeek": false,
  "pollIntervalSec": 60,
  "detectIntervalSec": 5
}
```

Unknown or invalid values fall back to defaults rather than failing to start.

## 8. Security

- Token files are opened **read-only** and never written.
- Tokens are never logged, never included in error messages, and never persisted by
  this app.
- Tokens are transmitted only to `api.anthropic.com` and `chatgpt.com` over HTTPS.
- **Tokens are never refreshed by this app.** Performing a refresh could invalidate the
  live CLI sessions that own them. On a 401 the UI shows a "re-authenticate in the CLI"
  hint and enters `Error` state.

## 9. Error handling

| Condition | Behavior |
|---|---|
| Network failure | Keep last good values, mark `Stale`, show age |
| 401 / expired token | Mark `Error`, show re-auth hint, keep last values dimmed |
| Malformed response | Mark `Error`, log shape mismatch without token or body contents |
| Codex endpoint down | Fall back to session JSONL, mark `Stale` |
| No config / first run | Defaults: bottom-right, primary monitor, compact, 100% |
| Credentials file missing | Layer shows "not signed in" instead of an error |

Values are never blanked out on failure. A stale number with a visible age is more useful
than an empty box.

## 10. Accessibility

- All custom controls are real `<button>` elements with `aria-label`.
- Usage bars use `role="progressbar"` with `aria-valuenow`/`min`/`max` and an
  `aria-valuetext` such as "25 percent used, resets in 3 days".
- A `aria-live="polite"` region announces refreshes only when a value actually changes,
  to avoid announcing every minute.
- Full keyboard operation, with a visible focus ring meeting contrast requirements.
- `prefers-reduced-motion` disables slide and pulse animations.
- Text and bar colors meet WCAG AA in both light and dark themes.
- Percentage is never conveyed by color alone; text always accompanies it.

## 11. Responsiveness

Layout is driven by container queries on the panel, not viewport media queries, since the
window is resizable and scalable independently of the screen. Layers stack vertically in
compact and square states. Text scales with the user's scale setting without clipping or
overflow at any supported size.

## 12. Testing

Per project rules: TDD, 80% minimum coverage.

**Rust unit tests (fixtures, no network):**
- Detection with a fixture directory containing both stale and live lock files.
- Detection when `Code` is running but no lock file exists, and vice versa.
- Parsing the verified Codex payload, including `secondary: null`.
- Parsing a payload with zero windows.
- A window present at 0% parses to a rendered row (not treated as absent).
- A window absent from the response produces no row.
- Both windows present produce two rows, for each provider.
- Corner math when the preferred monitor is absent, and when it returns.
- Config loading with missing, partial, and invalid values.

**TypeScript unit tests:**
- Size-state transitions.
- `resets_at` formatting across boundaries (seconds, hours, days, past).
- Stale/error state derivation from `fetched_at`.
- Worst-case percentage selection for the bubble.

**Accessibility tests:**
- axe run against each size state and both themes.
- Keyboard traversal of all controls.

**Integration:**
- Provider fetch against a mock HTTP server for success, 401, timeout, and malformed body.

## 13. Risks

| Risk | Mitigation |
|---|---|
| Usage endpoint shapes are inferred, not verified | Confirm both live before building UI; parsers are isolated behind the normalized model, so only one module changes |
| Endpoints are undocumented and may change | Codex has the JSONL fallback; Claude degrades to `Error` with last known values retained |
| Polling might consume quota | Unproven assumption; verified empirically as the first implementation task (§3.6), with a zero-network fallback if it fails |
| Process-name detection may catch unrelated processes | Match on executable path, not name alone |
| Desktop app vs CLI ambiguity for `claude` | Collapses to one signal with no behavior change (§4) |

## Phase 0 results

The live probe was run on 2026-07-31. Claude returned HTTP 200 with the expected
`five_hour.utilization`, `seven_day.utilization`, and RFC-3339 `resets_at` fields. The
implementation converts those reset strings to Unix seconds. A reliable no-metering verdict
was not recorded because later probe requests did not return another valid usage payload.

The Codex endpoint returned HTTP 403 on this machine. The implementation therefore treats
the endpoint as optional and falls back to the newest `~/.codex/sessions/**/*.jsonl`
`rate_limits` object, marked `Stale`, which is the verified local source.

## Approved UI refinements (2026-08-01)

- The overlay is intentionally minimal: no `LIVE QUOTA` title, status line, source count,
  default window controls, or inline settings section.
- Claude and ChatGPT/Codex are rendered as separate provider cards. Each provider's windows
  are circular meters with the percentage in the center; the 5-hour and weekly windows sit
  side by side.
- The two approved layouts are `stacked-compact` (default) and `provider-columns`, selected
  instantly from the tray-launched settings popup.
- The minimize control is a quiet inline arrow. The minimized state is a small two-dot
  provider-status pill and never renders the full overlay grid or a fabricated percentage.
- 5-hour windows show a live `HH:MM:SS` countdown to reset. Weekly windows show the local
  reset date and time. The previous last-updated footer is removed.
- Settings open in a separate popup from the tray icon only. Changes save immediately; the
  monitor control is a friendly dropdown populated from the currently available displays.
