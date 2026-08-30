import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ExportControls } from "./ExportControls";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

describe("ExportControls", () => {
  beforeEach(() => vi.mocked(invoke).mockReset());
  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("renders the export panel with plain-language headings", () => {
    const { container } = render(<ExportControls />);
    expect(container.querySelector(".history-utility")).not.toBeNull();
    expect(screen.getByRole("heading", { name: "Export" })).toBeTruthy();
    expect(container.textContent).not.toMatch(/archive|ledger/i);
  });

  it("does not steal focus or scroll the archive on initial render", () => {
    render(<ExportControls />);
    expect(document.activeElement).not.toBe(
      screen.getByRole("button", { name: "Clear history" }),
    );
  });

  it.each(["json", "csv"] as const)(
    "uses the opaque export authorization for %s",
    async (format) => {
      vi.mocked(invoke)
        .mockResolvedValueOnce("export-handle")
        .mockResolvedValue(undefined);
      render(<ExportControls />);
      fireEvent.change(screen.getByLabelText("Format"), {
        target: { value: format },
      });
      fireEvent.click(screen.getByRole("button", { name: "Export history" }));
      await waitFor(() =>
        expect(invoke).toHaveBeenCalledWith(
          "export_history",
          expect.objectContaining({ exportHandle: "export-handle" }),
        ),
      );
      expect(invoke).not.toHaveBeenCalledWith(
        "export_history",
        expect.objectContaining({ destination: expect.anything() }),
      );
    },
  );

  it("supports cancel", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(null);
    render(<ExportControls />);
    fireEvent.click(screen.getByRole("button", { name: "Export history" }));
    await waitFor(() =>
      expect(screen.getByRole("status").textContent).toMatch(/cancelled/),
    );
  });

  it("focuses and confirms clear", async () => {
    const onCleared = vi.fn();
    vi.mocked(invoke).mockResolvedValue(undefined);
    render(<ExportControls onCleared={onCleared} />);
    const clear = screen.getByRole("button", { name: "Clear history" });
    fireEvent.click(clear);
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Confirm clear" }),
    );
    expect(
      screen.getByRole("dialog").classList.contains("history-modal"),
    ).toBe(true);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(clear);
    fireEvent.click(clear);
    fireEvent.click(screen.getByRole("button", { name: "Confirm clear" }));
    await waitFor(() => expect(onCleared).toHaveBeenCalled());
  });
});
