import type { Config, MonitorOption } from "../types";

export interface SettingsActions {
  onChange: (config: Config) => void;
  onClose: () => void;
}

export function renderSettings(config: Config, monitors: MonitorOption[], actions: SettingsActions): HTMLElement {
  const root = document.createElement("main");
  root.className = "settings-window";
  root.setAttribute("aria-labelledby", "settings-title");
  root.innerHTML = `
    <header class="settings-window__header">
      <h1 id="settings-title">Settings</h1>
      <button type="button" data-close aria-label="Close settings">×</button>
    </header>
    <label>Screen<select name="monitorId"></select></label>
    <label>Corner<select name="corner">
      <option value="top-left">Top left</option>
      <option value="top-right">Top right</option>
      <option value="bottom-left">Bottom left</option>
      <option value="bottom-right">Bottom right</option>
    </select></label>
    <label>Layout<select name="layout">
      <option value="stacked-compact">Stacked compact</option>
      <option value="provider-columns">Provider columns</option>
    </select></label>
    <label>Scale <output data-scale-value>${Math.round(config.scale * 100)}%</output><input name="scale" type="range" min="75" max="150" step="5" value="${Math.round(config.scale * 100)}" /></label>
    <label class="settings-toggle"><input name="alwaysOnTop" type="checkbox" ${config.alwaysOnTop ? "checked" : ""} /> Always on top</label>
    <p class="settings-window__note">Changes save instantly.</p>`;

  const monitor = root.querySelector<HTMLSelectElement>("select[name=monitorId]")!;
  for (const option of monitors) {
    const item = document.createElement("option");
    item.value = option.id;
    item.textContent = option.label;
    monitor.appendChild(item);
  }
  monitor.value = config.monitorId ?? monitors[0]?.id ?? "";

  const corner = root.querySelector<HTMLSelectElement>("select[name=corner]")!;
  corner.value = config.corner;
  const layout = root.querySelector<HTMLSelectElement>("select[name=layout]")!;
  layout.value = config.layout;
  const scale = root.querySelector<HTMLInputElement>("input[name=scale]")!;
  const scaleValue = root.querySelector<HTMLOutputElement>("[data-scale-value]")!;
  const alwaysOnTop = root.querySelector<HTMLInputElement>("input[name=alwaysOnTop]")!;

  let current = { ...config };
  const commit = (patch: Partial<Config>) => {
    current = { ...current, ...patch };
    actions.onChange(current);
  };
  monitor.addEventListener("change", () => commit({ monitorId: monitor.value || null }));
  corner.addEventListener("change", () => commit({ corner: corner.value }));
  layout.addEventListener("change", () => commit({ layout: layout.value as Config["layout"] }));
  scale.addEventListener("input", () => {
    scaleValue.value = `${scale.value}%`;
    commit({ scale: Number(scale.value) / 100 });
  });
  alwaysOnTop.addEventListener("change", () => commit({ alwaysOnTop: alwaysOnTop.checked }));
  root.querySelector<HTMLButtonElement>("[data-close]")!.addEventListener("click", actions.onClose);
  return root;
}
