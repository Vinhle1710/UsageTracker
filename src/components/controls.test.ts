import { describe, expect, it, vi } from "vitest";
import { renderControls } from "./controls";

describe("renderControls", () => {
  it("renders only the quiet minimize button", () => {
    const el = renderControls(vi.fn());
    expect(el.querySelectorAll("button")).toHaveLength(1);
    const button = el.querySelector("button");
    expect(button?.getAttribute("aria-label")).toBe("Minimize overlay to screen edge");
    expect(button?.getAttribute("title")).toBe("Minimize to screen edge");
    expect(button?.querySelector("svg")).not.toBeNull();
    expect(button?.textContent).not.toContain("›");
  });
  it("emits the action when a control is activated", () => {
    const onAction = vi.fn();
    const el = renderControls(onAction);
    el.querySelector<HTMLButtonElement>("button")!.click();
    expect(onAction).toHaveBeenCalledWith("minimize");
  });
});
