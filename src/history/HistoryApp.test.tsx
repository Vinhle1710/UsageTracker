import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { HistoryApp } from "./HistoryApp";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const p = { provider: "claude", windowKind: "session_5h", sampledAt: 2_999_999, usedPercent: 25, model: null, apiCalls: null, estimatedCostMicros: null, overageCostMicros: null };
describe("HistoryApp", () => {
 beforeEach(() => vi.mocked(invoke).mockReset());
 it("renders summaries and honest unavailable model state", async () => { vi.mocked(invoke).mockResolvedValue({ points: [p], billing: [] }); render(<HistoryApp now={() => 3_000_000} />); await waitFor(() => expect(screen.getByText(/Session usage/)).toBeInTheDocument()); expect(screen.getByText(/Per-model data unavailable/)).toBeInTheDocument(); expect(screen.getByText(/API calls: Unavailable/)).toBeInTheDocument(); });
 it("reloads when range and filters change", async () => { vi.mocked(invoke).mockResolvedValue({ points: [], billing: [] }); render(<HistoryApp now={() => 3_000_000} />); await waitFor(() => expect(invoke).toHaveBeenCalled()); fireEvent.click(screen.getByRole("button", { name: "30 days" })); fireEvent.change(screen.getByLabelText(/Provider/), { target: { value: "claude" } }); await waitFor(() => expect(invoke).toHaveBeenCalledWith("query_history", { query: expect.objectContaining({ provider: "claude", from: 408000, to: 3000001 }) })); });
 it("suppresses stale responses and clears an old error after success", async () => { let resolveOld!: (v: unknown) => void; const old = new Promise(r => { resolveOld = r; }); vi.mocked(invoke).mockReturnValueOnce(old as never).mockResolvedValueOnce({ points: [p], billing: [] }); render(<HistoryApp now={() => 3_000_000} />); fireEvent.click(screen.getByRole("button", { name: "30 days" })); await waitFor(() => expect(screen.getByText(/Claude/)).toBeInTheDocument()); resolveOld({ points: [], billing: [] }); expect(screen.getByText(/Claude/)).toBeInTheDocument(); });
});
