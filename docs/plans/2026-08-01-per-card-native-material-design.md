# Per-card native material design

## Problem

The overlay currently applies Windows Acrylic or Blur to the main Tauri window and then clips that window to the measured provider cards. Live screenshots show that the compositor still treats the full rectangular host as the material surface. The result is a large purple or black rectangle, occasional split cards, and brief native-frame flashes while the host is reshaped or restored.

Changing tint or opacity cannot correct the surface boundary. Acrylic and Blur need separate native surfaces whose actual window bounds match the provider cards.

## Architecture

Use three coordinated windows:

- `main`: the existing transparent WebView foreground containing provider names, usage meters, reset labels, and the minimize control.
- `material-claude`: a native window with no WebView, sized to the Claude card only.
- `material-openai`: a native window with no WebView, sized to the ChatGPT card only.

The main window remains transparent and never receives a native Acrylic or Blur effect. The two material windows are placed immediately behind it in the Windows z-order. They are borderless, shadowless, excluded from the taskbar, non-focusable, and click-through.

The material windows are created hidden during application setup. They are shown only when their provider is active and the selected theme requires a native material.

## Provider identity

Every measured card rectangle sent by the frontend includes its provider identity:

```text
{ provider: "claude" | "openai", x, y, width, height, radius }
```

The backend updates the matching fixed-label material window. It never assigns card windows from array position. Usage events remain keyed by the same provider enum, and frontend reconciliation continues to locate cards by `data-provider`.

This prevents a provider disappearing, appearing, or changing order from transferring data, colors, or backdrop geometry to the other provider.

## Material behavior

- **Translucent gradient:** CSS-only provider gradient on the foreground card. Native material windows stay hidden.
- **Acrylic:** one full-card native Acrylic surface per active provider, with no CSS gradient over it.
- **Blur:** one full-card native low-tint Blur surface per active provider, with no CSS gradient over it.
- **Solid:** CSS-only single background color. Native material windows stay hidden.

The Background setting changes the card tint only. Card opacity controls the strength of that tint rather than making the main host visible. Native opacity is mapped to a useful translucent range so the desktop color remains visible instead of becoming an opaque purple or black block.

On Windows versions where generic Blur is unavailable, Blur falls back to Acrylic, matching TranslucentTB's compatibility behavior. The setting remains selected so Blur can be used automatically on systems that support it.

## Geometry and lifecycle

The frontend measures each rendered provider card after layout. The backend:

1. Calculates the main overlay position inside the chosen monitor work area, so no window covers the taskbar.
2. Positions each active material window at the main window origin plus its provider rectangle.
3. Sizes and rounds each material window to that rectangle.
4. Applies material only when the theme, tint, or opacity changes.
5. Places the material windows directly behind the main foreground without activating them.

The main foreground is no longer reshaped with `SetWindowRgn`, and native material is never reapplied to it. This removes the compositor operations responsible for split cards, full-host material, native-title flashes, and periodic visual disappearance.

All three windows hide together when the tray Show/Hide action is used or no supported provider is active. Material windows also hide when minimized to the side. Showing the overlay restores the backdrops first and the foreground last, without focusing a backdrop.

## Updates and animation

Usage updates continue to mutate existing provider and meter elements in place. A polling response must not replace the overlay root, provider card, or backdrop windows. Progress rings animate from their prior values, including reset-to-zero behavior, while geometry remains unchanged unless the provider set or layout actually changes.

## Accessibility and input

Only the foreground WebView participates in input and accessibility. Backdrop windows are decorative, click-through, and cannot receive keyboard focus. Existing progressbar roles, reset text, settings controls, and reduced-motion behavior remain unchanged.

## Verification

Tests will cover:

- Provider-tagged geometry serialization and conversion.
- Fixed provider-to-window mapping when either provider appears or disappears.
- Native material windows hidden for Translucent gradient, Solid, and minimized states.
- Blur fallback selection on unsupported Windows builds.
- No material or region application to the main WebView window.
- No DOM replacement for ordinary usage-percentage updates.
- Coordinated show, hide, position, and z-order planning.

Live verification will compare Acrylic and Blur against the supplied TranslucentTB references on the user's desktop. Acceptance requires:

- Desktop colors remain visible through Acrylic and Blur.
- Material exists only inside the rounded provider cards.
- The gap and unused host area are fully transparent.
- No native title bar appears at startup, after closing settings, or after hiding and showing the overlay.
- Cards never split into separate visual pieces.
- Claude and ChatGPT always retain their own data, logo, accent color, and backdrop.
- Minute polling animates progress without a full-window blink or redraw.
