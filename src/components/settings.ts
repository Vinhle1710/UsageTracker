import type { Config, MonitorOption, ThemePreset } from "../types";

export interface SettingsActions {
  onChange: (config: Config) => void;
  onClose: () => void;
  onDrag?: () => void;
}

interface SelectOption {
  value: string;
  label: string;
}

let selectSequence = 0;

function createCustomSelect(name: string, label: string, value: string, options: SelectOption[], onChange: (value: string) => void): HTMLElement {
  const id = `settings-select-${++selectSequence}`;
  const field = document.createElement("div");
  field.className = "settings-field";
  field.dataset.select = name;
  field.innerHTML = `
    <span class="settings-field__label" id="${id}-label">${label}</span>
    <div class="custom-select">
      <button type="button" class="custom-select__trigger" role="combobox" aria-haspopup="listbox" aria-expanded="false" aria-controls="${id}-list" aria-labelledby="${id}-label ${id}-value">
        <span id="${id}-value" data-select-value></span>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 6 4 4 4-4" /></svg>
      </button>
      <ul id="${id}-list" class="custom-select__list" role="listbox" aria-labelledby="${id}-label" hidden></ul>
    </div>`;

  const trigger = field.querySelector<HTMLButtonElement>(".custom-select__trigger")!;
  const valueLabel = field.querySelector<HTMLElement>("[data-select-value]")!;
  const list = field.querySelector<HTMLUListElement>("[role=listbox]")!;
  let current = options.some((option) => option.value === value) ? value : options[0]?.value ?? "";

  const close = (restoreFocus = false) => {
    trigger.setAttribute("aria-expanded", "false");
    list.hidden = true;
    if (restoreFocus) trigger.focus();
  };
  const focusOption = (index: number) => {
    const items = Array.from(list.querySelectorAll<HTMLElement>("[role=option]"));
    items[(index + items.length) % items.length]?.focus();
  };
  const open = (focusIndex?: number) => {
    trigger.setAttribute("aria-expanded", "true");
    list.hidden = false;
    const items = Array.from(list.querySelectorAll<HTMLElement>("[role=option]"));
    const selectedIndex = Math.max(0, items.findIndex((item) => item.dataset.value === current));
    queueMicrotask(() => focusOption(focusIndex ?? selectedIndex));
  };
  const select = (next: string) => {
    current = next;
    const selected = options.find((option) => option.value === current);
    valueLabel.textContent = selected?.label ?? current;
    list.querySelectorAll<HTMLElement>("[role=option]").forEach((item) => item.setAttribute("aria-selected", String(item.dataset.value === current)));
    close(true);
    onChange(current);
  };

  for (const option of options) {
    const item = document.createElement("li");
    item.className = "custom-select__option";
    item.dataset.value = option.value;
    item.setAttribute("role", "option");
    item.setAttribute("tabindex", "-1");
    item.setAttribute("aria-selected", String(option.value === current));
    item.textContent = option.label;
    item.addEventListener("click", () => select(option.value));
    list.appendChild(item);
  }
  valueLabel.textContent = options.find((option) => option.value === current)?.label ?? current;

  trigger.addEventListener("click", () => trigger.getAttribute("aria-expanded") === "true" ? close() : open());
  trigger.addEventListener("keydown", (event) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const last = Math.max(0, options.length - 1);
    open(event.key === "ArrowUp" || event.key === "End" ? last : 0);
  });
  list.addEventListener("keydown", (event) => {
    const items = Array.from(list.querySelectorAll<HTMLElement>("[role=option]"));
    const index = items.indexOf(document.activeElement as HTMLElement);
    if (event.key === "Escape" || event.key === "Tab") {
      close(event.key === "Escape");
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (index >= 0) select(items[index].dataset.value ?? "");
      return;
    }
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    if (event.key === "Home") focusOption(0);
    else if (event.key === "End") focusOption(items.length - 1);
    else focusOption(index + (event.key === "ArrowDown" ? 1 : -1));
  });
  field.addEventListener("focusout", (event) => {
    if (!field.contains(event.relatedTarget as Node | null)) close();
  });
  return field;
}

export function renderSettings(config: Config, monitors: MonitorOption[], actions: SettingsActions): HTMLElement {
  const root = document.createElement("main");
  root.className = "settings-window";
  root.setAttribute("aria-labelledby", "settings-title");
  root.innerHTML = `
    <header class="settings-window__header" data-drag-handle data-tauri-drag-region>
      <div class="settings-window__identity">
        <p class="settings-window__eyebrow">Usage Tracker</p>
        <h1 id="settings-title">Settings</h1>
      </div>
      <button class="settings-window__close" type="button" data-close aria-label="Close settings">×</button>
    </header>
    <div class="settings-layout">
      <nav class="settings-nav" aria-label="Settings pages" role="tablist" aria-orientation="vertical">
        <button id="settings-page-general" type="button" role="tab" data-page="general" aria-controls="settings-general" aria-selected="true">General</button>
        <button id="settings-page-display" type="button" role="tab" data-page="display" aria-controls="settings-display" aria-selected="false" tabindex="-1">Display</button>
        <button id="settings-page-theme" type="button" role="tab" data-page="theme" aria-controls="settings-theme" aria-selected="false" tabindex="-1">Theme</button>
        <button id="settings-page-behavior" type="button" role="tab" data-page="behavior" aria-controls="settings-behavior" aria-selected="false" tabindex="-1">Behavior</button>
      </nav>
      <div class="settings-pages">
        <section id="settings-general" class="settings-panel" data-panel="general" role="tabpanel" aria-labelledby="settings-page-general" aria-hidden="false">
          <div class="settings-panel__intro"><h2>General</h2><p>Choose where the overlay lives.</p></div>
          <div data-select-mount="monitorId"></div>
          <div data-select-mount="corner"></div>
        </section>
        <section id="settings-display" class="settings-panel" data-panel="display" role="tabpanel" aria-labelledby="settings-page-display" aria-hidden="true" hidden>
          <div class="settings-panel__intro"><h2>Display</h2><p>Arrange and scale the provider cards.</p></div>
          <div data-select-mount="layout"></div>
          <label>Scale <output data-scale-value>${Math.round(config.scale * 100)}%</output><input name="scale" type="range" min="75" max="150" step="5" value="${Math.round(config.scale * 100)}" /></label>
        </section>
        <section id="settings-theme" class="settings-panel" data-panel="theme" role="tabpanel" aria-labelledby="settings-page-theme" aria-hidden="true" hidden>
          <div class="settings-panel__intro"><h2>Theme</h2><p>Choose how the cards blend with your desktop.</p></div>
          <div class="theme-grid theme-grid--single-column" role="group" aria-label="Theme presets">
            <button type="button" class="theme-option" data-theme="clear" aria-pressed="${config.theme === "clear"}"><span class="theme-preview theme-preview--clear" data-preview-theme="clear"><i></i><i></i></span><strong>Clear</strong></button>
            <button type="button" class="theme-option" data-theme="acrylic" aria-pressed="${config.theme === "acrylic"}"><span class="theme-preview theme-preview--acrylic" data-preview-theme="acrylic"><i></i><i></i></span><strong>Acrylic</strong></button>
            <button type="button" class="theme-option" data-theme="blur" aria-pressed="${config.theme === "blur"}"><span class="theme-preview theme-preview--blur" data-preview-theme="blur"><i></i><i></i></span><strong>Blur</strong></button>
            <button type="button" class="theme-option" data-theme="solid" aria-pressed="${config.theme === "solid"}"><span class="theme-preview theme-preview--solid" data-preview-theme="solid"><i></i><i></i></span><strong>Solid</strong></button>
          </div>
          <label class="settings-color">Background <span class="settings-color__control"><input name="backgroundColor" type="color" value="${config.backgroundColor}" aria-label="Choose card background color" /><output data-color-value>${config.backgroundColor.toUpperCase()}</output></span></label>
          <label>Card opacity <output data-opacity-value>${Math.round(config.cardOpacity * 100)}%</output><input name="cardOpacity" type="range" min="70" max="100" step="1" value="${Math.round(config.cardOpacity * 100)}" /></label>
          <section class="custom-theme-tools" aria-labelledby="custom-theme-title">
            <div><h3 id="custom-theme-title">Custom themes</h3><span>Coming later</span></div>
            <div class="custom-theme-tools__actions">
              <button type="button" data-custom-theme-action="create" disabled>Create</button>
              <button type="button" data-custom-theme-action="import" disabled>Import</button>
              <button type="button" data-custom-theme-action="download" disabled>Download</button>
            </div>
          </section>
        </section>
        <section id="settings-behavior" class="settings-panel" data-panel="behavior" role="tabpanel" aria-labelledby="settings-page-behavior" aria-hidden="true" hidden>
          <div class="settings-panel__intro"><h2>Behavior</h2><p>Control how the overlay stays visible.</p></div>
          <label class="settings-toggle"><input name="alwaysOnTop" type="checkbox" ${config.alwaysOnTop ? "checked" : ""} /><span>Always on top</span></label>
        </section>
      </div>
    </div>`;

  const scale = root.querySelector<HTMLInputElement>("input[name=scale]")!;
  const scaleValue = root.querySelector<HTMLOutputElement>("[data-scale-value]")!;
  const opacity = root.querySelector<HTMLInputElement>("input[name=cardOpacity]")!;
  const opacityValue = root.querySelector<HTMLOutputElement>("[data-opacity-value]")!;
  const color = root.querySelector<HTMLInputElement>("input[name=backgroundColor]")!;
  const colorValue = root.querySelector<HTMLOutputElement>("[data-color-value]")!;
  const alwaysOnTop = root.querySelector<HTMLInputElement>("input[name=alwaysOnTop]")!;

  let current = { ...config };
  const commit = (patch: Partial<Config>) => {
    current = { ...current, ...patch };
    actions.onChange(current);
  };

  const monitorOptions = monitors.length
    ? monitors.map((monitor) => ({ value: monitor.id, label: monitor.label }))
    : [{ value: "", label: "Automatic screen" }];
  root.querySelector<HTMLElement>('[data-select-mount="monitorId"]')!.replaceWith(createCustomSelect("monitorId", "Monitor", config.monitorId ?? monitorOptions[0].value, monitorOptions, (value) => commit({ monitorId: value || null })));
  root.querySelector<HTMLElement>('[data-select-mount="corner"]')!.replaceWith(createCustomSelect("corner", "Corner", config.corner, [
    { value: "top-left", label: "Top left" },
    { value: "top-right", label: "Top right" },
    { value: "bottom-left", label: "Bottom left" },
    { value: "bottom-right", label: "Bottom right" },
  ], (value) => commit({ corner: value })));
  root.querySelector<HTMLElement>('[data-select-mount="layout"]')!.replaceWith(createCustomSelect("layout", "Layout", config.layout, [
    { value: "stacked-compact", label: "Vertical" },
    { value: "provider-columns", label: "Horizontal" },
  ], (value) => commit({ layout: value as Config["layout"] })));

  const activatePage = (page: string) => {
    root.querySelectorAll<HTMLButtonElement>('[role="tab"]').forEach((button) => {
      const selected = button.dataset.page === page;
      button.setAttribute("aria-selected", String(selected));
      button.tabIndex = selected ? 0 : -1;
    });
    root.querySelectorAll<HTMLElement>("[data-panel]").forEach((panel) => {
      const active = panel.dataset.panel === page;
      panel.hidden = !active;
      panel.setAttribute("aria-hidden", String(!active));
    });
  };
  const syncThemeControls = () => {
    root.querySelectorAll<HTMLButtonElement>("[data-theme]").forEach((button) => button.setAttribute("aria-pressed", String(button.dataset.theme === current.theme)));
    opacity.value = String(Math.round(current.cardOpacity * 100));
    opacityValue.value = `${opacity.value}%`;
    color.value = current.backgroundColor;
    colorValue.value = current.backgroundColor.toUpperCase();
  };

  scale.addEventListener("input", () => {
    scaleValue.value = `${scale.value}%`;
    commit({ scale: Number(scale.value) / 100 });
  });
  opacity.addEventListener("input", () => {
    opacityValue.value = `${opacity.value}%`;
    commit({ cardOpacity: Number(opacity.value) / 100 });
    syncThemeControls();
  });
  color.addEventListener("input", () => {
    colorValue.value = color.value.toUpperCase();
    commit({ backgroundColor: color.value });
    syncThemeControls();
  });
  alwaysOnTop.addEventListener("change", () => commit({ alwaysOnTop: alwaysOnTop.checked }));

  const tabs = Array.from(root.querySelectorAll<HTMLButtonElement>('[role="tab"]'));
  tabs.forEach((button, index) => {
    button.addEventListener("click", () => activatePage(button.dataset.page ?? "general"));
    button.addEventListener("keydown", (event) => {
      if (!["ArrowDown", "ArrowRight", "ArrowUp", "ArrowLeft"].includes(event.key)) return;
      event.preventDefault();
      const direction = event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : -1;
      const next = tabs[(index + direction + tabs.length) % tabs.length];
      next.focus();
      next.click();
    });
  });
  root.querySelectorAll<HTMLButtonElement>("[data-theme]").forEach((button) => button.addEventListener("click", () => {
    const theme = button.dataset.theme as ThemePreset;
    commit({ theme });
    syncThemeControls();
  }));
  root.querySelector<HTMLButtonElement>("[data-close]")!.addEventListener("click", actions.onClose);
  root.querySelector<HTMLElement>("[data-drag-handle]")!.addEventListener("mousedown", (event) => {
    if (event.button === 0 && !(event.target as Element).closest("button, input, [role=option]")) actions.onDrag?.();
  });
  return root;
}
