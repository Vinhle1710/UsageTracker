import { describe, expect, it, vi } from "vitest";
import { renderControls } from "./controls";

describe("renderControls", () => {
  it("renders every control as a real button", () => {
    const el = renderControls({ sizeState: "compact", alwaysOnTop: true }, vi.fn());
    const buttons = el.querySelectorAll("button");
    expect(buttons.length).toBeGreaterThanOrEqual(3);
    buttons.forEach((b) => expect(b.getAttribute("aria-label")).toBeTruthy());
  });
  it("reports the always-on-top state to assistive tech", () => {
    const el = renderControls({ sizeState: "compact", alwaysOnTop: true }, vi.fn());
    expect(el.querySelector('[data-action="pin"]')!.getAttribute("aria-pressed")).toBe("true");
  });
  it("emits the action when a control is activated", () => {
    const onAction = vi.fn();
    const el = renderControls({ sizeState: "compact", alwaysOnTop: false }, onAction);
    el.querySelector<HTMLButtonElement>('[data-action="bubble"]')!.click();
    expect(onAction).toHaveBeenCalledWith("bubble");
  });
});
