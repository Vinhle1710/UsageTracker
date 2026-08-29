# Design system

Two visual worlds live in this app. Do not mix them.

| Surface | Windows | Character |
|---|---|---|
| **Overlay** | `main`, `edge-tab`, `popover` | Floats on the desktop. Translucent, provider-coloured. Tokens: `--bg`, `--surface`, `--accent`, `--claude`, `--chatgpt`. |
| **App windows** | `settings`, `history` | Ordinary windows. Opaque neutral field, hairline separation, one accent. Tokens: `--app-*`. |

Source of truth is [`src/styles/tokens.css`](src/styles/tokens.css). This file records *why*; the
tokens file records *what*. Never hardcode a value that a token already names.

## Direction — app windows

Content is the interface; chrome recedes. The reference point is a native settings window, not a
marketing page. Concretely that means **no** ambient gradients, grain, graph-paper overlays,
notched corners, glow shadows, decorative section numbering, or invented brand marks. All of those
were removed in this pass — they are what made the surfaces read as generated.

*Deviation on record:* the `frontend-design` skill recommends atmosphere (gradients, texture,
layered transparency) by default. For these two windows the brief wins: atmosphere was the problem.
It still applies to the overlay, which genuinely floats over a desktop.

## Decisions, and what was rejected

**Type — one family.** `--font-ui` (Segoe UI Variable) for everything a person reads;
`--font-data` (Cascadia Code, tabular figures) *only* for numbers that get compared — metric
values, chart axes, amounts, percentages. Applied via `.telemetry-value`.
*Rejected:* a display/body pairing (at 700–1100px window widths a second family reads as
decoration), and monospace on labels and prose, which was the loudest generated tell.

Scale: `--text-xs` 11 → `--text-2xl` 28. **11px is the floor.** The previous design set meaningful
text at 7–9px.

**Colour — accent is blue on purpose.** Amber and teal already mean "Claude" and "ChatGPT" on every
chart, card, and bubble. Spending either on buttons makes ordinary chrome look like a provider
reading, so interactive chrome is `--app-accent` / `--app-accent-fill` and the provider hues stay
semantic.
*Rejected:* the previous amber `--signal` as global accent.

The accent is **two tokens, not one**: a blue bright enough to read as text on a dark field is too
light to carry white text. `--app-accent` is for text, icons, and borders; `--app-accent-fill`
backs solid buttons. Each clears 4.5:1 for its own job.

**Space — 4px rhythm.** `--s1` 4 → `--s10` 40. *Rejected:* the previous ad-hoc
3/5/7/9/11/13/17/23/27px values.

**Finish — hairlines, not glow.** `--app-line` separates; shadows appear only on things that
genuinely float (dropdown, modal). Radius: `--r-sm` 6 controls, `--r-lg` 12 cards, `--r-xl` 16
window.

**Motion — opacity-led and short.** `--ease` `cubic-bezier(.32,.72,0,1)`, `--dur` 220ms. GSAP
entrance is 8px / 0.44s / 0.03s stagger. Lenis runs light (`lerp: 0.1`) because it is replacing an
instant OS scroll. Motion fires **once per completed data load**, never on every state tick.

## Non-negotiable

- **4.5:1 on all text**, checked against the surface it actually sits on — not just the canvas.
  A raised card is a different background than the window. Verified with axe in a real browser
  across both themes; jsdom's `color-contrast` rule is disabled and proves nothing.
- **Both themes.** Every colour is a token with a `prefers-color-scheme: light` counterpart.
  Never write a raw hex in `app.css` below the overlay section.
- **Never colour alone.** Selected theme = border + inner ring + checkmark. Chart series are
  labelled in the legend and in the SVG `<desc>`.
- **One filled action per view.** Destructive actions stay outlined and never take the primary slot.
- **Hand-rolled dialogs use `trapFocus`** ([`src/focus-trap.ts`](src/focus-trap.ts)). A native
  `<dialog>` opened with `showModal()` already traps and restores focus; a `div` does not.
- **Text belongs in HTML, not in a scaled SVG.** Text inside a `viewBox` grows with the chart —
  chart axis labels are HTML positioned over the plot for exactly this reason.

## Contracts that look like style but are not

`src/styles/app-css.test.ts` pins these because the Rust side derives native window regions from
them. Changing either half alone crops the window.

- `#app[data-window="settings"]` → `padding: 10px` and `.settings-window` → `border-radius: 16px`
  mirror `SETTINGS_PANEL_PADDING` / `SETTINGS_PANEL_RADIUS` in `material.rs`.
- `#app` → `--overlay-headroom` must equal `OVERLAY_HEADROOM` in `src/geometry.ts`.
- `.history-scroll` must keep `width` first, plus `height: 100%` and `overflow-y: auto`.

Note `ruleFor()` in that test matches the **first** substring occurrence, so a grouped selector
ending in a pinned name (`.a, .history-utility {`) will shadow the real rule. Keep pinned selectors
in their own rules.
