# Popover and localization acceptance

Automated checks cover typed fourteen-locale catalog/fallback, localized React surfaces, status precedence, accessible roles, popover geometry/state, and refresh coalescing. `npm test`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass in this environment.

## Manual-only Windows matrix

The packaged executable and a live Windows desktop are required to check tray hit-testing and OS geometry. On each supported taskbar edge and with monitor removal, verify: left-click toggles the attached popover; right-click shows Refresh, Settings, Quit in that order; refresh requests coalesce; stale/error banners retain last-good indicators; auto-size stays within 240–480 by 120–640; detach/drag/restart/attach restores and clamps position; and focus loss/Escape hide only attached popovers.

For every locale (`en`, `vi`, `es`, `fr`, `de`, `it`, `pt`, `pt-BR`, `ja`, `ko`, `zh-CN`, `zh-TW`, `tr`, `uk`), inspect clipping and confirm the native menu is rebuilt in the selected language.

Keyboard/screen-reader checks left to a Windows run: Tab reaches the dialog controls in logical order, focus is visible, Escape closes an attached popover and returns focus to the tray host, detached regions remain open on blur, and Narrator/another screen reader announces the dialog/region name, status updates, and button names without raw provider errors.
