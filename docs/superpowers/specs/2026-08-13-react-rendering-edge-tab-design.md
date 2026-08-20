# React Rendering Foundation and Screen-Edge Tab

## Goal

Replace the imperative frontend with React and TypeScript without changing current usage semantics, make every visible circle and rounded surface antialiased at Windows display scales, and add a minimal screen-edge tab that can hide the on-screen overlay completely.

This is the first dependency branch in the broader Usage Tracker expansion. It establishes stable UI, window, geometry, and presentation boundaries for later themes, compact indicators, authentication, billing, history, notifications, automation, popovers, and localization.

## Scope

This branch delivers:

- a complete React + TypeScript migration for the overlay and Settings window;
- behavior parity for provider activation, loading/error states, card rendering, independent provider bubbles, morph animations, geometry updates, and account settings;
- CSS-owned antialiased rounded and circular edges, no longer clipped by a binary rounded GDI region;
- a separate always-on-top screen-edge tab window;
- animated overlay hide/reveal with a smoothly reversing arrow;
- independent `Show tray indicator` and `Show screen overlay` settings;
- safe fallback behavior when a selected presentation surface cannot be created;
- automated and live Windows regression coverage for the new contracts.

This branch does not add the glow theme, used/remaining display, dynamic compact tray styles, billing data, per-model limits, history, notifications, shortcuts, new authentication methods, popovers, or localization. Those remain separate feature branches on top of this foundation.

## Product decisions

- The product remains single-account.
- The frontend migrates fully to React rather than mixing React and imperative component renderers.
- Rust/Tauri continues to own native windows, provider polling, credentials, persistence, and Win32 integration.
- The tab side follows the configured corner: left corners use the left edge and right corners use the right edge.
- The tab's vertical anchor follows the configured top or bottom corner.
- Screen-overlay visibility is session state. A process restart begins unhidden so the application cannot remain unexpectedly invisible after login.
- Tray and screen-overlay surfaces are independently configurable, but at least one must remain enabled.
- The automatic session initializer and all billing/history work are outside this branch.

## Window architecture

The process owns three Tauri windows:

1. `main` hosts the persistent provider overlay.
2. `settings` hosts application settings.
3. `edge-tab` is a narrow, transparent, always-on-top controller flush with the selected monitor's usable left or right edge.

The `edge-tab` window is independent of `main`. Hiding the overlay therefore calls `hide` on the main native window after its exit animation; there is no transparent overlay HWND left covering desktop space. The tab remains interactive because it has its own minimal HWND.

The tab uses the configured monitor's work area so it does not cover a taskbar docked to a screen edge. Its horizontal coordinate is exactly the left work-area edge or the right work-area edge minus the tab width. Its vertical coordinate is the existing corner margin from the top or bottom work-area edge.

The tab is visible whenever `showScreenOverlay` is enabled. This preserves access in overlay-only mode even when no provider process is active. When no provider is active, the main overlay remains hidden and the tab's context menu still exposes Settings and Quit.

The edge tab has a 24-by-48 logical-pixel native target with a narrower painted silhouette. The screen edge supplies an effectively unbounded target on one side while the 24-by-48 target meets WCAG 2.2's minimum target-size criterion. Its button is keyboard focusable and has a visible internal focus treatment.

## React frontend

### Entry point

One TypeScript entry point reads the current Tauri window label and mounts exactly one root:

- `OverlayApp` for `main`;
- `SettingsApp` for `settings`;
- `EdgeTabApp` for `edge-tab`.

Development Strict Mode remains enabled. Native subscriptions and controllers must therefore have symmetric, idempotent setup and cleanup so the development-only effect replay cannot duplicate listeners or native commands.

### State boundary

A framework-independent application store owns normalized frontend state and exposes it through `useSyncExternalStore`. The store receives Tauri bootstrap data and events, applies the existing pure state transformations, and publishes immutable snapshots.

React components do not call provider APIs or mutate native geometry directly during render. They dispatch typed actions to controllers:

- `UsageController` subscribes to bootstrap, source, and usage events.
- `GeometryController` measures committed DOM nodes and sends coalesced geometry requests.
- `MorphController` owns the existing provider-card-to-bubble Web Animations sequences.
- `OverlayVisibilityController` coordinates edge-tab and tray hide/reveal requests with Rust.
- `SettingsController` persists validated settings and applies live changes.

### DOM measurement and animation

Geometry measurement runs in `useLayoutEffect` after React commits stable provider nodes and before paint. Provider components use stable provider keys, and the controller receives refs to cards, bubbles, and their container. React never replaces a provider node merely because its numeric usage changed.

The current Web Animations implementation remains an imperative controller rather than being rewritten as render-time React state. It receives explicit source and destination elements, captures computed material and geometry, runs the animation, and always cleans up ghosts in a `finally` path.

Focus transfer remains provider-specific:

- minimizing a card moves focus to that provider's resulting bubble;
- restoring a bubble moves focus to that provider's minimize button;
- hiding the full overlay leaves focus on the edge-tab button;
- revealing the overlay does not steal focus unless activation originated from keyboard input.

## Antialiased surface contract

### Root cause

The current main window combines `CreateRoundRectRgn` regions and applies them with `SetWindowRgn`. GDI regions are binary masks. When their integer rounded boundary intersects a CSS border radius or circle, they remove WebView2's partially transparent edge pixels and produce visible stair-stepping, especially on the 48-pixel provider bubble and at fractional display scales.

### New ownership rule

CSS/WebView2 owns every visible curve. Native Windows regions only limit the broad area in which a surface can paint and receive input.

For each measured card, bubble, or tab surface, the frontend sends a logical coverage rectangle plus an effect outset. Rust converts it to physical coverage using containment rounding:

- left and top use `floor`;
- right and bottom use `ceil`;
- width and height are derived from those endpoints;
- finite and overflow validation remains mandatory.

Rust combines `CreateRectRgn` regions rather than `CreateRoundRectRgn` regions. The CSS curve and its antialias fringe remain completely inside the native region. Transparent gaps between separate cards stay outside the combined region and remain non-interactive. Transparent pixels in a card's small corner square are accepted as the bounded cost of preserving antialiasing; the large invisible animation headroom remains excluded.

`effectOutset` is zero for ordinary branch-one surfaces. The contract exists now so the later glow theme can request enough native coverage for a halo without changing geometry serialization again.

The Settings window keeps its inset panel coverage rectangle. Because that rectangle remains inside the native window bounds, Windows' outer non-client border stays clipped away, while the CSS panel radius is free to antialias inside the rectangular coverage.

Transient morph coverage remains a rectangle that contains the entire animation. Exact steady-state coverage is restored only after final geometry has been applied.

## Edge-tab interaction

The edge-tab arrow communicates the action that will occur:

- with a visible overlay, it points toward the configured screen edge, meaning “hide toward this edge”;
- with a hidden overlay, it points inward, meaning “reveal from this edge.”

The arrow rotates 180 degrees with the same 200-millisecond cubic easing used for the native slide. With reduced motion, both changes are immediate.

Rust owns a single `OverlayVisibilityState` containing:

- requested screen-overlay enablement;
- current provider availability;
- current user-hidden state;
- current animation direction and generation;
- last stable overlay position.

Hide and reveal commands are idempotent. A new command during an animation invalidates the old generation and continues from the current interpolated position rather than snapping to an endpoint.

Hide sequence:

1. The tab changes to its inward-arrow state.
2. Rust moves the main HWND toward the configured edge while fading it over 200 milliseconds.
3. Rust hides the main window at completion and restores its canonical geometry off-screen from the user.
4. Provider polling behavior remains unchanged in this branch; revealing requests an immediate refresh.

Reveal sequence:

1. Rust applies current monitor, scale, and corner geometry while the window is hidden.
2. Rust shows the main window at the edge-offset start position with zero opacity.
3. Rust moves and fades it to its canonical position.
4. The tab changes to its edge-pointing state.

If native animation fails, the controller settles to the requested final state, restores full opacity and canonical geometry, emits a sanitized diagnostic, and never leaves a half-visible window blocking the desktop.

## Presentation settings

The persisted configuration adds:

```json
{
  "showTrayIndicator": true,
  "showScreenOverlay": true
}
```

Missing values migrate to `true`, matching the existing product's tray-plus-overlay behavior. Sanitization rejects a configuration in which both are false by restoring `showTrayIndicator` to true.

The Settings UI prevents the user from disabling the last enabled surface and explains why. Changes apply immediately:

- disabling the screen overlay hides both `main` and `edge-tab`;
- enabling it creates or reveals the tab and resumes normal provider-driven main-window visibility;
- disabling the tray indicator removes the tray icon;
- enabling it recreates the current static tray icon and menu.

Dynamic tray icon styles arrive in the later display-system branch. Branch one preserves the current tray artwork and commands.

When the tray is disabled, the edge tab supplies a minimal native context menu containing Show/Hide Overlay, Settings, and Quit. This guarantees that overlay-only mode remains controllable. The later popover branch may extend the same menu with Refresh without changing this fallback contract.

## Data and event flow

1. Rust creates the configured presentation surfaces and emits bootstrap data.
2. The window-specific React root starts its controller and subscribes to the shared store or visibility events.
3. Provider detection updates the store; `OverlayApp` renders stable provider-keyed components.
4. `useLayoutEffect` measures committed surfaces and queues one latest-wins geometry request.
5. Rust converts logical coverage to physical rectangular regions, applies size and position, then applies the combined native region.
6. Settings changes persist in Rust and are broadcast to all windows so tab side, overlay position, tray visibility, and React appearance update together.
7. Tab or tray visibility commands pass through the one Rust visibility state machine, preventing conflicting window transitions.

No window maintains an independent authoritative configuration copy. React may render optimistically, but the sanitized Rust configuration broadcast becomes canonical.

## Failure handling

- If React root initialization fails, the affected window renders a minimal static error with a retry action and emits a sanitized diagnostic.
- If the edge-tab window cannot be created while the tray is disabled, Rust forces the tray visible for that process and reports the fallback.
- If the configured monitor disconnects, both main and tab re-anchor to the same corner of the fallback monitor; they return when the preferred monitor reconnects.
- If geometry is invalid or empty, Rust keeps the last known-good coverage rather than expanding to the full transparent HWND.
- If a React effect is replayed in Strict Mode, cleanup removes the previous listener before a replacement is installed.
- If a morph is interrupted by a full-overlay hide, the morph controller removes ghosts, restores destination opacity, and yields ownership to the visibility controller.
- If settings persistence fails, controls revert to the last canonical config and show an inline error rather than claiming the change was saved.

## Accessibility

- Every interactive surface is a native HTML `button` or native menu item.
- The edge tab exposes `Hide usage overlay` or `Show usage overlay` according to state.
- The arrow is decorative and never the only accessible label.
- Focus indicators render inside native coverage and cannot be clipped.
- Keyboard activation supports Enter and Space.
- Reduced motion removes morph, slide, fade, and arrow rotation while preserving state changes.
- Existing meter progress semantics and provider-specific button labels remain unchanged through the React migration.
- Automated axe coverage runs against overlay states, provider bubbles, Settings pages, and both edge-tab directions.

## Testing strategy

Implementation follows red-green-refactor. Existing tests are migrated before their corresponding imperative renderer is removed.

### TypeScript tests

- window-label routing mounts the correct React root;
- bootstrap and Tauri events produce the same provider state as the current store;
- usage-only changes preserve provider DOM node identity;
- independent provider minimize/restore and focus transfer remain intact;
- geometry effects emit one coalesced request after committed layout;
- Strict Mode setup/cleanup does not duplicate event listeners or commands;
- tab arrow direction matches side and visibility state;
- reduced motion skips movement but reaches the same final state;
- settings reject both presentation surfaces being disabled;
- settings changes update all mounted window stores;
- tray-only, overlay-only, and both configurations render the expected controls;
- interruption removes morph ghosts and restores opacity;
- axe reports no violations for supported states.

### Rust tests

- logical rectangles convert with floor/ceil containment at 100%, 125%, 150%, and 200% scale;
- fractional coordinates never clip the requested logical bounds;
- combined native coverage uses rectangular surfaces and preserves disjoint gaps;
- Settings coverage remains inset from the outer window frame;
- invalid and overflowing coverage is rejected without losing last-known-good state;
- left/right and top/bottom tab placement follows the configured corner and monitor work area;
- presentation config migration and the at-least-one invariant;
- visibility commands are idempotent and reversible mid-animation;
- failed tab creation forces a tray fallback;
- monitor disconnect/reconnect keeps tab and overlay on the same selected monitor;
- native animation failure settles opacity, position, and visibility consistently.

### Build and live verification

The branch is not complete until all existing and new frontend tests, Rust tests, coverage, and production builds pass.

Live Windows verification uses the packaged executable at 100%, 125%, 150%, and 200% display scaling on contrasting light and dark desktop backgrounds. It covers:

- every card corner and circular provider bubble without a binary staircase or clipped CSS border;
- no drift between CSS surfaces and native coverage;
- transparent gaps between cards remaining non-interactive;
- the provider bubble's bounded 48-by-48 footprint;
- the hidden main HWND no longer blocking desktop input;
- edge-tab placement at all four configured corners;
- rapid hide/reveal reversal without snapping or stranded opacity;
- keyboard, focus, reduced-motion, tray-only, overlay-only, and both modes;
- taskbar positions and monitor disconnect/reconnect;
- settings and native frame repair across repeated focus changes.

## Rollout and compatibility

The feature branch is `codex/feature-react-rendering-foundation`.

The migration is performed by window and component while preserving a continuously buildable branch. Imperative production renderers are removed only after the React equivalent passes the migrated behavior tests. Pure formatting, state, geometry, and animation helpers are retained where their contracts remain valid.

Configuration loading remains backward compatible. No credential or usage-history format changes occur in this branch. If an unforeseen React regression prevents a window from mounting, Rust lifecycle and tray controls remain available for recovery.

## Acceptance criteria

- The frontend contains no remaining imperative component renderer or `innerHTML`-driven application view.
- Current provider, settings, animation, geometry, and accessibility behavior is preserved unless this document explicitly changes it.
- CSS owns every visible curve; steady-state Windows coverage does not use `CreateRoundRectRgn` for application surfaces.
- Rounded cards and circular bubbles retain antialiased edge pixels at all supported scales.
- Hiding from the edge tab hides the main native window and releases its desktop footprint.
- The tab follows the configured corner side and top/bottom anchor and reverses its arrow smoothly.
- Tray-only, overlay-only, and both modes work, and the application never saves a configuration with no control surface.
- All automated checks and the packaged Windows verification matrix pass before merge.
