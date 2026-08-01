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
