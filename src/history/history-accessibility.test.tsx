import { axe } from "vitest-axe";
import { render } from "@testing-library/react";
import { expect, it } from "vitest";
import { HistoryChart } from "./HistoryChart";
import { BillingTable } from "./BillingTable";
import { ExportControls } from "./ExportControls";
import { HistoryApp } from "./HistoryApp";
import { invoke } from "@tauri-apps/api/core";
import { vi } from "vitest";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const point = { provider: "claude", windowKind: "session_5h", sampledAt: 1, usedPercent: 20, model: null, apiCalls: null, estimatedCostMicros: null, overageCostMicros: null };
const aggregate = { provider: "claude", currency: "USD", source: "estimated", amountMicros: 123456 };
it("has no violations across history chart billing and export surfaces", async () => {
  const { container } = render(<><HistoryChart points={[point]} /><BillingTable entries={[]} /><ExportControls /></>);
  // jsdom does not implement getComputedStyle for pseudo-elements, which makes
  // axe's color-contrast rule emit noisy stderr. Contrast is covered manually
  // in the desktop webview; keep structural, ARIA, and keyboard rules active.
  expect((await axe(container, { rules: { "color-contrast": { enabled: false } } })).violations).toEqual([]);
});
it("has no violations across the full history app", async () => {
  vi.mocked(invoke).mockImplementation((name) => Promise.resolve(name === "query_history" ? { points: [], billing: [] } : [aggregate]) as never);
  const { container } = render(<HistoryApp now={() => 3_000_000} />);
  expect((await axe(container, { rules: { "color-contrast": { enabled: false } } })).violations).toEqual([]);
});
