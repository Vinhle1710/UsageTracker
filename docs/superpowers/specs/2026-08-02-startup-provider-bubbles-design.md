# Reliable Startup and Provider Bubbles

## Goal

Make the hidden tracker listener reliably start with Windows, reveal the overlay immediately when Claude or ChatGPT becomes active, prevent native title bars during focus changes, and replace the shared minimized state with independent provider bubbles.

## Confirmed failures

### Windows startup

The installed startup registry entry is currently:

```text
C:\Users\<user>\AppData\Local\Usage Tracker Overlay\usage-tracker-overlay.exe 
```

The executable path contains spaces but is not quoted. The `auto-launch` dependency used by `tauri-plugin-autostart` writes the executable and arguments as one unquoted string, so Windows cannot reliably execute this command at sign-in.

### Delayed reveal

Process detection already runs once per second, but the native window remains hidden until both the WebView and a complete usage-fetch cycle are ready. Network or credential delays therefore make a correctly detected provider look as if it was never detected.

### Native frame after focus loss

The current window-event handler repairs the overlay only for `Focused(true)`. It does not repair either app window after focus loss, matching the reported title bar that appears when another application receives focus.

## Architecture

Keep the recovered single Tauri process and one transparent overlay window. Do not reintroduce native provider helper windows and do not migrate to Electron or WinUI.

Native Tauri, Electron background material, and `window-vibrancy` effects operate on a complete native window. Electron therefore does not provide native per-DOM-card Acrylic. WinUI can use Acrylic brushes on controls, but adopting it would require a full application rewrite. The existing card-local CSS Frosted and Blur themes remain the appropriate implementation for this overlay.

## Startup and activation

- Release builds register a Windows `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value from Rust during setup.
- The command quotes the complete executable path and contains no trailing empty argument.
- Debug builds never replace the installed startup entry with a development executable.
- The frontend no longer owns startup registration.
- The Rust process detector starts during application setup and continues checking every second while the tracker is in the tray.
- When a provider becomes active, the overlay reveals as soon as the WebView is ready. It renders cached usage when available or that provider's loading state while polling runs.
- Provider activation wakes the usage poller immediately. API completion updates the existing provider card or bubble in place; it does not control whether the window is visible.
- The process remains hidden in the tray when neither provider is active.
- Tray `Show/Hide` remains an explicit manual override. The override resets after all providers become inactive, preserving the current behavior.

## Borderless lifecycle

- Both the `main` and `settings` windows are repaired on focus gain and focus loss.
- Repair strips caption, system menu, resize frame, minimize button, and maximize button styles and disables native shadow/non-client rendering.
- A deferred repair runs after the focus event so Windows or WebView2 cannot restore a frame after the synchronous callback completes.
- The main overlay reapplies its cached card region after a frame repair. Settings remains a rectangular custom window without native decorations.
- Existing show, geometry, settings-close, minimize, and tray restore paths continue to enforce the same borderless contract.

## Provider-specific minimize state

Collapse state is keyed by provider (`claude` or `openai`) instead of one global boolean.

- Every expanded card owns a provider-bound minimize button.
- Minimizing Claude does not modify the ChatGPT card, and minimizing ChatGPT does not modify the Claude card.
- A minimized provider is represented by a 48-pixel themed button containing its supplied logo.
- Clicking the bubble restores only that provider.
- Collapse state lasts for the current tracker process. If a provider closes and returns during that session, it returns in its previous expanded or minimized state.
- Provider data, colors, logos, meters, and collapse state are always selected by provider identity rather than card position.

## Bubble layout

- Minimized bubbles form a horizontal row in stable Claude-then-ChatGPT order.
- The row starts at the overlay edge that matches the selected screen corner and remains pinned over the top corner of the overlay.
- A bubble occupies the location used by the current shared minimize control.
- If one provider is expanded while the other is minimized, the expanded card remains in normal flow beneath the bubble row. Its minimize button shifts inward so it cannot overlap a bubble.
- If both providers are minimized, the native window shrinks to the bubble row.
- If no providers are minimized, the existing vertical or horizontal card layout is unchanged.
- Bubble regions participate in measured window clipping, so transparent unused host space remains invisible and non-interactive.

## Theme and motion

- Provider bubbles inherit Translucent gradient, Frosted, Blur, and Solid appearances from the active theme.
- Background color and card opacity affect bubbles instantly without changing the selected theme.
- Bubble collapse, expansion, hover, press, and focus feedback use short transform/opacity transitions.
- `prefers-reduced-motion` disables nonessential movement while retaining immediate state feedback.

## Accessibility

- Bubbles and minimize controls are native `button` elements.
- Controls expose provider-specific names such as `Minimize Claude usage` and `Expand ChatGPT usage`.
- Each target is at least 44 by 44 CSS pixels, supports keyboard activation, and has a visible focus indicator.
- Collapse and expansion preserve a logical focus destination: minimize moves focus to the resulting provider bubble; expansion moves focus to the restored provider's minimize control.
- Logo images are decorative within already-named buttons.
- Provider appearance is not communicated by color alone.

## Testing and live verification

Automated regression coverage must be written before production changes and include:

- quoted Windows startup command generation and debug-build exclusion;
- immediate reveal without waiting for usage readiness;
- frame repair on both focus states and deferred repair scheduling;
- independent provider collapse and restore;
- mixed expanded-card and bubble layout;
- two-bubble ordering and corner alignment;
- provider ownership across close, reopen, polling, and collapse changes;
- keyboard names, focus behavior, target size, reduced motion, and theme inheritance.

Live Windows verification uses the packaged executable and covers:

- the exact installed startup registry value;
- a real sign-in-compatible launch command;
- activation after starting Claude and ChatGPT independently;
- immediate loading/cached reveal followed by in-place usage updates;
- repeated focus changes with continuous Win32 style sampling;
- independent Claude and ChatGPT collapse/restore in both layouts;
- one expanded card with the other provider pinned as a bubble;
- both providers minimized to two logo bubbles;
- Settings, tray Show/Hide, monitor placement, and taskbar avoidance.

The release version and installer are updated only after the full frontend, Rust, accessibility, lint, build, and packaged-app checks pass.
