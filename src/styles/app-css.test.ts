/// <reference types="vite/client" />

import { describe, expect, it } from "vitest";

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

  it("lays provider bubbles out in a stable horizontal top-corner row at 48px", () => {
    const row = ruleFor(".provider-bubble-row");
    expect(row).toContain("display: flex;");
    expect(row).toContain("flex-direction: row;");
    expect(row).toContain("position: absolute;");
    expect(row).toContain("top: 0;");

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

  it("pins bubbles to the selected horizontal corner and avoids a right-corner collision", () => {
    expect(ruleFor('#app[data-corner="top-left"] .provider-bubble-row')).toContain("left: 0;");
    expect(ruleFor('#app[data-corner="bottom-left"] .provider-bubble-row')).toContain("left: 0;");
    expect(ruleFor('#app[data-corner="top-right"] .provider-bubble-row')).toContain("right: 0;");
    expect(ruleFor('#app[data-corner="bottom-right"] .provider-bubble-row')).toContain("right: 0;");
    expect(css).toContain('[data-bubble-count="1"][data-corner="top-right"] .layer .minimize-control');
    expect(css).toContain("right: 56px;");
    expect(css).toContain('#app[data-layout="provider-columns"][data-expanded-count="1"] .layers');
  });

  it("uses no host padding when collapsed and reserves only the row plus gap when mixed", () => {
    const collapsed = ruleFor('#app[data-expanded-count="0"][data-bubble-count="1"]');
    expect(collapsed).toContain('#app[data-expanded-count="0"][data-bubble-count="2"]');
    expect(collapsed).toContain("padding: 0;");

    expect(ruleFor('#app[data-expanded-count="1"][data-bubble-count="1"]'))
      .toContain("padding: 0 8px 8px;");
    expect(ruleFor('#app[data-expanded-count="1"][data-bubble-count="1"] .layers'))
      .toContain("padding-top: 57px;");
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

  it("removes the shared pill and keeps the app/window surface transparent", () => {
    expect(css).not.toContain(".minimized-pill");
    expect(ruleFor("body { font:")).toContain("background: transparent;");
    const appRule = ruleFor("#app");
    expect(appRule).toContain("background: transparent;");
    expect(appRule).not.toContain("backdrop-filter");
  });
});
