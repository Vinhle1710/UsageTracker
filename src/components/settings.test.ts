import { describe, expect, it, vi } from "vitest";
import { renderSettings } from "./settings";
import type { Config, MonitorOption } from "../types";

const config: Config = {
  monitorId: "display-2",
  corner: "bottom-right",
  scale: 1,
  cardOpacity: 0.96,
  layout: "stacked-compact",
  alwaysOnTop: true,
  offscreenPeek: false,
  pollIntervalSec: 60,
  detectIntervalSec: 5,
};

const monitors: MonitorOption[] = [
  { id: "display-1", label: "Monitor 1 — 1920×1080" },
  { id: "display-2", label: "Monitor 2 — 2560×1440" },
];

describe("renderSettings", () => {
  it("uses a friendly monitor dropdown instead of an ID input", () => {
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() });
    expect(el.querySelector("input[name=monitorId]")).toBeNull();
    expect(el.querySelectorAll("select[name=monitorId] option")).toHaveLength(2);
    expect(el.textContent).toContain("Monitor 2 — 2560×1440");
    expect(el.textContent).not.toContain("display-2");
    expect(el.textContent).toContain("Monitor");
  });

  it("saves scale changes immediately", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    const scale = el.querySelector<HTMLInputElement>("input[name=scale]")!;
    scale.value = "125";
    scale.dispatchEvent(new Event("input", { bubbles: true }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ scale: 1.25 }));
  });

  it("saves layout changes immediately and has no save button", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    const layout = el.querySelector<HTMLSelectElement>("select[name=layout]")!;
    layout.value = "provider-columns";
    layout.dispatchEvent(new Event("change", { bubbles: true }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ layout: "provider-columns" }));
    expect(el.querySelector("button[data-save]")).toBeNull();
    expect(el.textContent).toContain("Horizontal");
    expect(el.textContent).toContain("Vertical");
  });

  it("does not expose panel size and saves opacity changes immediately", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    expect(el.querySelector("select[name=sizeState]")).toBeNull();

    const opacity = el.querySelector<HTMLInputElement>("input[name=cardOpacity]")!;
    opacity.value = "88";
    opacity.dispatchEvent(new Event("input", { bubbles: true }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ cardOpacity: 0.88 }));
    expect(el.textContent).not.toContain("Panel size");
    expect(el.textContent).toContain("Card opacity");
  });

  it("calls the close action from the settings button", () => {
    const onClose = vi.fn();
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose });
    el.querySelector<HTMLButtonElement>("[data-close]")!.click();
    expect(onClose).toHaveBeenCalledOnce();
  });
});
