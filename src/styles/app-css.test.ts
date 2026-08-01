/// <reference types="vite/client" />

import { describe, expect, it } from "vitest";

interface NodeProcess {
  getBuiltinModule(name: "fs"): { readFileSync(path: string, encoding: "utf8"): string };
}

const nodeProcess = (globalThis as typeof globalThis & { process: NodeProcess }).process;
const css = nodeProcess.getBuiltinModule("fs").readFileSync("src/styles/app.css", "utf8");

function ruleFor(selector: string): string {
  const start = css.indexOf(`${selector} {`);
  expect(start).toBeGreaterThanOrEqual(0);
  const end = css.indexOf("}", start);
  expect(end).toBeGreaterThan(start);
  return css.slice(start, end + 1);
}

describe("provider card material CSS", () => {
  it("defines a full-card Frosted material", () => {
    const rule = ruleFor('#app[data-theme="frosted"] .layer');
    expect(rule).toContain("-webkit-backdrop-filter: blur(18px) saturate(145%);");
    expect(rule).toContain("backdrop-filter: blur(18px) saturate(145%);");
  });

  it("defines a full-card Blur material", () => {
    const rule = ruleFor('#app[data-theme="blur"] .layer');
    expect(rule).toContain("-webkit-backdrop-filter: blur(12px);");
    expect(rule).toContain("backdrop-filter: blur(12px);");
  });

  it("has no Acrylic selectors and no gradients in the full-card glass rules", () => {
    expect(css).not.toContain('data-theme="acrylic"');
    expect(ruleFor('#app[data-theme="frosted"] .layer')).not.toContain("linear-gradient");
    expect(ruleFor('#app[data-theme="blur"] .layer')).not.toContain("linear-gradient");
  });
});
