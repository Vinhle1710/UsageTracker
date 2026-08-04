import { describe, expect, it, vi } from "vitest";
import { renderSettings } from "./settings";
import type { Config, MonitorOption } from "../types";

const config: Config = {
  monitorId: "display-2",
  corner: "bottom-right",
  scale: 1,
  cardOpacity: 0.96,
  theme: "frosted",
  backgroundColor: "#07101f",
  layout: "stacked-compact",
  alwaysOnTop: true,
  offscreenPeek: false,
  launchAtStartup: true,
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
    expect(el.querySelector("select[name=monitorId]")).toBeNull();
    expect(el.querySelectorAll('[data-select="monitorId"] [role="option"]')).toHaveLength(2);
    expect(el.querySelector('[data-select="monitorId"] [role="combobox"]')).not.toBeNull();
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
    el.querySelector<HTMLButtonElement>('[data-select="layout"] [role="combobox"]')!.click();
    el.querySelector<HTMLElement>('[data-select="layout"] [role="option"][data-value="provider-columns"]')!.click();
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ layout: "provider-columns" }));
    expect(el.querySelector("button[data-save]")).toBeNull();
    expect(el.textContent).toContain("Horizontal");
    expect(el.textContent).toContain("Vertical");
  });

  it("saves opacity changes immediately", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });

    const opacity = el.querySelector<HTMLInputElement>("input[name=cardOpacity]")!;
    opacity.value = "88";
    opacity.dispatchEvent(new Event("input", { bubbles: true }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ cardOpacity: 0.88 }));
    expect(el.textContent).toContain("Card opacity");
  });

  it("toggles launch at startup immediately", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    el.querySelector<HTMLButtonElement>('[data-page="behavior"]')!.click();
    const launchAtStartup = el.querySelector<HTMLInputElement>("input[name=launchAtStartup]")!;
    expect(launchAtStartup.checked).toBe(true);
    launchAtStartup.checked = false;
    launchAtStartup.dispatchEvent(new Event("change", { bubbles: true }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ launchAtStartup: false }));
  });

  it("calls the close action from the settings button", () => {
    const onClose = vi.fn();
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose });
    el.querySelector<HTMLButtonElement>("[data-close]")!.click();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("provides accessible pages, concise previews, and future custom-theme actions", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    expect(el.querySelectorAll('[role="tab"]')).toHaveLength(5);
    expect(Array.from(el.querySelectorAll<HTMLElement>("[data-panel]")).filter((panel) => panel.getAttribute("aria-hidden") === "false").map((panel) => panel.dataset.panel)).toEqual(["general"]);
    expect(el.textContent).not.toContain("Changes save instantly");
    el.querySelector<HTMLButtonElement>('[data-page="theme"]')!.click();
    expect(el.querySelector<HTMLButtonElement>('[data-page="theme"]')!.getAttribute("aria-selected")).toBe("true");
    expect(el.querySelector<HTMLElement>('[data-panel="general"]')!.getAttribute("aria-hidden")).toBe("true");
    expect(el.querySelector<HTMLElement>('[data-panel="theme"]')!.getAttribute("aria-hidden")).toBe("false");
    expect(el.querySelector(".theme-grid--single-column")).not.toBeNull();
    expect(el.querySelectorAll("[data-preview-theme]")).toHaveLength(4);
    expect(el.querySelectorAll(".theme-option small")).toHaveLength(0);
    expect(el.querySelector('[data-theme="frosted"]')).not.toBeNull();
    expect(el.querySelector('[data-theme="acrylic"]')).toBeNull();
    expect(el.querySelector('[data-theme="blur"]')).not.toBeNull();
    expect(el.querySelector('[data-theme="opaque"]')).toBeNull();
    el.querySelector<HTMLButtonElement>('[data-theme="solid"]')!.click();
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ theme: "solid", cardOpacity: config.cardOpacity }));
    const color = el.querySelector<HTMLInputElement>("input[name=backgroundColor]")!;
    color.value = "#203040";
    color.dispatchEvent(new Event("input", { bubbles: true }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ theme: "solid", backgroundColor: "#203040" }));
    el.querySelector<HTMLButtonElement>('[data-theme="frosted"]')!.click();
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ theme: "frosted", cardOpacity: config.cardOpacity }));
    expect(el.querySelectorAll("[data-custom-theme-action][disabled]")).toHaveLength(3);
  });

  it("keeps the header draggable without a visible drag control", () => {
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn(), onDrag: vi.fn() });
    const header = el.querySelector<HTMLElement>("[data-drag-handle]")!;
    expect(header.hasAttribute("data-tauri-drag-region")).toBe(true);
    expect(el.querySelector("[data-drag-grip]")).toBeNull();
  });

  it("has no saved or theme-updated status line", () => {
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() });
    expect(el.querySelector("[data-feedback]")).toBeNull();
    expect(el.textContent).not.toContain("Saved");
    expect(el.textContent).not.toContain("Theme updated");
  });

  it("does not change theme when opacity changes", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    const opacity = el.querySelector<HTMLInputElement>("input[name=cardOpacity]")!;
    opacity.value = "84";
    opacity.dispatchEvent(new Event("input", { bubbles: true }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ theme: "frosted", cardOpacity: 0.84 }));
  });

  describe("Claude account panel", () => {
    function openAccountPanel(el: HTMLElement) {
      el.querySelector<HTMLButtonElement>('[data-page="account"]')!.click();
    }

    it("shows a sign-in button when signed out, and starts the flow on click", () => {
      const onClaudeSignIn = vi.fn();
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn(), onClaudeSignIn }, null);
      openAccountPanel(el);
      expect(el.textContent).toContain("Not signed in");
      el.querySelector<HTMLButtonElement>(".claude-account__action")!.click();
      expect(onClaudeSignIn).toHaveBeenCalledOnce();
      expect(el.textContent).toContain("Paste the code");
    });

    it("shows the account and a log-out button when already signed in", () => {
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, { organizationUuid: "org-12345678-abcd" });
      openAccountPanel(el);
      expect(el.textContent).toContain("Signed in");
      expect(el.textContent).toContain("org-1234");
      expect(el.querySelector<HTMLButtonElement>(".claude-account__action")!.textContent).toBe("Log out");
    });

    it("logs out and returns to the signed-out view", async () => {
      const onClaudeLogout = vi.fn().mockResolvedValue(undefined);
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn(), onClaudeLogout }, { organizationUuid: "org-1" });
      openAccountPanel(el);
      el.querySelector<HTMLButtonElement>(".claude-account__action")!.click();
      await Promise.resolve();
      await Promise.resolve();
      expect(onClaudeLogout).toHaveBeenCalledOnce();
      expect(el.textContent).toContain("Not signed in");
    });

    it("submits the pasted code and shows the signed-in account on success", async () => {
      const onClaudeSignInSubmit = vi.fn().mockResolvedValue({ ok: true, account: { organizationUuid: "org-99999999" } });
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn(), onClaudeSignIn: vi.fn(), onClaudeSignInSubmit }, null);
      openAccountPanel(el);
      el.querySelector<HTMLButtonElement>(".claude-account__action")!.click();
      const input = el.querySelector<HTMLInputElement>(".claude-account__code-input")!;
      input.value = "abc123#state456";
      el.querySelector<HTMLButtonElement>(".claude-account__action")!.click();
      await Promise.resolve();
      await Promise.resolve();
      expect(onClaudeSignInSubmit).toHaveBeenCalledWith("abc123#state456");
      expect(el.textContent).toContain("org-9999");
      expect(el.querySelector(".claude-account__code-input")).toBeNull();
    });

    it("shows an inline error and stays on the paste step when the code is rejected", async () => {
      const onClaudeSignInSubmit = vi.fn().mockResolvedValue({ ok: false, error: "That code has expired." });
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn(), onClaudeSignIn: vi.fn(), onClaudeSignInSubmit }, null);
      openAccountPanel(el);
      el.querySelector<HTMLButtonElement>(".claude-account__action")!.click();
      el.querySelector<HTMLButtonElement>(".claude-account__action")!.click();
      await Promise.resolve();
      await Promise.resolve();
      expect(el.textContent).toContain("That code has expired.");
      expect(el.querySelector(".claude-account__code-input")).not.toBeNull();
    });
  });
});
