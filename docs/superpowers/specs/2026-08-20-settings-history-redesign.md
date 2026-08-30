# Settings and History Redesign

## Outcome

Settings and History become two expressions of one custom desktop product: a compact telemetry atelier built for people who want usage information to feel immediate, legible, and calm. The redesign replaces the current generic form/dashboard presentation without changing the backend command contract or stored configuration.

## Design direction

The visual language is an instrument panel filtered through an editorial layout rather than a conventional admin dashboard.

- Near-black blue surfaces create depth with subtle grain-like gradients, not glass everywhere.
- Warm signal amber identifies actions and selection; cyan and coral distinguish data series.
- `Segoe UI Variable` carries interface copy and `Cascadia Code` carries measurements, timestamps, and compact labels so the app stays offline and adds no font-loading failure mode.
- Hairline borders, inset highlights, deliberate asymmetry, and strong number typography create a recognizable desktop-tool identity.
- Corners are controlled and varied: the application shell is softly rounded, while field groups and analytics panels use smaller radii. Pills are reserved for statuses, ranges, and compact filters.

## Settings information architecture

The 600×560 window remains a focused control surface.

- A slim drag header contains product identity, an autosave status, and the close action.
- The left rail uses icons, short labels, and a bright active marker. History is visually separated as a destination rather than impersonating a settings tab.
- Each settings page begins with a numbered overline, title, and one-line purpose.
- Related controls are grouped into bordered setting rows with explanatory text. Toggle switches, selectors, range controls, and theme previews use a consistent tactile language.
- The theme page shows meaningful miniature overlay previews. Behavior separates automation, shortcuts, and runtime health instead of presenting one undifferentiated form.
- Account states remain inline and preserve the existing Claude authentication workflow.
- Existing instant-save semantics remain unchanged.

## History information architecture

History becomes an analytics workspace sized for its 960×680 resizable window.

- A sticky editorial header pairs the title and current range with compact provider/series filters.
- Range selection is a segmented timeline control rather than a row of default buttons.
- Summary metrics are promoted into scan-friendly cards with units, availability states, and restrained visual accents.
- The chart sits in a wide hero panel with grid lines, a proper legend, visible axes, a gradient-underlay line treatment, and keyboard-accessible inspection points.
- Model and billing information use custom data panels with strong row rhythm and tabular numerals.
- Export and destructive clearing actions live in a dedicated utility panel at the bottom, with a designed modal instead of an unstyled dialog block.
- Empty, loading, and error states occupy the same panel geometry so the page does not jump.

## Motion and scrolling

Lenis owns only `.settings-pages` and `.history-scroll`, leaving native controls and nested lists predictable. It is driven by GSAP's ticker and destroyed whenever a surface is replaced or unmounted.

GSAP provides:

- a short shell reveal followed by staggered navigation and content entrances;
- directional settings-page transitions based on rail order;
- range/filter feedback and chart line drawing;
- metric-card and table-row entrances after data resolves;
- modal entrance/exit choreography.

Motion uses opacity plus transforms, never layout-affecting width/height animation. `prefers-reduced-motion` disables Lenis smoothing and makes GSAP apply final states immediately. Scroll and animation enhancements are non-essential: controls remain functional if either library cannot initialize.

## Accessibility

- Existing tab, combobox, chart-point, dialog, table, and live-region semantics are preserved.
- Every interactive target remains at least 36×36 CSS pixels in the dense desktop layout, with visible focus rings.
- Selected states never rely on color alone; they include shape, text, or marker changes.
- Chart series retain textual summaries and expose point values through real buttons.
- The confirmation modal traps intent through initial focus, Escape support, and focus restoration.
- Contrast targets WCAG AA for normal text, and all motion respects the operating-system preference.

## Architecture

The redesign keeps the stable boundary already used by production:

- `renderSettings` remains the synchronous Settings renderer so native controller wiring and focused unit tests do not regress.
- A shared motion module enhances the returned DOM and returns a cleanup callback.
- History remains React and uses a small hook around the same motion module.
- Shared design tokens and surface classes live in the existing style sheets.

This avoids a high-risk simultaneous rewrite of settings behavior while still giving both surfaces a single design and motion system. Completing the full Settings React migration can follow as a separate architectural change once the production React entrypoint owns all existing native synchronization.

## Approaches considered

1. **Rewrite Settings in React while redesigning.** Clean long-term component structure, but combines behavior migration, native synchronization, design work, and animation integration in one change. Rejected for this pass because the existing React Settings shell does not implement most production controls.
2. **CSS-only reskin.** Lowest code risk, but cannot deliver the requested smooth scrolling, meaningful transitions, or improved content hierarchy. Rejected.
3. **Progressively enhance the stable Settings renderer and redesign History in React.** Preserves behavior, enables one visual language, and keeps motion disposable and testable. Selected.

## Verification

- Add failing component and stylesheet contract tests before each implementation slice.
- Keep axe coverage for Settings, History, chart, billing, export, and modal states.
- Run the complete Vitest suite, TypeScript/Vite production build, Rust tests, and Clippy.
- Build and install the Windows bundle, then inspect Settings and History in the running Tauri application at their minimum and default sizes.
