# CSS Frosted and Blur Themes

## Goal

Replace the Windows-native Acrylic and Blur theme implementations with card-local CSS effects while preserving the recovered single-window borderless lifecycle.

## Theme model

The four visible presets remain:

- **Translucent gradient**: unchanged.
- **Frosted**: replaces Acrylic and uses a stronger CSS blur, saturation, and a light translucent color wash.
- **Blur**: uses a softer CSS blur and the selected card background color without a gradient.
- **Solid**: unchanged.

The stored theme identifier changes from `acrylic` to `frosted`. Existing `acrylic`, `opaque`, and old `custom` values migrate to `frosted`, so upgrades do not reset unrelated settings.

## Rendering

Both Frosted and Blur render on each complete provider card:

- Frosted uses a semi-transparent background derived from `--card-background`, `backdrop-filter: blur(18px) saturate(145%)`, and the WebKit-prefixed equivalent.
- Blur uses a semi-transparent background derived from `--card-background`, `backdrop-filter: blur(12px)`, and the WebKit-prefixed equivalent.
- Neither preset uses a gradient.
- Card opacity continues to control the card background alpha without changing the selected theme.
- The app root, gap between cards, and unused parts of the native window remain transparent.

For Frosted and Blur, Rust applies the existing native **Clear** material while retaining the existing rounded multi-card window region. This prevents native Acrylic/Blur from affecting the whole host rectangle and keeps title-bar, clipping, minimize, settings, tray, and positioning behavior on the recovered path.

CSS `backdrop-filter` only blurs pixels available to the WebView compositor. If WebView2 does not expose the Windows desktop as part of that backdrop, the cards will still show the translucent color treatment but desktop blur will not appear. This experiment will not add another native helper window as a fallback.

## Settings

- Rename Acrylic to Frosted.
- Update the Frosted and Blur previews to visually distinguish stronger frosted glass from softer blur.
- Keep theme changes and opacity changes instant.
- Preserve the existing keyboard, focus, pressed-state, and screen-reader behavior.

## Verification

- Rust tests cover theme migration and confirm Frosted/Blur both select native Clear.
- TypeScript tests cover the new `frosted` theme identifier, settings selection, instant opacity updates, and the four previews.
- Existing accessibility, geometry, provider ownership, polling, and window tests must remain green.
- Live Windows verification covers initial reveal, Frosted and Blur switching, card-only transparency, polling stability, minimize/restore, settings close, and tray hide/reopen.
- A release installer is built only after the untouched packaged executable passes the live window checks.
