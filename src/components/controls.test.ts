import { describe, expect, it, vi } from "vitest";
import { renderControls } from "./controls";

describe("renderControls", () => {
  it("renders a provider-specific minimize button", () => {
    const el = renderControls("claude", vi.fn());
    expect(el.querySelectorAll("button")).toHaveLength(1);
    const button = el.querySelector("button");
    expect(button?.getAttribute("aria-label")).toBe("Minimize Claude usage");
    expect(button?.getAttribute("title")).toBe("Minimize Claude usage");
    expect(button?.querySelector("svg")).not.toBeNull();
    expect(button?.textContent).not.toContain("›");
  });
  it("emits the action when a control is activated", () => {
    const onAction = vi.fn();
    const el = renderControls("openai", onAction);
    el.querySelector<HTMLButtonElement>("button")!.click();
    expect(onAction).toHaveBeenCalledWith({ action: "minimize", provider: "openai" });
  });
  it("activates from Enter and Space without changing the provider key", () => {
    const onAction = vi.fn();
    const button = renderControls("claude", onAction).querySelector<HTMLButtonElement>("button")!;
    button.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    button.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true }));
    expect(onAction).toHaveBeenCalledTimes(2);
    expect(onAction).toHaveBeenLastCalledWith({ action: "minimize", provider: "claude" });
  });
});
