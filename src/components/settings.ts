import type { Config, MonitorOption, ThemePreset } from "../types";

export interface SettingsActions {
  onChange: (config: Config) => void;
  onClose: () => void;
  onDrag?: () => void;
}

const presetOpacity: Record<Exclude<ThemePreset, "custom">, number> = {
  clear: 0.86,
  opaque: 0.96,
  solid: 1,
};

export function renderSettings(config: Config, monitors: MonitorOption[], actions: SettingsActions): HTMLElement {
  const root = document.createElement("main");
  root.className = "settings-window";
  root.setAttribute("aria-labelledby", "settings-title");
  root.innerHTML = `
    <header class="settings-window__header" data-drag-handle>
      <div>
        <p class="settings-window__eyebrow">Usage Tracker</p>
        <h1 id="settings-title">Settings</h1>
      </div>
      <button type="button" data-close aria-label="Close settings">×</button>
    </header>
    <div class="settings-layout">
      <nav class="settings-nav" aria-label="Settings pages" role="tablist" aria-orientation="vertical">
        <button id="settings-page-general" type="button" role="tab" data-page="general" aria-controls="settings-general" aria-selected="true">General</button>
        <button id="settings-page-display" type="button" role="tab" data-page="display" aria-controls="settings-display" aria-selected="false" tabindex="-1">Display</button>
        <button id="settings-page-theme" type="button" role="tab" data-page="theme" aria-controls="settings-theme" aria-selected="false" tabindex="-1">Theme</button>
        <button id="settings-page-behavior" type="button" role="tab" data-page="behavior" aria-controls="settings-behavior" aria-selected="false" tabindex="-1">Behavior</button>
      </nav>
      <div class="settings-pages">
        <section id="settings-general" class="settings-panel" data-panel="general" role="tabpanel" aria-labelledby="settings-page-general">
          <div class="settings-panel__intro"><h2>General</h2><p>Choose where the overlay lives.</p></div>
          <label>Monitor<select name="monitorId"></select></label>
          <label>Corner<select name="corner">
            <option value="top-left">Top left</option>
            <option value="top-right">Top right</option>
            <option value="bottom-left">Bottom left</option>
            <option value="bottom-right">Bottom right</option>
          </select></label>
        </section>
        <section id="settings-display" class="settings-panel" data-panel="display" role="tabpanel" aria-labelledby="settings-page-display" hidden>
          <div class="settings-panel__intro"><h2>Display</h2><p>Arrange and scale the provider cards.</p></div>
          <label>Layout<select name="layout">
            <option value="stacked-compact">Vertical</option>
            <option value="provider-columns">Horizontal</option>
          </select></label>
          <label>Scale <output data-scale-value>${Math.round(config.scale * 100)}%</output><input name="scale" type="range" min="75" max="150" step="5" value="${Math.round(config.scale * 100)}" /></label>
        </section>
        <section id="settings-theme" class="settings-panel" data-panel="theme" role="tabpanel" aria-labelledby="settings-page-theme" hidden>
          <div class="settings-panel__intro"><h2>Theme</h2><p>Pick a surface style or make your own.</p></div>
          <div class="theme-grid" role="group" aria-label="Theme presets">
            <button type="button" class="theme-option" data-theme="clear" aria-pressed="${config.theme === "clear"}"><span class="theme-preview theme-preview--clear"><i></i><i></i></span><strong>Clear</strong><small>Light and airy</small></button>
            <button type="button" class="theme-option" data-theme="opaque" aria-pressed="${config.theme === "opaque"}"><span class="theme-preview theme-preview--opaque"><i></i><i></i></span><strong>Opaque</strong><small>Balanced default</small></button>
            <button type="button" class="theme-option" data-theme="solid" aria-pressed="${config.theme === "solid"}"><span class="theme-preview theme-preview--solid"><i></i><i></i></span><strong>Solid</strong><small>Maximum contrast</small></button>
            <button type="button" class="theme-option" data-theme="custom" aria-pressed="${config.theme === "custom"}"><span class="theme-preview theme-preview--custom"><i></i><i></i></span><strong>Custom</strong><small>Your color and opacity</small></button>
          </div>
          <label class="settings-color">Navy background <span class="settings-color__control"><input name="backgroundColor" type="color" value="${config.backgroundColor}" aria-label="Choose background color" /><output data-color-value>${config.backgroundColor.toUpperCase()}</output></span></label>
          <label>Card opacity <output data-opacity-value>${Math.round(config.cardOpacity * 100)}%</output><input name="cardOpacity" type="range" min="70" max="100" step="1" value="${Math.round(config.cardOpacity * 100)}" /></label>
        </section>
        <section id="settings-behavior" class="settings-panel" data-panel="behavior" role="tabpanel" aria-labelledby="settings-page-behavior" hidden>
          <div class="settings-panel__intro"><h2>Behavior</h2><p>Control how the overlay stays visible.</p></div>
          <label class="settings-toggle"><input name="alwaysOnTop" type="checkbox" ${config.alwaysOnTop ? "checked" : ""} /> Always on top</label>
        </section>
      </div>
    </div>
    <p class="settings-feedback" data-feedback aria-live="polite"></p>`;

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
  const opacity = root.querySelector<HTMLInputElement>("input[name=cardOpacity]")!;
  const opacityValue = root.querySelector<HTMLOutputElement>("[data-opacity-value]")!;
  const color = root.querySelector<HTMLInputElement>("input[name=backgroundColor]")!;
  const colorValue = root.querySelector<HTMLOutputElement>("[data-color-value]")!;
  const alwaysOnTop = root.querySelector<HTMLInputElement>("input[name=alwaysOnTop]")!;
  const feedback = root.querySelector<HTMLElement>("[data-feedback]")!;

  let current = { ...config };
  const announce = (message: string) => {
    feedback.textContent = message;
    feedback.classList.remove("settings-feedback--visible");
    requestAnimationFrame(() => feedback.classList.add("settings-feedback--visible"));
  };
  const commit = (patch: Partial<Config>, message = "Saved") => {
    current = { ...current, ...patch };
    actions.onChange(current);
    announce(message);
  };
  const activatePage = (page: string) => {
    root.querySelectorAll<HTMLButtonElement>('[role="tab"]').forEach((button) => {
      const selected = button.dataset.page === page;
      button.setAttribute("aria-selected", String(selected));
      button.tabIndex = selected ? 0 : -1;
    });
    root.querySelectorAll<HTMLElement>("[data-panel]").forEach((panel) => {
      panel.hidden = panel.dataset.panel !== page;
    });
  };
  const syncThemeControls = () => {
    root.querySelectorAll<HTMLButtonElement>("[data-theme]").forEach((button) => button.setAttribute("aria-pressed", String(button.dataset.theme === current.theme)));
    opacity.value = String(Math.round(current.cardOpacity * 100));
    opacityValue.value = `${opacity.value}%`;
    color.value = current.backgroundColor;
    colorValue.value = current.backgroundColor.toUpperCase();
  };

  monitor.addEventListener("change", () => commit({ monitorId: monitor.value || null }));
  corner.addEventListener("change", () => commit({ corner: corner.value }));
  layout.addEventListener("change", () => commit({ layout: layout.value as Config["layout"] }));
  scale.addEventListener("input", () => {
    scaleValue.value = `${scale.value}%`;
    commit({ scale: Number(scale.value) / 100 });
  });
  opacity.addEventListener("input", () => {
    opacityValue.value = `${opacity.value}%`;
    commit({ cardOpacity: Number(opacity.value) / 100, theme: "custom" });
  });
  color.addEventListener("input", () => {
    colorValue.value = color.value.toUpperCase();
    commit({ backgroundColor: color.value, theme: "custom" });
  });
  alwaysOnTop.addEventListener("change", () => commit({ alwaysOnTop: alwaysOnTop.checked }));
  const tabs = Array.from(root.querySelectorAll<HTMLButtonElement>('[role="tab"]'));
  tabs.forEach((button, index) => {
    button.addEventListener("click", () => activatePage(button.dataset.page ?? "general"));
    button.addEventListener("keydown", (event) => {
      if (event.key !== "ArrowDown" && event.key !== "ArrowRight" && event.key !== "ArrowUp" && event.key !== "ArrowLeft") return;
      event.preventDefault();
      const direction = event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : -1;
      const next = tabs[(index + direction + tabs.length) % tabs.length];
      next.focus();
      next.click();
    });
  });
  root.querySelectorAll<HTMLButtonElement>("[data-theme]").forEach((button) => button.addEventListener("click", () => {
    const theme = button.dataset.theme as ThemePreset;
    const patch = theme === "custom" ? { theme } : { theme, cardOpacity: presetOpacity[theme as Exclude<ThemePreset, "custom">] };
    commit(patch, "Theme updated");
    syncThemeControls();
  }));
  root.querySelector<HTMLButtonElement>("[data-close]")!.addEventListener("click", actions.onClose);
  root.querySelector<HTMLElement>("[data-drag-handle]")!.addEventListener("mousedown", (event) => {
    if (event.button === 0 && !(event.target as Element).closest("button, input, select")) actions.onDrag?.();
  });
  return root;
}
