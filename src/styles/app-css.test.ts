/// <reference types="vite/client" />

import { describe, expect, it } from "vitest";
import { OVERLAY_HEADROOM } from "../geometry";

interface NodeProcess {
  getBuiltinModule(name: "fs"): { readFileSync(path: string | URL, encoding: "utf8"): string };
}

const nodeProcess = (globalThis as typeof globalThis & { process: NodeProcess }).process;
const fs = nodeProcess.getBuiltinModule("fs");
const moduleFileUrl = new URL(import.meta.url);
const sourceUrl = (url: URL, relativePath: string): URL => url.protocol === "file:" ? url : new URL(relativePath, moduleFileUrl);
const css = fs.readFileSync(sourceUrl(new URL("./app.css", import.meta.url), "./app.css"), "utf8");
const main = fs.readFileSync(sourceUrl(new URL("../main.ts", import.meta.url), "../main.ts"), "utf8");

function ruleFor(selector: string): string {
  const start = css.indexOf(selector);
  expect(start).toBeGreaterThanOrEqual(0);
  const openingBrace = css.indexOf("{", start);
  expect(openingBrace).toBeGreaterThan(start);
  const end = css.indexOf("}", openingBrace);
  expect(end).toBeGreaterThan(start);
  return css.slice(start, end + 1);
}

function blockFor(marker: string): string {
  const start = css.indexOf(marker);
  expect(start).toBeGreaterThanOrEqual(0);
  const openingBrace = css.indexOf("{", start);
  expect(openingBrace).toBeGreaterThan(start);
  let depth = 0;
  for (let index = openingBrace; index < css.length; index += 1) {
    if (css[index] === "{") depth += 1;
    if (css[index] === "}") depth -= 1;
    if (depth === 0) return css.slice(start, index + 1);
  }
  throw new Error(`Unclosed CSS block: ${marker}`);
}

describe("provider card material CSS", () => {
  it("defines a full-card Frosted material", () => {
    const rule = ruleFor('#app[data-theme="frosted"] .layer');
    expect(rule).toContain("background: color-mix(in srgb, var(--card-background) var(--frosted-opacity), transparent);");
    expect(rule).toContain("-webkit-backdrop-filter: blur(18px) saturate(145%);");
    expect(rule).toContain("backdrop-filter: blur(18px) saturate(145%);");
  });

  it("defines a full-card Blur material", () => {
    const rule = ruleFor('#app[data-theme="blur"] .layer');
    expect(rule).toContain("background: color-mix(in srgb, var(--card-background) var(--blur-opacity), transparent);");
    expect(rule).toContain("-webkit-backdrop-filter: blur(12px);");
    expect(rule).toContain("backdrop-filter: blur(12px);");
  });

  it("has no Acrylic selectors and no gradients in the full-card glass rules", () => {
    expect(css).not.toContain('data-theme="acrylic"');
    expect(ruleFor('#app[data-theme="frosted"] .layer')).not.toContain("linear-gradient");
    expect(ruleFor('#app[data-theme="blur"] .layer')).not.toContain("linear-gradient");
  });

  it("keeps the Frosted and Blur opacity mappings in applyAppearance", () => {
    expect(main).toContain('app.style.setProperty("--frosted-opacity", `${Math.round(config.cardOpacity * 72)}%`);');
    expect(main).toContain('app.style.setProperty("--blur-opacity", `${Math.round(config.cardOpacity * 58)}%`);');
  });
});

describe("overlay headroom", () => {
  it("keeps the CSS slack in lockstep with OVERLAY_HEADROOM", () => {
    // The native card regions are placed at this same inset from the window edge. If the CSS and
    // the geometry constant disagree, the cards and the window region that clips them drift
    // apart, which shows up as cards being cropped along one edge.
    expect(ruleFor("#app {")).toContain(`--overlay-headroom: ${OVERLAY_HEADROOM}px;`);
  });

  it("drops the slack for the settings window, where nothing animates outside the panel", () => {
    expect(ruleFor('#app[data-window="settings"]')).toContain("--overlay-headroom: 0px;");
  });
});

describe("settings panel clip contract", () => {
  // The settings window is clipped to the panel's rect so Windows 11's frame border — which
  // traces the window rect — falls outside the region and never renders. settings_region in
  // material.rs rebuilds that rect from SETTINGS_PANEL_PADDING/SETTINGS_PANEL_RADIUS, so if these
  // CSS values move without the Rust ones, the clip cuts into the panel or lets the border back.
  it("insets the panel by the padding the native region is derived from", () => {
    expect(ruleFor('#app[data-window="settings"]')).toContain("padding: 10px;");
  });

  it("rounds the panel to the radius the native region is derived from", () => {
    expect(ruleFor(".settings-window {")).toContain("border-radius: 16px;");
  });
});

describe("settings and history atelier surfaces", () => {
  it("gives history its own full-window scroll shell and sticky command header", () => {
    expect(ruleFor('#app[data-window="history"]')).toContain("overflow: hidden;");
    expect(ruleFor(".history-scroll {")).toContain("overflow-y: auto;");
    expect(ruleFor(".history-header {")).toContain("position: sticky;");
  });

  it("uses a distinct data face and strong visible focus across both surfaces", () => {
    expect(ruleFor(".telemetry-value")).toContain('font-family: "Cascadia Code"');
    expect(ruleFor(".surface-control:focus-visible")).toContain("outline: 2px solid var(--signal);");
  });

  it("keeps screen-reader chart summaries visually hidden", () => {
    expect(ruleFor(".sr-only")).toContain("clip-path: inset(50%);");
    expect(ruleFor(".sr-only")).toContain("position: absolute;");
  });

  it("keeps navigation and content motion decorative under reduced motion", () => {
    const reduced = blockFor("@media (prefers-reduced-motion: reduce)");
    expect(reduced).toContain(".surface-motion-item");
    expect(reduced).toContain("transform: none !important;");
  });

  it("lays out History as an editorial analytics workspace", () => {
    expect(ruleFor(".history-shell {")).toContain("background:");
    expect(ruleFor(".history-commandbar {")).toContain("grid-template-columns:");
    expect(ruleFor(".history-metric-grid {")).toContain("grid-template-columns: repeat(5");
    expect(ruleFor(".history-data-grid {")).toContain("grid-template-columns:");
  });

  it("styles data visualization and destructive confirmation as first-class surfaces", () => {
    expect(ruleFor(".history-chart__line")).toContain("vector-effect: non-scaling-stroke;");
    expect(ruleFor(".history-chart__plot > button")).toContain("position: absolute;");
    expect(ruleFor(".history-modal-backdrop")).toContain("position: fixed;");
    expect(ruleFor(".history-utility {")).toContain("grid-template-columns:");
  });
});

describe("provider bubble interaction CSS", () => {
  it("gives each compact minimize glyph a 44px interactive target without covering card content", () => {
    const target = ruleFor(".minimize-control__button");
    expect(target).toContain("width: 44px;");
    expect(target).toContain("height: 44px;");
    expect(target).toContain("background: transparent;");
    expect(target).toContain("border: 0;");

    const visual = ruleFor(".minimize-control__button::before");
    expect(visual).toContain("width: 27px;");
    expect(visual).toContain("height: 27px;");
    expect(ruleFor(".layer__title")).toContain("padding-right: 40px;");
    expect(ruleFor(".layer__title")).toContain("min-height: 27px;");
  });

  it("lays provider bubbles out in a stable horizontal row at 48px", () => {
    const row = ruleFor(".provider-bubble-row");
    expect(row).toContain("display: flex;");
    expect(row).toContain("flex-direction: row;");
    // In flow, never pinned to a window edge: an edge-pinned row sits at a distance from the
    // anchored corner that depends on window height, so it moved on every resize.
    expect(row).toContain("position: relative;");
    expect(row).not.toContain("position: absolute;");

    const bubble = ruleFor(".provider-bubble {");
    expect(bubble).toContain("width: 48px;");
    expect(bubble).toContain("height: 48px;");
    expect(ruleFor(".provider-bubble__logo")).toContain("width: 26px;");
    expect(ruleFor(".provider-bubble__logo")).toContain("height: 26px;");
  });

  it("uses provider color and live card opacity variables in every bubble theme", () => {
    expect(ruleFor('.provider-bubble[data-provider="claude"]')).toContain("--provider-accent: var(--claude);");
    expect(ruleFor('.provider-bubble[data-provider="openai"]')).toContain("--provider-accent: var(--chatgpt);");

    const clear = ruleFor('#app[data-theme="clear"] .provider-bubble');
    expect(clear).toContain("linear-gradient");
    expect(clear).toContain("var(--provider-accent)");
    expect(clear).toContain("var(--card-background) var(--card-opacity)");

    const frosted = ruleFor('#app[data-theme="frosted"] .provider-bubble');
    expect(frosted).toContain("var(--card-background) var(--frosted-opacity)");
    expect(frosted).toContain("backdrop-filter: blur(18px) saturate(145%);");

    const blur = ruleFor('#app[data-theme="blur"] .provider-bubble');
    expect(blur).toContain("var(--card-background) var(--blur-opacity)");
    expect(blur).toContain("backdrop-filter: blur(12px);");

    expect(ruleFor('#app[data-theme="solid"] .provider-bubble'))
      .toContain("var(--card-background) var(--card-opacity)");
  });

  it("keeps geometry-measured bubble boxes stable while animating only their contents", () => {
    const bubble = ruleFor(".provider-bubble {");
    expect(bubble).not.toContain("transition: transform");
    expect(css).not.toMatch(/\.provider-bubble-row\s*\{[^}]*\btransform\s*:/);
    expect(css).not.toMatch(/\.provider-bubble(?:\s*|:(?:hover|active|focus-visible))\s*\{[^}]*\btransform\s*:/);
    expect(css).not.toContain(".provider-bubble-row { animation: bubbly-in");
    expect(ruleFor(".provider-bubble:hover .provider-bubble__logo"))
      .toContain("transform: translateY(-1px) scale(1.04);");
    expect(ruleFor(".provider-bubble:active .provider-bubble__logo"))
      .toContain("transform: scale(.9);");
  });

  it("keeps measured card geometry stable through restore entry and usage updates", () => {
    expect(ruleFor(".layers {")).not.toContain("transform");
    expect(ruleFor(".layer {")).not.toContain("transform");

    const entryMotion = blockFor("@keyframes card-fade-in");
    expect(entryMotion).toContain("opacity: 0;");
    expect(entryMotion).toContain("opacity: 1;");
    expect(entryMotion).not.toContain("transform");
    expect(blockFor("@media (prefers-reduced-motion: no-preference)"))
      .toContain(".layer { animation: card-fade-in 260ms cubic-bezier(.2,.8,.3,1) both; }");

    expect(ruleFor(".meter[data-usage-change] .meter__progress"))
      .toContain("animation: usage-change");
    expect(ruleFor(".meter--resetting {"))
      .toContain("animation: bubbly-reset");
    expect(css).not.toMatch(/\.layer(?:\[[^\]]+\])?\s*\{[^}]*animation:\s*(?:usage-change|bubbly-reset)/);
    expect(blockFor("@media (prefers-reduced-motion: reduce)"))
      .toMatch(/\.layer[^}]*animation: none; transition: none;/);
  });

  it("renders bubble focus inside the native region and preserves general control focus", () => {
    const focus = ruleFor(".provider-bubble:focus-visible");
    expect(focus).toContain("outline: none;");
    expect(focus).toContain("inset 0 0 0 3px var(--provider-accent)");
    expect(focus).not.toContain("outline-offset");
    expect(ruleFor('button:focus-visible, input:focus-visible, [role="option"]:focus-visible'))
      .toContain("outline: 2px solid var(--accent);");
  });

  it("disables nonessential bubble motion when reduced motion is requested", () => {
    expect(css).toContain("@media (prefers-reduced-motion: reduce)");
    expect(css).toMatch(/prefers-reduced-motion: reduce[\s\S]*\.provider-bubble__logo[\s\S]*transition: none;/);
    expect(css).toMatch(/prefers-reduced-motion: reduce[\s\S]*\.provider-bubble-row[\s\S]*animation: none;/);
  });

  it("pins bubbles to the selected horizontal corner", () => {
    expect(ruleFor(".provider-bubble-row")).toContain("justify-content: flex-end;");
    expect(ruleFor('#app[data-corner$="left"] .provider-bubble-row')).toContain("justify-content: flex-start;");
    expect(css).toContain('#app[data-layout="provider-columns"][data-expanded-count="1"] .layers');
  });

  it("puts bubbles on the side away from the anchored corner and packs the stack toward it", () => {
    // Cards stay pinned against the anchored corner and the window grows away from it, so the
    // card never moves on screen and every offset from that corner is independent of window
    // size — which is what makes a mid-animation native resize invisible instead of a jump.
    // Bottom-anchored keeps bubbles above the card; top-anchored puts them below.
    expect(ruleFor(".provider-bubble-row")).toContain("order: -1;");
    expect(ruleFor('#app[data-corner^="top"] .provider-bubble-row')).toContain("order: 1;");

    // #app packs a content-sized stack against the anchor, both axes. Packing a full-height
    // .layers instead left a fractional sliver that drifted the DOM off its native region.
    expect(ruleFor('#app[data-corner^="bottom"] {')).toContain("align-content: end;");
    expect(ruleFor('#app[data-corner^="top"] {')).toContain("align-content: start;");
    expect(ruleFor('#app[data-corner$="right"] {')).toContain("justify-items: end;");
    expect(ruleFor('#app[data-corner$="left"] {')).toContain("justify-items: start;");
    expect(ruleFor(".layers {")).not.toContain("height: 100%;");
  });

  it("never shifts the expanded card's minimize button for the bubble row", () => {
    // The row is a sibling grid item, so it can never overlap the card's own minimize-control
    // and needs no collision rule. A leftover "right: 56px" one shifted the button anyway, for
    // no reason a user could see — it just looked broken.
    expect(css).not.toContain('[data-bubble-count="1"][data-corner="top-right"] .layer .minimize-control');
    expect(css).not.toContain('[data-bubble-count="1"][data-corner="bottom-right"] .layer .minimize-control');
    // The old padding reserve went with it: grid gap spaces the row from the card now, so there
    // is no hardcoded 57px that has to be kept in sync with the row's height.
    expect(css).not.toContain("padding-top: 57px;");
  });

  it("uses only invisible headroom when collapsed and reserves the row plus gap when mixed", () => {
    const collapsed = ruleFor('#app[data-expanded-count="0"][data-bubble-count="1"]');
    expect(collapsed).toContain('#app[data-expanded-count="0"][data-bubble-count="2"]');
    // No card padding at all — just the transparent slack the animation overshoots into.
    expect(collapsed).toContain("padding: var(--overlay-headroom);");

    // The row is flush against the edge away from the anchor, so that edge gets only headroom
    // and no card padding; the anchor side gets both.
    expect(ruleFor('#app[data-expanded-count="1"][data-bubble-count="1"]'))
      .toContain("padding: var(--overlay-headroom) calc(8px + var(--overlay-headroom)) calc(8px + var(--overlay-headroom));");
    expect(ruleFor('#app[data-corner^="top"][data-expanded-count="1"][data-bubble-count="1"]'))
      .toContain("padding: calc(8px + var(--overlay-headroom)) calc(8px + var(--overlay-headroom)) var(--overlay-headroom);");
  });

  it("keeps the native surface and geometry contract free of the legacy pill", () => {
    expect(main).toContain("expandedProviderCount");
    expect(main).toContain("bubbleCount");
    expect(main).toContain("contentWidth");
    expect(main).toContain("new GeometryRequestScheduler");
    expect(main).toContain("geometryScheduler.enqueue(geometryRequest())");
    expect(main).not.toContain("let lastGeometry");
    expect(main).not.toContain("minimized");
  });

  it("sizes expanded cards from the layout, not from the current window width", () => {
    // The window is sized from the measured card width. If the card width were in turn taken
    // from the window, restoring out of a 48px bubble would pin the card at 48px forever.
    const layers = ruleFor(".layers {");
    expect(layers).not.toContain("width: 100%;");
    expect(layers).toContain("width: var(--layers-width);");

    expect(ruleFor('#app[data-layout="stacked-compact"] .layers')).toContain("--layers-width: 310px;");
    expect(ruleFor('#app[data-layout="provider-columns"] .layers')).toContain("--layers-width: 604px;");
  });

  it("keeps the bubble row in flow in every layout state, so max-content sizes around it", () => {
    // An out-of-flow row is invisible to `width: max-content` — with no expanded card, .layers
    // had nothing else in flow, collapsed to zero width, and the whole overlay vanished. The row
    // is now in flow unconditionally, rather than only being switched back for that one state.
    expect(css).not.toContain("position: static;");
    expect(ruleFor(".provider-bubble-row")).toContain("position: relative;");
    expect(ruleFor('#app[data-expanded-count="0"] .layers')).toContain("--layers-width: max-content;");
  });

  it("keeps a card that has no usage yet from collapsing into a broken-looking strip", () => {
    expect(ruleFor(".layer__empty {")).toContain("min-height: 72px;");
    expect(ruleFor('.layer[data-state="stale"], .layer[data-state="pending"]')).toContain("opacity: .72;");
  });

  it("lets a bubble-only row shrink to its own width", () => {
    expect(ruleFor('#app[data-expanded-count="0"] .layers')).toContain("--layers-width: max-content;");
  });

  it("removes the shared pill and keeps the app/window surface transparent", () => {
    expect(css).not.toContain(".minimized-pill");
    expect(ruleFor("body { font:")).toContain("background: transparent;");
    const appRule = ruleFor("#app");
    expect(appRule).toContain("background: transparent;");
    expect(appRule).not.toContain("backdrop-filter");
  });
});
