# Settings and History Redesign Implementation Plan

> Execute every implementation task test-first. Each task begins with a focused failing test, proceeds with the smallest production change, and ends with the relevant tests green and a commit.

**Goal:** Deliver one distinctive, accessible Settings/History design with Lenis scrolling and GSAP motion while preserving all native behavior and backend contracts.

**Architecture:** Keep the synchronous Settings renderer and React History application. Introduce a shared progressive-enhancement motion layer used directly by Settings and through a React hook in History. Rebuild markup semantics and CSS around a shared telemetry-atelier design system.

**Stack:** TypeScript 5.6, React 19, Lenis, GSAP, Vitest, Testing Library, vitest-axe, Tauri 2, Rust.

---

### Task 1: Lock the shared surface contract

**Files:**
- Modify: `src/styles/tokens.css`
- Modify: `src/styles/app.css`
- Modify: `src/styles/app-css.test.ts`

1. Add failing CSS contract tests for the history/settings shell selectors, data typography, sticky history header, focus treatment, and reduced-motion overrides.
2. Run `npm test -- src/styles/app-css.test.ts` and confirm the new assertions fail.
3. Add shared surface tokens and the base shell rules without touching the overlay-specific selectors.
4. Re-run the focused test and commit.

### Task 2: Redesign Settings markup without changing behavior

**Files:**
- Modify: `src/components/settings.test.ts`
- Modify: `src/a11y.test.ts`
- Modify: `src/components/settings.ts`
- Modify: `src/styles/app.css`

1. Add failing tests for the rail, autosave status, section headers, grouped control rows, runtime health block, and accessible history destination.
2. Run `npm test -- src/components/settings.test.ts src/a11y.test.ts` and confirm failure.
3. Restructure the existing HTML template and generated custom-select/account fragments around the new classes while preserving names, data attributes, ARIA, and event binding.
4. Implement responsive, focus, hover, selected, disabled, dialog, theme-preview, and scrollbar styles.
5. Run the focused tests and commit.

### Task 3: Add progressive Lenis/GSAP enhancement

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Create: `src/motion/surface-motion.ts`
- Create: `src/motion/surface-motion.test.ts`
- Modify: `src/main.ts`

1. Add failing tests around reduced-motion detection, safe no-op behavior, cleanup, and settings-page transition callbacks using mocked animation dependencies.
2. Install `lenis` and `gsap`.
3. Implement the enhancement with a scoped Lenis instance, GSAP ticker bridge, scoped contexts, and idempotent cleanup.
4. Wire cleanup into persistent Settings window repainting.
5. Run focused tests and the Settings tests, then commit.

### Task 4: Recompose the History workspace

**Files:**
- Modify: `src/history/HistoryApp.test.tsx`
- Modify: `src/history/history-accessibility.test.tsx`
- Modify: `src/history/HistoryApp.tsx`
- Modify: `src/styles/app.css`

1. Add failing tests for the custom header, selected range summary, filter toolbar, metric cards, stable loading regions, and named data panels.
2. Confirm failure with `npm test -- src/history/HistoryApp.test.tsx src/history/history-accessibility.test.tsx`.
3. Recompose the React markup using semantic regions and robust class names while preserving query behavior.
4. Add the history scroll container and responsive layout styles.
5. Re-run the focused tests and commit.

### Task 5: Rebuild the chart and data panels

**Files:**
- Modify: `src/history/HistoryChart.test.tsx`
- Modify: `src/history/BillingTable.test.tsx`
- Modify: `src/history/ExportControls.test.tsx`
- Modify: `src/history/HistoryChart.tsx`
- Modify: `src/history/BillingTable.tsx`
- Modify: `src/history/ExportControls.tsx`
- Modify: `src/styles/app.css`

1. Add failing tests for chart grid/legend/series paths, data-panel wrappers, formatted labels, designed empty states, and modal structure.
2. Confirm failure with the focused Vitest files.
3. Implement the SVG chart presentation and retain its keyboard-navigation buttons and text description.
4. Implement custom billing/model tables, utility panel, and confirmation modal.
5. Run the focused and accessibility suites, then commit.

### Task 6: Animate History with cleanup and reduced-motion support

**Files:**
- Create: `src/motion/use-surface-motion.ts`
- Create: `src/motion/use-surface-motion.test.tsx`
- Modify: `src/history/HistoryApp.tsx`
- Modify: `src/history/HistoryChart.tsx`

1. Add failing tests for mounting, dependency-driven content reveal, cleanup, and reduced-motion behavior.
2. Implement the React hook using scoped refs and the shared animation functions.
3. Add chart line draw and data-refresh choreography without hiding content from assistive technology.
4. Run the History suite and commit.

### Task 7: Full verification and installed-app inspection

**Files:**
- Modify only if a verified defect is found.

1. Run `npm test`.
2. Run `npm run build`.
3. Run `cargo test --manifest-path src-tauri/Cargo.toml`.
4. Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
5. Build the Windows bundle with `npm run tauri build`.
6. Install the resulting NSIS package, launch the app, and visually inspect Settings and History at default/minimum sizes, including keyboard focus, scroll behavior, modal focus, and reduced motion.
7. Fix any verified defects test-first and repeat the affected checks.
8. Commit final verification fixes and record results in the handoff.
