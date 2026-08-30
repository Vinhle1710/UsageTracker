import { describe, expect, it, vi } from "vitest";
import { renderConsoleCosts, renderSettings, type ConsoleCostsView } from "./settings";
import type { Config, MonitorOption } from "../types";
import { axe } from "vitest-axe";

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
  it("renders the settings shell without decorative numbering", () => {
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() });
    expect(el.querySelector(".settings-rail")).not.toBeNull();
    expect(el.querySelector(".settings-save-state")).toBeNull();
    expect(Array.from(el.querySelectorAll<HTMLElement>('[role="tab"]')).map((tab) => tab.textContent))
      .toEqual(["General", "Display", "Behavior", "Account"]);
    expect(el.querySelector('[data-panel="general"] .settings-panel__intro h2')?.textContent).toBe("General");
    expect(el.querySelector(".settings-nav__index")).toBeNull();
    expect(el.querySelector(".settings-panel__number")).toBeNull();
    expect(el.querySelector(".settings-window__brand-mark")).toBeNull();
    expect(el.textContent).not.toMatch(/control room|workspace/i);
    expect(el.querySelectorAll('[data-panel="general"] .settings-control-card')).toHaveLength(2);
    expect(el.querySelector(".settings-pages")?.hasAttribute("data-smooth-scroll")).toBe(true);
  });

  it("separates behavior automation, shortcuts, and runtime health", () => {
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() });
    const behavior = el.querySelector('[data-panel="behavior"]')!;
    expect(behavior.querySelector('[aria-labelledby="automation-title"]')).not.toBeNull();
    expect(behavior.querySelector('[aria-labelledby="shortcuts-title"]')).not.toBeNull();
    expect(behavior.querySelector('[aria-labelledby="runtime-title"]')).not.toBeNull();
    expect(behavior.querySelectorAll(".runtime-health__item")).toHaveLength(3);
  });

  it("keeps History navigation outside the tablist and axe-clean", async () => {
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() });
    const history = el.querySelector("#settings-history")!;
    const layout = el.querySelector(".settings-layout")!;
    expect(history.closest('[role="tablist"]')).toBeNull();
    expect(history.closest(".settings-sidebar")).not.toBeNull();
    expect(layout.children).toHaveLength(2);
    expect(history.getAttribute("aria-label")).toBe("Open history");
    expect((await axe(el)).violations).toEqual([]);
  });
  it("keeps History navigation outside the tablist with an accessible name", () => {
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() });
    const history = el.querySelector("#settings-history")!;
    expect(history.getAttribute("aria-label")).toBe("Open history");
    expect(history.closest('[role="tablist"]')).toBeNull();
    expect(history.getAttribute("role")).toBeNull();
  });
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

  it("requires the exact paid-session acknowledgement before enabling auto-init", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    el.querySelector<HTMLButtonElement>('[data-page="behavior"]')!.click();
    const toggle = el.querySelector<HTMLInputElement>("input[name=autoInitializeSession]")!;
    toggle.checked = true; toggle.dispatchEvent(new Event("change", { bubbles: true }));
    expect(el.querySelector('[role="dialog"]')).not.toBeNull();
    expect(el.textContent).toContain("I understand this can start a paid API/CLI session");
    expect(onChange).not.toHaveBeenCalledWith(expect.objectContaining({ autoInitializeSession: true }));
    const ack = el.querySelector<HTMLInputElement>("[data-cost-ack]")!;
    ack.checked = true; ack.dispatchEvent(new Event("change", { bubbles: true }));
    el.querySelector<HTMLButtonElement>("[data-confirm]")!.click();
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ autoInitializeSession: true, autoInitCostWarningAccepted: true }));
  });

  it("rejects duplicate shortcuts accessibly before saving", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    el.querySelector<HTMLButtonElement>('[data-page="behavior"]')!.click();
    const popover = el.querySelector<HTMLInputElement>("input[name=shortcutPopover]")!;
    const refresh = el.querySelector<HTMLInputElement>("input[name=shortcutRefresh]")!;
    popover.value = "Ctrl+Shift+U"; popover.dispatchEvent(new Event("change", { bubbles: true }));
    refresh.value = "ctrl+shift+u"; refresh.dispatchEvent(new Event("change", { bubbles: true }));
    expect(el.querySelector<HTMLElement>("[data-shortcut-error]")!.hidden).toBe(false);
    expect(el.querySelector("[data-shortcut-error]")!.textContent).toContain("Shortcut conflict");
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
    expect(el.querySelectorAll('[role="tab"]')).toHaveLength(4);
    expect(Array.from(el.querySelectorAll<HTMLElement>("[data-panel]")).filter((panel) => panel.getAttribute("aria-hidden") === "false").map((panel) => panel.dataset.panel)).toEqual(["general"]);
    expect(el.textContent).not.toContain("Changes save instantly");
    el.querySelector<HTMLButtonElement>('[data-page="display"]')!.click();
    expect(el.querySelector<HTMLButtonElement>('[data-page="display"]')!.getAttribute("aria-selected")).toBe("true");
    expect(el.querySelector<HTMLElement>('[data-panel="general"]')!.getAttribute("aria-hidden")).toBe("true");
    expect(el.querySelector<HTMLElement>('[data-panel="display"]')!.getAttribute("aria-hidden")).toBe("false");
    expect(el.querySelector(".theme-grid--single-column")).not.toBeNull();
    expect(el.querySelectorAll("[data-preview-theme]")).toHaveLength(4);
    expect(el.querySelectorAll(".theme-option small")).toHaveLength(0);
    expect(el.querySelector('[data-theme="frosted"]')).not.toBeNull();
    expect(el.querySelector('[data-theme="acrylic"]')).toBeNull();
    expect(el.querySelector('[data-theme="opaque"]')).toBeNull();
    el.querySelector<HTMLButtonElement>('[data-theme="solid"]')!.click();
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ theme: "solid" }));
    const color = el.querySelector<HTMLInputElement>("input[name=backgroundColor]")!;
    color.value = "#203040";
    color.dispatchEvent(new Event("input", { bubbles: true }));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ theme: "solid", backgroundColor: "#203040" }));
    // Switching back off Solid keeps the opacity Solid pinned rather than resurrecting the
    // pre-Solid value: the config only holds one opacity, and 1.0 is what was last written.
    el.querySelector<HTMLButtonElement>('[data-theme="frosted"]')!.click();
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ theme: "frosted", cardOpacity: 1 }));
    expect(el.querySelectorAll("[data-custom-theme-action][disabled]")).toHaveLength(3);
  });

  it("has no Theme page — theme lives inside Display", () => {
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() });
    expect(el.querySelector('[data-page="theme"]')).toBeNull();
    expect(el.querySelector('[data-panel="theme"]')).toBeNull();
    const display = el.querySelector<HTMLElement>('[data-panel="display"]')!;
    expect(display.querySelector(".theme-grid")).not.toBeNull();
    expect(display.querySelector("input[name=cardOpacity]")).not.toBeNull();
    expect(display.querySelector("input[name=backgroundColor]")).not.toBeNull();
  });

  it("offers Neon and no longer offers Blur, which Windows cannot actually render", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    expect(el.querySelector('[data-theme="blur"]')).toBeNull();
    expect(el.querySelector('[data-preview-theme="blur"]')).toBeNull();
    el.querySelector<HTMLButtonElement>('[data-theme="neon"]')!.click();
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ theme: "neon" }));
  });

  it("pins Solid to full opacity and disables the slider, since a translucent Solid is a contradiction", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    const opacity = el.querySelector<HTMLInputElement>("input[name=cardOpacity]")!;
    expect(opacity.disabled).toBe(false);

    el.querySelector<HTMLButtonElement>('[data-theme="solid"]')!.click();
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ theme: "solid", cardOpacity: 1 }));
    expect(opacity.disabled).toBe(true);
    expect(opacity.value).toBe("100");

    el.querySelector<HTMLButtonElement>('[data-theme="frosted"]')!.click();
    expect(opacity.disabled).toBe(false);
  });

  it("offers a meter shape for the overlay card, defaulting to the ring", () => {
    const onChange = vi.fn();
    const el = renderSettings(config, monitors, { onChange, onClose: vi.fn() });
    const shapes = Array.from(el.querySelectorAll<HTMLButtonElement>("[data-meter-shape]")).map((button) => button.dataset.meterShape);
    expect(shapes).toEqual(["ring", "charge", "reactor", "columns", "line", "semicircle"]);
    expect(Array.from(el.querySelectorAll<HTMLButtonElement>("[data-meter-shape] strong")).map((label) => label.textContent))
      .toEqual(["Ring", "Charge", "Arc Reactor", "Columns", "Line", "Semi Circle"]);
    expect(el.querySelector<HTMLButtonElement>('[data-meter-shape="ring"]')!.getAttribute("aria-pressed")).toBe("true");
    el.querySelector<HTMLButtonElement>('[data-meter-shape="reactor"]')!.click();
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ meterShape: "reactor" }));
    expect(el.querySelector<HTMLButtonElement>('[data-meter-shape="reactor"]')!.getAttribute("aria-pressed")).toBe("true");
    expect(el.querySelector<HTMLButtonElement>('[data-meter-shape="ring"]')!.getAttribute("aria-pressed")).toBe("false");
  });

  it("opens directly on the page named by initialPage instead of always defaulting to General", () => {
    const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, null, "account");
    expect(el.querySelector<HTMLButtonElement>('[data-page="account"]')!.getAttribute("aria-selected")).toBe("true");
    expect(el.querySelector<HTMLButtonElement>('[data-page="general"]')!.getAttribute("aria-selected")).toBe("false");
    expect(el.querySelector<HTMLElement>('[data-panel="account"]')!.getAttribute("aria-hidden")).toBe("false");
    expect(el.querySelector<HTMLElement>('[data-panel="general"]')!.getAttribute("aria-hidden")).toBe("true");
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

  describe("Claude.ai session key", () => {
    function openAccount(el: HTMLElement) {
      el.querySelector<HTMLButtonElement>('[data-page="account"]')!.click();
    }

    it("offers a session key field, since extra credit lives on claude.ai behind the cookie", () => {
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() });
      openAccount(el);
      const field = el.querySelector<HTMLInputElement>("[data-session-key-input]")!;
      expect(field).not.toBeNull();
      // The key is a credential: never rendered back into the DOM, and masked while typed.
      expect(field.type).toBe("password");
      expect(field.value).toBe("");
    });

    it("saves a pasted key and reports connected without echoing it back", async () => {
      const onSaveSessionKey = vi.fn().mockResolvedValue(undefined);
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn(), onSaveSessionKey });
      openAccount(el);
      const field = el.querySelector<HTMLInputElement>("[data-session-key-input]")!;
      field.value = "sk-ant-sid01-abcdefghijklmnop";
      el.querySelector<HTMLButtonElement>("[data-session-key-save]")!.click();
      await Promise.resolve();
      await Promise.resolve();

      expect(onSaveSessionKey).toHaveBeenCalledWith("sk-ant-sid01-abcdefghijklmnop");
      expect(field.value).toBe("");
    });

    it("surfaces a rejected key inline instead of failing silently", async () => {
      const onSaveSessionKey = vi.fn().mockRejectedValue("That does not look like a session key.");
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn(), onSaveSessionKey });
      openAccount(el);
      el.querySelector<HTMLInputElement>("[data-session-key-input]")!.value = "nope";
      el.querySelector<HTMLButtonElement>("[data-session-key-save]")!.click();
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();

      expect(el.querySelector<HTMLElement>("[data-session-key-status]")!.textContent)
        .toContain("does not look like a session key");
    });

    it("keeps the manual paste behind a disclosure", () => {
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() });
      const manual = el.querySelector<HTMLDetailsElement>(".session-key__manual")!;

      expect(manual.tagName).toBe("DETAILS");
      expect(manual.open).toBe(false);
      expect(manual.querySelector("[data-session-key-input]")).not.toBeNull();
      expect(manual.querySelector("[data-session-key-save]")).not.toBeNull();
    });

    it("describes manual app-owned storage before and after connection", () => {
      const before = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, null, "account", false);
      expect(before.querySelector(".session-key__description")!.textContent).toContain("Optionally connect claude.ai");

      const after = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, null, "account", true);
      expect(after.querySelector(".session-key__description")!.textContent).toContain("Stored by Usage Tracker");
    });

    it("shows a disconnect action only once a key is stored", async () => {
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, null, "account", true);
      expect(el.querySelector("[data-session-key-clear]")).not.toBeNull();

      const without = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, null, "account", false);
      expect(without.querySelector("[data-session-key-clear]")).toBeNull();
    });

    it("does not report a claude.ai disconnect when secure deletion fails", async () => {
      const onClearSessionKey = vi.fn().mockRejectedValue("Windows Credential Manager is unavailable.");
      const el = renderSettings(
        config,
        monitors,
        { onChange: vi.fn(), onClose: vi.fn(), onClearSessionKey },
        null,
        "account",
        true,
      );
      el.querySelector<HTMLButtonElement>("[data-session-key-clear]")!.click();

      await vi.waitFor(() =>
        expect(el.querySelector("[data-session-key-status]")!.textContent).toMatch(/unavailable/i),
      );
      expect(el.querySelector("[data-session-key-status]")!.textContent).not.toBe("Not connected.");
    });
  });

  describe("Claude account panel", () => {
    function openAccountPanel(el: HTMLElement) {
      el.querySelector<HTMLButtonElement>('[data-page="account"]')!.click();
    }

    it("directs signed-out users to Claude Code without offering app-owned OAuth", () => {
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, null);
      openAccountPanel(el);
      const account = el.querySelector<HTMLElement>("[data-claude-account]")!;
      expect(account.textContent).toContain("Claude Code is not signed in");
      expect(account.textContent).toContain("Sign in from Claude Code");
      expect(account.querySelector("button")).toBeNull();
      expect(account.querySelector("input")).toBeNull();
    });

    it("shows a read-only Claude Code connection without a log-out button", () => {
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, { organizationUuid: "org-12345678-abcd" });
      openAccountPanel(el);
      const account = el.querySelector<HTMLElement>("[data-claude-account]")!;
      expect(account.textContent).toContain("Connected through Claude Code");
      expect(account.textContent).toContain("org-1234");
      expect(account.querySelector("button")).toBeNull();
    });

    it("explains that the CLI owns the credential in both states", () => {
      const signedOut = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, null);
      openAccountPanel(signedOut);
      expect(signedOut.querySelector<HTMLElement>("[data-claude-account]")!.textContent).toContain("Usage Tracker only reads");

      const signedIn = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, { organizationUuid: "org-1" });
      openAccountPanel(signedIn);
      expect(signedIn.querySelector<HTMLElement>("[data-claude-account]")!.textContent).toContain("Manage this session in Claude Code");
    });

    it("prefers the account email over the org id once the backend provides one", () => {
      const el = renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn() }, { organizationUuid: "org-12345678-abcd", email: "person@example.com" });
      openAccountPanel(el);
      expect(el.textContent).toContain("Connected through Claude Code as person@example.com");
      expect(el.textContent).not.toContain("org-1234");
    });
  });
});

describe("Anthropic Console session key", () => {
  const render = (connected = false, actions = {}) =>
    renderSettings(config, monitors, { onChange: vi.fn(), onClose: vi.fn(), ...actions }, null, "account", false, connected);

  it("names the Console sign-in as separate from claude.ai", () => {
    const el = render();
    const section = el.querySelector("[aria-labelledby='console-keys-title']")!;
    expect(section.textContent).toMatch(/separate/i);
    expect(section.textContent).toContain("platform.claude.com");
  });

  it("shows connected state and a disconnect control only when a key is stored", () => {
    expect(render(false).querySelector("[data-console-key-clear]")).toBeNull();
    const connected = render(true);
    expect(connected.querySelector("[data-console-key-clear]")).not.toBeNull();
    expect(connected.querySelector("[data-console-keys-status]")!.textContent).toBe("Connected.");
  });

  it("saves a pasted key and clears the field", async () => {
    const onSaveConsoleSessionKey = vi.fn().mockResolvedValue(undefined);
    const el = render(false, { onSaveConsoleSessionKey });
    const input = el.querySelector<HTMLInputElement>("[data-console-key-input]")!;
    input.value = "  sk-ant-sid01-fixture  ";
    el.querySelector<HTMLButtonElement>("[data-console-key-save]")!.click();
    await vi.waitFor(() => expect(onSaveConsoleSessionKey).toHaveBeenCalledWith("sk-ant-sid01-fixture"));
    await vi.waitFor(() => expect(input.value).toBe(""));
    expect(el.innerHTML).not.toContain("sk-ant-sid01-fixture");
  });

  it("keeps a rejected key in the field and shows why", async () => {
    const onSaveConsoleSessionKey = vi.fn().mockRejectedValue("That key is the wrong length — copy the cookie's full value.");
    const el = render(false, { onSaveConsoleSessionKey });
    const input = el.querySelector<HTMLInputElement>("[data-console-key-input]")!;
    input.value = "nope";
    el.querySelector<HTMLButtonElement>("[data-console-key-save]")!.click();
    await vi.waitFor(() =>
      expect(el.querySelector("[data-console-keys-status]")!.textContent).toMatch(/wrong length/),
    );
    expect(input.value).toBe("nope");
  });

  it("refuses an empty submit without calling the backend", () => {
    const onSaveConsoleSessionKey = vi.fn();
    const el = render(false, { onSaveConsoleSessionKey });
    el.querySelector<HTMLButtonElement>("[data-console-key-save]")!.click();
    expect(onSaveConsoleSessionKey).not.toHaveBeenCalled();
    expect(el.querySelector("[data-console-keys-status]")!.textContent).toMatch(/paste/i);
  });

  it("does not report a Console disconnect when secure deletion fails", async () => {
    const onClearConsoleSessionKey = vi.fn().mockRejectedValue("Could not delete the stored credential.");
    const el = render(true, { onClearConsoleSessionKey });
    el.querySelector<HTMLButtonElement>("[data-console-key-clear]")!.click();

    await vi.waitFor(() =>
      expect(el.querySelector("[data-console-keys-status]")!.textContent).toMatch(/could not delete/i),
    );
    expect(el.querySelector("[data-console-keys-status]")!.textContent).not.toBe("Not connected.");
  });
});

describe("renderConsoleCosts", () => {
  const money = (minorUnits: string) => ({ minorUnits, currency: "USD" });
  const fresh = <T,>(value: T) => ({ value, state: "fresh", errorCode: null });
  const gone = (errorCode: string | null) => ({ value: null, state: "unavailable", errorCode });
  const view = (over: Partial<ConsoleCostsView> = {}): ConsoleCostsView => ({
    period: { startsAt: "2026-08-01T00:00:00Z", endsAt: "2026-09-01T00:00:00Z", timezone: "UTC" },
    spend: fresh(money("39352052")),
    prepaidBalance: fresh(money("50000000")),
    daily: fresh([{ key: "2026-08-01", label: "2026-08-01", amount: money("21585640") }]),
    byApiKey: fresh([{ key: "id-01", label: "Key …d-01", amount: money("39352052") }]),
    byModel: fresh([
      { key: "claude-sonnet-4", label: "claude-sonnet-4", amount: money("10595210") },
      { key: "claude-opus-4", label: "claude-opus-4", amount: money("17667688") },
    ]),
    ...over,
  });
  const render = (v = view()) => {
    const host = document.createElement("div");
    renderConsoleCosts(host, v);
    return host;
  };

  it("renders micro-units as money, not as raw integers", () => {
    const el = render();
    expect(el.querySelector(".console-costs__amount")!.textContent).toMatch(/39\.3521$/);
    expect(el.textContent).not.toContain("39352052");
  });

  it("renders the UTC period as plain dates", () => {
    expect(render().querySelector(".console-costs__period")!.textContent).toBe("2026-08-01 – 2026-09-01 (UTC)");
  });

  it("never renders an unavailable section as a zero amount", () => {
    const el = render(view({ spend: gone("insufficientRole"), prepaidBalance: gone("noCredential") }));
    expect(el.textContent).toContain("role cannot read this");
    expect(el.textContent).toContain("Connect a Console session key");
    expect(el.textContent).not.toMatch(/[\d]0\.00/);
  });

  it("keeps available sections when another is unavailable", () => {
    const el = render(view({ prepaidBalance: gone("insufficientRole") }));
    expect(el.querySelector(".console-costs__amount")!.textContent).toMatch(/39\.3521$/);
  });

  it("explains an unsupported breakdown rather than showing an empty table", () => {
    const el = render(view({ byApiKey: gone("unsupportedBySource") }));
    expect(el.textContent).toContain("Not available from the Console API");
  });

  it("sorts breakdown rows by amount descending", () => {
    const tables = [...render().querySelectorAll("table")];
    const rows = tables[tables.length - 2].querySelectorAll("tbody th");
    expect([...rows].map((r) => r.textContent)).toEqual(["claude-opus-4", "claude-sonnet-4"]);
  });

  it("shows only the redacted key label", () => {
    expect(render().textContent).toContain("Key …d-01");
    expect(render().textContent).not.toContain("id-01");
  });

  it("is axe-clean", async () => {
    const el = render();
    document.body.appendChild(el);
    try {
      expect((await axe(el)).violations).toEqual([]);
    } finally {
      el.remove();
    }
  });
});
