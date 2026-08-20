/// <reference types="vite/client" />

import { expect, it } from "vitest";

interface NodeProcess {
  getBuiltinModule(name: "fs"): { readFileSync(path: string | URL, encoding: "utf8"): string };
}

const nodeProcess = (globalThis as typeof globalThis & { process: NodeProcess }).process;
const fs = nodeProcess.getBuiltinModule("fs");
const moduleFileUrl = new URL(import.meta.url);
const sourceUrl = (url: URL, relativePath: string): URL => url.protocol === "file:" ? url : new URL(relativePath, moduleFileUrl);
const chart = fs.readFileSync(sourceUrl(new URL("./HistoryChart.tsx", import.meta.url), "./HistoryChart.tsx"), "utf8");

it("keeps chart source compatible with the configured ES2020 target", () => {
  expect(chart).not.toContain(".replaceAll(");
  expect(chart).not.toMatch(/\.at\s*\(/);
});
