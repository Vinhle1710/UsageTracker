import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { enable } from "@tauri-apps/plugin-autostart";
import { renderControls, type ControlAction } from "./components/controls";
import { renderLayer } from "./components/layer";
import { formatPercent } from "./format";
import { nextSize, visibleLayers, worstPercent } from "./state";
import type { ActiveSources, Config, SizeState, UsageSnapshot } from "./types";
import "./styles/app.css";

const now = () => Math.floor(Date.now() / 1000);
const demoSnapshot = (used: number, resetAfter: number): UsageSnapshot => ({
  windows: [{ label: "5 hour", used_percent: used, resets_at: now() + resetAfter }, { label: "Weekly", used_percent: Math.min(100, used + 18), resets_at: now() + 3 * 86400 }],
  fetched_at: now(),
  state: "fresh",
});

const app = document.querySelector<HTMLElement>("#app")!;
let sizeState: SizeState = "compact";
let alwaysOnTop = true;
let settingsOpen = false;
let sources: ActiveSources = { claude: true, openai: true };
const snapshots: Partial<Record<"claude" | "openai", UsageSnapshot>> = {
  claude: demoSnapshot(21, 2 * 3600),
  openai: demoSnapshot(34, 5 * 3600),
};
let config: Config = { monitorId: null, corner: "bottom-right", scale: 1, sizeState, alwaysOnTop, offscreenPeek: false, pollIntervalSec: 60, detectIntervalSec: 5 };

function render(): void {
  const active = visibleLayers(sources);
  const shown = active.map((provider) => snapshots[provider]).filter((snapshot): snapshot is UsageSnapshot => Boolean(snapshot));
  app.dataset.size = sizeState;
  app.innerHTML = "";

  const header = document.createElement("header");
  header.className = "panel-header";
  const title = document.createElement("div");
  title.innerHTML = `<span class="eyebrow">LIVE QUOTA</span><h1>Usage tracker</h1>`;
  header.appendChild(title);
  header.appendChild(renderControls({ sizeState, alwaysOnTop }, handleAction));
  app.appendChild(header);

  if (sizeState === "bubble") {
    const bubble = document.createElement("button");
    bubble.className = "bubble";
    bubble.type = "button";
    bubble.setAttribute("aria-label", `Highest usage ${formatPercent(worstPercent(shown) ?? 0)}. Expand usage tracker`);
    bubble.textContent = formatPercent(worstPercent(shown) ?? 0);
    bubble.addEventListener("click", () => { sizeState = "compact"; render(); });
    app.appendChild(bubble);
    return;
  }

  const status = document.createElement("p");
  status.className = "status-line";
  status.textContent = active.length ? `${active.length} source${active.length === 1 ? "" : "s"} active · refreshes every minute` : "No supported AI client detected";
  app.appendChild(status);

  const content = document.createElement("div");
  content.className = "layers";
  for (const provider of active) {
    const snapshot = snapshots[provider];
    if (snapshot) content.appendChild(renderLayer(provider === "claude" ? "Claude" : "Codex / ChatGPT", snapshot, now()));
  }
  if (!active.length) {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.textContent = "Open Claude, ChatGPT, Codex, or a supported VS Code extension to start tracking.";
    content.appendChild(empty);
  }
  app.appendChild(content);
  if (settingsOpen) app.appendChild(renderSettings());
}

function renderSettings(): HTMLElement {
  const section = document.createElement("section");
  section.className = "settings";
  section.setAttribute("aria-label", "Overlay settings");
  section.innerHTML = `<h2>Settings</h2><label>Monitor ID<input name="monitorId" value="${config.monitorId ?? ""}" placeholder="Primary monitor" /></label><label>Corner<select name="corner"><option value="top-left">Top left</option><option value="top-right">Top right</option><option value="bottom-left">Bottom left</option><option value="bottom-right">Bottom right</option></select></label><label>Scale <output id="scale-value">${Math.round(config.scale * 100)}%</output><input name="scale" type="range" min="75" max="150" value="${Math.round(config.scale * 100)}" /></label><button type="button" data-save>Save settings</button>`;
  const corner = section.querySelector<HTMLSelectElement>("[name=corner]")!;
  corner.value = config.corner;
  const scale = section.querySelector<HTMLInputElement>("[name=scale]")!;
  const output = section.querySelector<HTMLOutputElement>("#scale-value")!;
  scale.addEventListener("input", () => { output.value = `${scale.value}%`; });
  section.querySelector("[data-save]")!.addEventListener("click", () => {
    config = { ...config, monitorId: section.querySelector<HTMLInputElement>("[name=monitorId]")!.value || null, corner: corner.value, scale: Number(scale.value) / 100 };
    void invoke("set_config", { cfg: config }).catch(() => undefined);
    void invoke("apply_placement", { corner: config.corner, preferred: config.monitorId }).catch(() => undefined);
    settingsOpen = false;
    render();
  });
  return section;
}

function handleAction(action: ControlAction): void {
  if (action === "bubble") sizeState = "bubble";
  if (action === "resize") sizeState = nextSize(sizeState);
  if (action === "pin") { alwaysOnTop = !alwaysOnTop; config = { ...config, alwaysOnTop }; void invoke("set_config", { cfg: config }).catch(() => undefined); }
  if (action === "settings") settingsOpen = !settingsOpen;
  render();
}

async function connectNative(): Promise<void> {
  try {
    config = await invoke<Config>("get_config");
    await enable();
    sizeState = config.sizeState;
    alwaysOnTop = config.alwaysOnTop;
    await invoke("apply_placement", { corner: config.corner, preferred: config.monitorId });
    await listen<ActiveSources>("sources-changed", (event) => { sources = event.payload; render(); });
    await listen<UsageSnapshot>("claude-usage", (event) => { snapshots.claude = event.payload; render(); });
    await listen<UsageSnapshot>("codex-usage", (event) => { snapshots.openai = event.payload; render(); });
  } catch {
    // Vite browser preview has no Tauri runtime; the demo state keeps the UI inspectable.
  }
  render();
}

void connectNative();
