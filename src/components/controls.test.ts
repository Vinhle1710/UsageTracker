import { describe, expect, it, vi } from "vitest";
import { renderControls } from "./controls";

describe("renderControls", () => {
  it("renders only the quiet minimize button", () => {
    const el = renderControls(vi.fn());
    expect(el.querySelectorAll("button")).toHaveLength(1);
    expect(el.querySelector("button")?.getAttribute("aria-label")).toBe("Minimize overlay to screen edge");
  });
  it("emits the action when a control is activated", () => {
    const onAction = vi.fn();
    const el = renderControls(onAction);
    el.querySelector<HTMLButtonElement>("button")!.click();
    expect(onAction).toHaveBeenCalledWith("minimize");
  });
});
