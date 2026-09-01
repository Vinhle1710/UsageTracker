import type {
  ClaudeAccountInfo,
  Config,
  MeterShape,
  MonitorOption,
  ThemePreset,
} from "../types";
import { trapFocus } from "../focus-trap";
import { formatMicros } from "../format";
import { minimalSupportsMeterShape } from "../state";

export interface SettingsActions {
  onChange: (config: Config) => void;
  onClose: () => void;
  onDrag?: () => void;
  onRefreshNow?: () => void | Promise<void>;
  onHistory?: () => void;
  /** Stores the claude.ai browser session key, which the extra-credit endpoints require. */
  onSaveSessionKey?: (sessionKey: string) => Promise<void>;
  onClearSessionKey?: () => Promise<void>;
  /** Stores the platform.claude.com session cookie that the Console cost endpoints require.
   *  A different credential from the claude.ai key above: different host, different sign-in. */
  onSaveConsoleSessionKey?: (sessionKey: string) => Promise<void>;
  onClearConsoleSessionKey?: () => Promise<void>;
}

interface SelectOption {
  value: string;
  label: string;
}

let selectSequence = 0;

function createCustomSelect(
  name: string,
  label: string,
  value: string,
  options: SelectOption[],
  onChange: (value: string) => void,
): HTMLElement {
  const id = `settings-select-${++selectSequence}`;
  const field = document.createElement("div");
  field.className = "settings-field settings-control-card surface-motion-item";
  field.dataset.select = name;
  const fieldLabel = document.createElement("span");
  fieldLabel.className = "settings-field__label";
  fieldLabel.id = `${id}-label`;
  fieldLabel.textContent = label;
  const customSelect = document.createElement("div");
  customSelect.className = "custom-select";
  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.className = "custom-select__trigger surface-control";
  trigger.setAttribute("role", "combobox");
  trigger.setAttribute("aria-haspopup", "listbox");
  trigger.setAttribute("aria-expanded", "false");
  trigger.setAttribute("aria-controls", `${id}-list`);
  trigger.setAttribute("aria-labelledby", `${id}-label ${id}-value`);
  const valueLabel = document.createElement("span");
  valueLabel.id = `${id}-value`;
  valueLabel.dataset.selectValue = "";
  const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  icon.setAttribute("viewBox", "0 0 16 16");
  icon.setAttribute("aria-hidden", "true");
  const iconPath = document.createElementNS("http://www.w3.org/2000/svg", "path");
  iconPath.setAttribute("d", "m4 6 4 4 4-4");
  icon.appendChild(iconPath);
  trigger.append(valueLabel, icon);
  const list = document.createElement("ul");
  list.id = `${id}-list`;
  list.className = "custom-select__list";
  list.setAttribute("role", "listbox");
  list.setAttribute("aria-labelledby", `${id}-label`);
  list.hidden = true;
  customSelect.append(trigger, list);
  field.append(fieldLabel, customSelect);
  let current = options.some((option) => option.value === value)
    ? value
    : (options[0]?.value ?? "");

  const close = (restoreFocus = false) => {
    trigger.setAttribute("aria-expanded", "false");
    list.hidden = true;
    if (restoreFocus) trigger.focus();
  };
  const focusOption = (index: number) => {
    const items = Array.from(
      list.querySelectorAll<HTMLElement>("[role=option]"),
    );
    items[(index + items.length) % items.length]?.focus();
  };
  const open = (focusIndex?: number) => {
    trigger.setAttribute("aria-expanded", "true");
    list.hidden = false;
    const items = Array.from(
      list.querySelectorAll<HTMLElement>("[role=option]"),
    );
    const selectedIndex = Math.max(
      0,
      items.findIndex((item) => item.dataset.value === current),
    );
    queueMicrotask(() => focusOption(focusIndex ?? selectedIndex));
  };
  const select = (next: string) => {
    current = next;
    const selected = options.find((option) => option.value === current);
    valueLabel.textContent = selected?.label ?? current;
    list
      .querySelectorAll<HTMLElement>("[role=option]")
      .forEach((item) =>
        item.setAttribute(
          "aria-selected",
          String(item.dataset.value === current),
        ),
      );
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
  valueLabel.textContent =
    options.find((option) => option.value === current)?.label ?? current;

  trigger.addEventListener("click", () =>
    trigger.getAttribute("aria-expanded") === "true" ? close() : open(),
  );
  trigger.addEventListener("keydown", (event) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const last = Math.max(0, options.length - 1);
    open(event.key === "ArrowUp" || event.key === "End" ? last : 0);
  });
  list.addEventListener("keydown", (event) => {
    const items = Array.from(
      list.querySelectorAll<HTMLElement>("[role=option]"),
    );
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

function shortOrgLabel(organizationUuid: string | null): string {
  return organizationUuid
    ? `Connected through Claude Code · org ${organizationUuid.slice(0, 8)}…`
    : "Connected through Claude Code";
}

function renderClaudeAccountSection(
  container: HTMLElement,
  account: ClaudeAccountInfo | null,
): void {
  const status = document.createElement("p");
  status.className = "claude-account__status";
  status.textContent = account
    ? shortOrgLabel(account.organizationUuid)
    : "Claude Code is not signed in";
  const description = document.createElement("p");
  description.className = "claude-account__description";
  description.textContent = account
    ? "Usage Tracker only reads this CLI-owned credential. Manage this session in Claude Code."
    : "Sign in from Claude Code, then reopen settings. Usage Tracker only reads the credential the CLI owns.";
  container.replaceChildren(status, description);
}

export function setRuntimeHealthMessage(row: Element | null, message: string): void {
  const text = row?.querySelector<HTMLElement>("span");
  if (text) text.textContent = message;
}

export function renderSettings(
  config: Config,
  monitors: MonitorOption[],
  actions: SettingsActions,
  claudeAccount: ClaudeAccountInfo | null = null,
  initialPage = "general",
  hasSessionKey = false,
  hasConsoleSessionKey = false,
): HTMLElement {
  const meterShape: MeterShape = config.meterShape ?? "ring";
  // Solid means solid. The stored value is left alone; only what the slider shows is pinned,
  // and the commit path below writes the 1.0 back so config and UI never disagree.
  const openingOpacity = config.theme === "solid" ? 1 : config.cardOpacity;
  const root = document.createElement("main");
  root.className = "settings-window";
  root.setAttribute("aria-labelledby", "settings-title");
  root.innerHTML = `
    <header class="settings-window__header surface-motion-item" data-drag-handle data-tauri-drag-region>
      <h1 id="settings-title">Settings</h1>
      <div class="settings-window__actions"><button class="settings-window__close surface-control" type="button" data-close aria-label="Close settings">×</button></div>
    </header>
    <div class="settings-layout">
      <div class="settings-sidebar settings-rail surface-motion-item">
        <nav class="settings-nav" aria-label="Settings pages" role="tablist" aria-orientation="vertical">
          <button id="settings-page-general" type="button" role="tab" data-page="general" data-nav-index="0" aria-controls="settings-general" aria-selected="true">General</button>
          <button id="settings-page-display" type="button" role="tab" data-page="display" data-nav-index="1" aria-controls="settings-display" aria-selected="false" tabindex="-1">Display</button>
          <button id="settings-page-behavior" type="button" role="tab" data-page="behavior" data-nav-index="2" aria-controls="settings-behavior" aria-selected="false" tabindex="-1">Behavior</button>
          <button id="settings-page-account" type="button" role="tab" data-page="account" data-nav-index="3" aria-controls="settings-account" aria-selected="false" tabindex="-1">Account</button>
        </nav>
        <button id="settings-history" class="settings-history surface-control" type="button" aria-label="Open history">History<span class="settings-history__chevron" aria-hidden="true">›</span></button>
      </div>
      <div class="settings-pages" data-smooth-scroll>
        <section id="settings-general" class="settings-panel surface-motion-item" data-panel="general" role="tabpanel" aria-labelledby="settings-page-general" aria-hidden="false">
          <div class="settings-panel__intro"><h2>General</h2><p>Where the overlay sits on your screen.</p></div>
          <div data-select-mount="monitorId"></div>
          <div data-select-mount="corner"></div>
        </section>
        <section id="settings-display" class="settings-panel surface-motion-item" data-panel="display" role="tabpanel" aria-labelledby="settings-page-display" aria-hidden="true" hidden>
          <div class="settings-panel__intro"><h2>Display</h2><p>How the provider cards are arranged, sized, and coloured.</p></div>
          <div data-select-mount="layout"></div>
          <label class="settings-control-card surface-motion-item">Scale <output class="telemetry-value" data-scale-value>${Math.round(config.scale * 100)}%</output><input name="scale" type="range" min="75" max="150" step="5" value="${Math.round(config.scale * 100)}" /></label>
          <fieldset class="settings-group meter-shape-group" aria-labelledby="meter-shape-title">
            <legend id="meter-shape-title">Usage readout</legend>
            <div class="meter-shape-grid" role="group" aria-label="Usage readout shape">
              <button type="button" class="meter-shape-option" data-meter-shape="ring" aria-pressed="${meterShape === "ring"}"><span class="meter-shape-preview meter-shape-preview--ring" aria-hidden="true"></span><strong>Ring</strong></button>
              <button type="button" class="meter-shape-option" data-meter-shape="charge" aria-pressed="${meterShape === "charge"}"><span class="meter-shape-preview meter-shape-preview--charge" aria-hidden="true"></span><strong>Charge</strong></button>
              <button type="button" class="meter-shape-option" data-meter-shape="reactor" aria-pressed="${meterShape === "reactor"}"><span class="meter-shape-preview meter-shape-preview--reactor" aria-hidden="true"></span><strong>Arc Reactor</strong></button>
              <button type="button" class="meter-shape-option" data-meter-shape="columns" aria-pressed="${meterShape === "columns"}"><span class="meter-shape-preview meter-shape-preview--columns" aria-hidden="true"></span><strong>Columns</strong></button>
              <button type="button" class="meter-shape-option" data-meter-shape="line" aria-pressed="${meterShape === "line"}"><span class="meter-shape-preview meter-shape-preview--line" aria-hidden="true"></span><strong>Line</strong></button>
              <button type="button" class="meter-shape-option" data-meter-shape="semicircle" aria-pressed="${meterShape === "semicircle"}"><span class="meter-shape-preview meter-shape-preview--semicircle" aria-hidden="true"></span><strong>Semi Circle</strong></button>
            </div>
          </fieldset>
          <div class="settings-subhead"><h3>Theme</h3><p>How the cards blend with your desktop.</p></div>
          <div class="theme-grid theme-grid--single-column" role="group" aria-label="Theme presets">
            <button type="button" class="theme-option" data-theme="clear" aria-pressed="${config.theme === "clear"}"><span class="theme-preview theme-preview--clear" data-preview-theme="clear"><i></i><i></i></span><strong>Translucent gradient</strong></button>
            <button type="button" class="theme-option" data-theme="frosted" aria-pressed="${config.theme === "frosted"}"><span class="theme-preview theme-preview--frosted" data-preview-theme="frosted"><i></i><i></i></span><strong>Frosted</strong></button>
            <button type="button" class="theme-option" data-theme="solid" aria-pressed="${config.theme === "solid"}"><span class="theme-preview theme-preview--solid" data-preview-theme="solid"><i></i><i></i></span><strong>Solid</strong></button>
            <button type="button" class="theme-option" data-theme="neon" aria-pressed="${config.theme === "neon"}"><span class="theme-preview theme-preview--neon" data-preview-theme="neon"><i></i><i></i></span><strong>Neon</strong></button>
          </div>
          <label class="settings-color settings-control-card surface-motion-item">Background <span class="settings-color__control"><input name="backgroundColor" type="color" value="${config.backgroundColor}" aria-label="Choose card background color" /><output class="telemetry-value" data-color-value>${config.backgroundColor.toUpperCase()}</output></span></label>
          <label class="settings-control-card surface-motion-item">Card opacity <output class="telemetry-value" data-opacity-value>${Math.round(openingOpacity * 100)}%</output><input name="cardOpacity" type="range" min="70" max="100" step="1" value="${Math.round(openingOpacity * 100)}"${config.theme === "solid" ? " disabled" : ""} /><small class="settings-control-card__note" data-opacity-note${config.theme === "solid" ? "" : " hidden"}>Solid is always fully opaque.</small></label>
          <section class="custom-theme-tools" aria-labelledby="custom-theme-title">
            <div><h3 id="custom-theme-title">Custom themes</h3><span>Coming soon</span></div>
            <div class="custom-theme-tools__actions">
              <button type="button" data-custom-theme-action="create" disabled>Create</button>
              <button type="button" data-custom-theme-action="import" disabled>Import</button>
              <button type="button" data-custom-theme-action="download" disabled>Download</button>
            </div>
          </section>
        </section>
        <section id="settings-behavior" class="settings-panel surface-motion-item" data-panel="behavior" role="tabpanel" aria-labelledby="settings-page-behavior" aria-hidden="true" hidden>
          <div class="settings-panel__intro"><h2>Behavior</h2><p>When usage refreshes, and how the overlay stays visible.</p></div>
          <section class="settings-group" aria-labelledby="automation-title"><h3 id="automation-title">Automation</h3>
          <label class="settings-toggle"><input name="alwaysOnTop" type="checkbox" ${config.alwaysOnTop ? "checked" : ""} /><span><strong>Always on top</strong><small>Keep usage above other windows.</small></span></label>
          <label class="settings-toggle"><input name="launchAtStartup" type="checkbox" ${config.launchAtStartup ? "checked" : ""} /><span><strong>Launch at startup</strong><small>Start quietly when Windows signs in.</small></span></label>
          <label class="settings-toggle"><input name="autoInitializeSession" type="checkbox" ${config.autoInitializeSession ? "checked" : ""} /><span><strong>Initialize sessions automatically</strong><small>Start a provider session when needed.</small></span></label>
          <label class="settings-toggle"><input name="monitorNetwork" type="checkbox" ${config.monitorNetwork !== false ? "checked" : ""} /><span><strong>Monitor network availability</strong><small>Pause polling while offline.</small></span></label>
          <label class="settings-toggle"><input name="refreshOnWake" type="checkbox" ${config.refreshOnWake !== false ? "checked" : ""} /><span><strong>Refresh after wake</strong><small>Catch up immediately after sleep.</small></span></label>
          </section>
          <fieldset class="settings-group settings-shortcuts" aria-labelledby="shortcuts-title"><legend id="shortcuts-title">Global shortcuts</legend>
            <label>Popover <input name="shortcutPopover" type="text" value="${config.shortcutPopover ?? ""}" placeholder="Ctrl+Shift+U" /></label>
            <label>Refresh <input name="shortcutRefresh" type="text" value="${config.shortcutRefresh ?? ""}" placeholder="Ctrl+Shift+R" /></label>
            <label>Settings <input name="shortcutSettings" type="text" value="${config.shortcutSettings ?? ""}" placeholder="Ctrl+Shift+S" /></label>
            <p data-shortcut-error role="alert" hidden></p>
          </fieldset>
          <label class="settings-control-card">Refresh interval <span class="settings-control-card__value"><input name="pollIntervalSec" type="number" min="15" max="3600" value="${config.pollIntervalSec}" /><span class="settings-control-card__unit">seconds</span></span></label>
          <section class="runtime-health" aria-labelledby="runtime-title"><div class="runtime-health__head"><h3 id="runtime-title">Runtime health</h3><button class="surface-control" type="button" data-refresh-now>Refresh now</button></div>
          <p class="runtime-health__item" data-runtime-status aria-live="polite"><i aria-hidden="true"></i><span>Automatic polling follows your network connection.</span></p>
          <p class="runtime-health__item" data-startup-status><i aria-hidden="true"></i><span>Startup registration is checked by the backend.</span></p>
          <p class="runtime-health__item" data-auto-init-status><i aria-hidden="true"></i><span>${config.lastAutoInitAt ? "Automatic initialization is cooling down after its last attempt." : "No automatic initialization attempt recorded."}</span></p></section>
        </section>
        <section id="settings-account" class="settings-panel surface-motion-item" data-panel="account" role="tabpanel" aria-labelledby="settings-page-account" aria-hidden="true" hidden>
          <div class="settings-panel__intro"><h2>Account</h2><p>Connect a provider to read usage directly.</p></div>
          <div class="claude-account" data-claude-account></div>
          <section class="session-key claude-account" aria-labelledby="session-key-title">
            <h3 id="session-key-title">Claude.ai session key</h3>
            <p class="session-key__description">${hasSessionKey ? "Stored by Usage Tracker in secure storage. It reads usage without spending any of your quota, and unlocks the extra credit bar." : "Optionally connect claude.ai by storing its session key in Usage Tracker. Usage limits still work from Claude Code without it."}</p>
            <details class="session-key__manual">
              <summary>Enter it manually instead</summary>
              <ol class="session-key__steps">
                <li>Open claude.ai while signed in, then press <kbd>F12</kbd>.</li>
                <li>Go to <strong>Application</strong> &rarr; <strong>Cookies</strong> &rarr; <code>https://claude.ai</code>.</li>
                <li>Find the <code>sessionKey</code> row.</li>
                <li>Double-click its <strong>Value</strong>, copy, and paste it below.</li>
              </ol>
              <div class="session-key__row">
                <input class="session-key__input" data-session-key-input type="password" autocomplete="off" spellcheck="false" placeholder="sessionKey cookie value" aria-label="Claude.ai session key" />
                <button type="button" class="claude-account__action" data-session-key-save>Save</button>
              </div>
            </details>
            <p class="session-key__caution">Treat it like a password: it grants access to your Claude account until you sign out of that browser. It is stored in Windows Credential Manager and is only ever sent to claude.ai.</p>
            ${hasSessionKey ? '<button type="button" class="settings-secondary-action surface-control" data-session-key-clear>Disconnect claude.ai</button>' : ""}
            <p class="session-key__status" role="status" data-session-key-status>${hasSessionKey ? "Connected — extra credit is being read." : "Not connected."}</p>
          </section>
          <section class="console-keys claude-account" aria-labelledby="console-keys-title">
            <h3 id="console-keys-title">Anthropic Console costs</h3>
            <p class="session-key__description">Optional. Shows what you have spent on the pay-as-you-go API this month. This is a <strong>separate</strong> sign-in from Claude.ai above — same cookie name, different site.</p>
            <details class="session-key__manual">
              <summary>How to get it</summary>
              <ol class="session-key__steps">
                <li>Open <code>platform.claude.com</code> while signed in, then press <kbd>F12</kbd>.</li>
                <li>Go to <strong>Application</strong> &rarr; <strong>Cookies</strong> &rarr; <code>https://platform.claude.com</code>.</li>
                <li>Find the <code>sessionKey</code> row.</li>
                <li>Double-click its <strong>Value</strong>, copy, and paste it below.</li>
              </ol>
              <div class="session-key__row">
                <input class="session-key__input" data-console-key-input type="password" autocomplete="off" spellcheck="false" placeholder="sessionKey cookie value" aria-label="Anthropic Console session key" />
                <button type="button" class="claude-account__action" data-console-key-save>Save</button>
              </div>
            </details>
            <p class="session-key__caution">Treat it like a password. Stored in Windows Credential Manager, only ever sent to platform.claude.com.</p>
            ${hasConsoleSessionKey ? '<button type="button" class="settings-secondary-action surface-control" data-console-key-clear>Disconnect Console</button>' : ""}
            <p class="session-key__status" role="status" data-console-keys-status>${hasConsoleSessionKey ? "Connected." : "Not connected."}</p>
            <div data-console-costs></div>
          </section>
        </section>
      </div>
    </div>`;

  const scale = root.querySelector<HTMLInputElement>("input[name=scale]")!;
  const scaleValue =
    root.querySelector<HTMLOutputElement>("[data-scale-value]")!;
  const opacity = root.querySelector<HTMLInputElement>(
    "input[name=cardOpacity]",
  )!;
  const opacityValue = root.querySelector<HTMLOutputElement>(
    "[data-opacity-value]",
  )!;
  const opacityNote = root.querySelector<HTMLElement>("[data-opacity-note]")!;
  const color = root.querySelector<HTMLInputElement>(
    "input[name=backgroundColor]",
  )!;
  const colorValue =
    root.querySelector<HTMLOutputElement>("[data-color-value]")!;
  const alwaysOnTop = root.querySelector<HTMLInputElement>(
    "input[name=alwaysOnTop]",
  )!;
  const launchAtStartup = root.querySelector<HTMLInputElement>(
    "input[name=launchAtStartup]",
  )!;
  const pollInterval = root.querySelector<HTMLInputElement>(
    "input[name=pollIntervalSec]",
  )!;
  const refreshOnWake = root.querySelector<HTMLInputElement>(
    "input[name=refreshOnWake]",
  )!;
  const autoInitialize = root.querySelector<HTMLInputElement>(
    "input[name=autoInitializeSession]",
  )!;
  const monitorNetwork = root.querySelector<HTMLInputElement>(
    "input[name=monitorNetwork]",
  )!;
  const shortcutError = root.querySelector<HTMLElement>(
    "[data-shortcut-error]",
  )!;

  // The filled portion of a range track cannot be expressed in CSS from the value alone,
  // so the current position is mirrored into a custom property on each input.
  const paintRange = (input: HTMLInputElement) => {
    const low = Number(input.min);
    const span = Number(input.max) - low;
    if (span > 0)
      input.style.setProperty(
        "--range-fill",
        `${((Number(input.value) - low) / span) * 100}%`,
      );
  };
  [scale, opacity].forEach(paintRange);

  let current = { ...config };
  const commit = (patch: Partial<Config>) => {
    current = { ...current, ...patch };
    actions.onChange(current);
  };

  const monitorOptions = monitors.length
    ? monitors.map((monitor) => ({ value: monitor.id, label: monitor.label }))
    : [{ value: "", label: "Automatic screen" }];
  root
    .querySelector<HTMLElement>('[data-select-mount="monitorId"]')!
    .replaceWith(
      createCustomSelect(
        "monitorId",
        "Monitor",
        config.monitorId ?? monitorOptions[0].value,
        monitorOptions,
        (value) => commit({ monitorId: value || null }),
      ),
    );
  root.querySelector<HTMLElement>('[data-select-mount="corner"]')!.replaceWith(
    createCustomSelect(
      "corner",
      "Corner",
      config.corner,
      [
        { value: "top-left", label: "Top left" },
        { value: "top-right", label: "Top right" },
        { value: "bottom-left", label: "Bottom left" },
        { value: "bottom-right", label: "Bottom right" },
      ],
      (value) => commit({ corner: value }),
    ),
  );
  root.querySelector<HTMLElement>('[data-select-mount="layout"]')!.replaceWith(
    createCustomSelect(
      "layout",
      "Layout",
      config.layout,
      [
        { value: "stacked-compact", label: "Vertical" },
        { value: "provider-columns", label: "Horizontal" },
        { value: "minimal", label: "Minimal" },
      ],
      (value) => {
        const layout = value as Config["layout"];
        const shape = current.meterShape ?? "ring";
        commit(layout === "minimal" && !minimalSupportsMeterShape(shape)
          ? { layout, meterShape: "ring" }
          : { layout });
        syncThemeControls();
      },
    ),
  );

  const activatePage = (page: string) => {
    root
      .querySelectorAll<HTMLButtonElement>('[role="tab"]')
      .forEach((button) => {
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
    root
      .querySelectorAll<HTMLButtonElement>("[data-theme]")
      .forEach((button) =>
        button.setAttribute(
          "aria-pressed",
          String(button.dataset.theme === current.theme),
        ),
      );
    // Mirrors the same rule Config::sanitized() enforces on the backend, so the slider never
    // shows a value the stored config would immediately reject.
    const locked = current.theme === "solid";
    opacity.disabled = locked;
    opacityNote.hidden = !locked;
    opacity.value = String(
      Math.round((locked ? 1 : current.cardOpacity) * 100),
    );
    opacityValue.value = `${opacity.value}%`;
    paintRange(opacity);
    color.value = current.backgroundColor;
    colorValue.value = current.backgroundColor.toUpperCase();
    root
      .querySelectorAll<HTMLButtonElement>("[data-meter-shape]")
      .forEach((button) => {
        const shape = button.dataset.meterShape as MeterShape;
        const incompatible = current.layout === "minimal" && !minimalSupportsMeterShape(shape);
        button.setAttribute(
          "aria-pressed",
          String(shape === (current.meterShape ?? "ring")),
        );
        button.disabled = incompatible;
        button.setAttribute("aria-disabled", String(incompatible));
        if (incompatible) button.dataset.incompatibleWithLayout = "true";
        else delete button.dataset.incompatibleWithLayout;
      });
  };

  syncThemeControls();

  scale.addEventListener("input", () => {
    scaleValue.value = `${scale.value}%`;
    paintRange(scale);
    commit({ scale: Number(scale.value) / 100 });
  });
  opacity.addEventListener("input", () => {
    opacityValue.value = `${opacity.value}%`;
    paintRange(opacity);
    commit({ cardOpacity: Number(opacity.value) / 100 });
    syncThemeControls();
  });
  color.addEventListener("input", () => {
    colorValue.value = color.value.toUpperCase();
    commit({ backgroundColor: color.value });
    syncThemeControls();
  });
  alwaysOnTop.addEventListener("change", () =>
    commit({ alwaysOnTop: alwaysOnTop.checked }),
  );
  launchAtStartup.addEventListener("change", () =>
    commit({ launchAtStartup: launchAtStartup.checked }),
  );
  pollInterval.addEventListener("change", () =>
    commit({
      pollIntervalSec: Math.max(
        15,
        Math.min(3600, Number(pollInterval.value) || 60),
      ),
    }),
  );
  refreshOnWake.addEventListener("change", () =>
    commit({ refreshOnWake: refreshOnWake.checked }),
  );
  monitorNetwork.addEventListener("change", () =>
    commit({ monitorNetwork: monitorNetwork.checked }),
  );
  const shortcutInputs = [
    [
      root.querySelector<HTMLInputElement>("input[name=shortcutPopover]")!,
      "shortcutPopover",
    ],
    [
      root.querySelector<HTMLInputElement>("input[name=shortcutRefresh]")!,
      "shortcutRefresh",
    ],
    [
      root.querySelector<HTMLInputElement>("input[name=shortcutSettings]")!,
      "shortcutSettings",
    ],
  ] as const;
  const saveShortcut = (
    input: HTMLInputElement,
    key: "shortcutPopover" | "shortcutRefresh" | "shortcutSettings",
  ) => {
    const value = input.value.trim();
    const all = shortcutInputs
      .map(([field]) =>
        field === input
          ? value.toLowerCase()
          : field.value.trim().toLowerCase(),
      )
      .filter(Boolean);
    if (all.length !== new Set(all).size) {
      shortcutError.hidden = false;
      shortcutError.textContent = `Shortcut conflict: ${value}`;
      return;
    }
    shortcutError.hidden = true;
    commit({ [key]: value || null });
  };
  shortcutInputs.forEach(([input, key]) =>
    input.addEventListener("change", () => saveShortcut(input, key)),
  );
  autoInitialize.addEventListener("change", () => {
    if (!autoInitialize.checked) {
      commit({ autoInitializeSession: false });
      return;
    }
    autoInitialize.checked = false;
    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    dialog.setAttribute("aria-modal", "true");
    dialog.setAttribute("aria-labelledby", "auto-init-title");
    dialog.className = "settings-modal";
    dialog.innerHTML = `<h2 id="auto-init-title">Automatic session initialization</h2><p>This can start a paid API/CLI session.</p><label><input type="checkbox" data-cost-ack /> I understand this can start a paid API/CLI session</label><div><button type="button" data-cancel>Cancel</button><button type="button" data-confirm disabled>Confirm</button></div>`;
    const acknowledgement =
      dialog.querySelector<HTMLInputElement>("[data-cost-ack]")!;
    const confirm = dialog.querySelector<HTMLButtonElement>("[data-confirm]")!;
    acknowledgement.addEventListener("change", () => {
      confirm.disabled = !acknowledgement.checked;
    });
    root.appendChild(dialog);
    // Tab stays inside the dialog while it is open, and every exit runs through
    // close(), so focus always lands back on the switch that opened it rather
    // than stranding on a node that has just been removed.
    const release = trapFocus(dialog, autoInitialize);
    const close = () => {
      release();
      dialog.remove();
    };
    dialog.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
    });
    dialog
      .querySelector<HTMLButtonElement>("[data-cancel]")!
      .addEventListener("click", close);
    confirm.addEventListener("click", () => {
      close();
      autoInitialize.checked = true;
      commit({
        autoInitializeSession: true,
        autoInitCostWarningAccepted: true,
      });
    });
    acknowledgement.focus();
  });
  root
    .querySelector<HTMLButtonElement>("[data-refresh-now]")!
    .addEventListener("click", () => actions.onRefreshNow?.());

  const tabs = Array.from(
    root.querySelectorAll<HTMLButtonElement>('[role="tab"]'),
  );
  tabs.forEach((button, index) => {
    button.addEventListener("click", () =>
      activatePage(button.dataset.page ?? "general"),
    );
    button.addEventListener("keydown", (event) => {
      if (
        !["ArrowDown", "ArrowRight", "ArrowUp", "ArrowLeft"].includes(event.key)
      )
        return;
      event.preventDefault();
      const direction =
        event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : -1;
      const next = tabs[(index + direction + tabs.length) % tabs.length];
      next.focus();
      next.click();
    });
  });
  root.querySelectorAll<HTMLButtonElement>("[data-theme]").forEach((button) =>
    button.addEventListener("click", () => {
      const theme = button.dataset.theme as ThemePreset;
      // Solid carries its opacity with it: committing the theme alone would leave a stale
      // translucent value in the config that the backend would then silently rewrite.
      commit(theme === "solid" ? { theme, cardOpacity: 1 } : { theme });
      syncThemeControls();
    }),
  );
  root
    .querySelectorAll<HTMLButtonElement>("[data-meter-shape]")
    .forEach((button) =>
      button.addEventListener("click", () => {
        commit({ meterShape: button.dataset.meterShape as MeterShape });
        syncThemeControls();
      }),
    );
  if (initialPage !== "general") activatePage(initialPage);
  root
    .querySelector<HTMLButtonElement>("[data-close]")!
    .addEventListener("click", actions.onClose);
  root
    .querySelector<HTMLButtonElement>("#settings-history")
    ?.addEventListener("click", () => actions.onHistory?.());
  root
    .querySelector<HTMLElement>("[data-drag-handle]")!
    .addEventListener("mousedown", (event) => {
      if (
        event.button === 0 &&
        !(event.target as Element).closest("button, input, [role=option]")
      )
        actions.onDrag?.();
    });
  renderClaudeAccountSection(
    root.querySelector<HTMLElement>("[data-claude-account]")!,
    claudeAccount,
  );

  const sessionKeyInput = root.querySelector<HTMLInputElement>(
    "[data-session-key-input]",
  )!;
  const sessionKeyStatus = root.querySelector<HTMLElement>(
    "[data-session-key-status]",
  )!;
  root
    .querySelector<HTMLButtonElement>("[data-session-key-save]")!
    .addEventListener("click", async () => {
      const value = sessionKeyInput.value.trim();
      if (!value) {
        sessionKeyStatus.textContent =
          "Paste the sessionKey cookie value first.";
        return;
      }
      try {
        await actions.onSaveSessionKey?.(value);
        // Cleared on success, never re-rendered: the field is a write-only door to the secure
        // store, so the key never sits in the DOM where a screenshot or devtools would show it.
        sessionKeyInput.value = "";
        sessionKeyStatus.textContent =
          "Connected — extra credit is being read.";
      } catch (error) {
        sessionKeyStatus.textContent =
          typeof error === "string" ? error : "That key was not accepted.";
      }
    });
  root
    .querySelector<HTMLButtonElement>("[data-session-key-clear]")
    ?.addEventListener("click", async () => {
      try {
        await actions.onClearSessionKey?.();
        sessionKeyStatus.textContent = "Not connected.";
      } catch (error) {
        sessionKeyStatus.textContent =
          typeof error === "string" ? error : "The stored credential could not be deleted.";
      }
    });

  const consoleInput = root.querySelector<HTMLInputElement>(
    "[data-console-key-input]",
  )!;
  const consoleStatus = root.querySelector<HTMLElement>(
    "[data-console-keys-status]",
  )!;
  root
    .querySelector<HTMLButtonElement>("[data-console-key-save]")!
    .addEventListener("click", async () => {
      const value = consoleInput.value.trim();
      if (!value) {
        consoleStatus.textContent = "Paste the sessionKey cookie value first.";
        return;
      }
      try {
        await actions.onSaveConsoleSessionKey?.(value);
        // Cleared only on success: a rejected paste stays put so it can be corrected.
        consoleInput.value = "";
        consoleStatus.textContent = "Connected.";
      } catch (error) {
        consoleStatus.textContent =
          typeof error === "string" ? error : "That key was not accepted.";
      }
    });
  root
    .querySelector<HTMLButtonElement>("[data-console-key-clear]")
    ?.addEventListener("click", async () => {
      try {
        await actions.onClearConsoleSessionKey?.();
        consoleStatus.textContent = "Not connected.";
      } catch (error) {
        consoleStatus.textContent =
          typeof error === "string" ? error : "The stored credential could not be deleted.";
      }
    });

  return root;
}

type CostSection<T> = { value: T | null; state: string; errorCode: string | null };
export interface ConsoleCostsView {
  period: { startsAt: string; endsAt: string; timezone: string };
  spend: CostSection<{ minorUnits: string; currency: string }>;
  prepaidBalance: CostSection<{ minorUnits: string; currency: string }>;
  daily: CostSection<Array<{ key: string; label: string; amount: { minorUnits: string; currency: string } }>>;
  byApiKey: CostSection<Array<{ key: string; label: string; amount: { minorUnits: string; currency: string } }>>;
  byModel: CostSection<Array<{ key: string; label: string; amount: { minorUnits: string; currency: string } }>>;
}

/** Why a section has no value. Never rendered as a zero amount — "no data" and "$0.00 spent"
 *  are different facts and conflating them is how a billing screen lies. */
function sectionNote(section: CostSection<unknown>): string | null {
  if (section.value !== null) return null;
  switch (section.errorCode) {
    case "noCredential": return "Connect a Console session key to see this.";
    case "insufficientRole": return "This Console account's role cannot read this.";
    case "unsupportedBySource": return "Not available from the Console API.";
    default: return "Temporarily unavailable.";
  }
}

function costTable(title: string, section: ConsoleCostsView["daily"]): HTMLElement {
  const wrap = document.createElement("section");
  wrap.className = "console-costs__group";
  const heading = document.createElement("h4");
  heading.textContent = title;
  wrap.appendChild(heading);
  const note = sectionNote(section);
  if (note || !section.value?.length) {
    const empty = document.createElement("p");
    empty.className = "console-keys__empty";
    empty.textContent = note ?? "No spend recorded this month.";
    wrap.appendChild(empty);
    return wrap;
  }
  const table = document.createElement("table");
  table.className = "console-costs__table";
  const caption = document.createElement("caption");
  caption.textContent = title;
  const head = document.createElement("thead");
  head.innerHTML = '<tr><th scope="col">Name</th><th scope="col">Amount</th></tr>';
  const body = document.createElement("tbody");
  // Sorted for display only; the opaque source key is what identifies the row.
  for (const point of [...section.value].sort((a, b) => Number(BigInt(b.amount.minorUnits) - BigInt(a.amount.minorUnits)))) {
    const row = document.createElement("tr");
    const name = document.createElement("th");
    name.scope = "row";
    name.textContent = point.label;
    const amount = document.createElement("td");
    amount.className = "telemetry-value";
    amount.textContent = formatMicros(Number(point.amount.minorUnits), point.amount.currency);
    row.append(name, amount);
    body.appendChild(row);
  }
  table.append(caption, head, body);
  wrap.appendChild(table);
  return wrap;
}

function totalCard(title: string, section: ConsoleCostsView["spend"]): HTMLElement {
  const card = document.createElement("article");
  card.className = "console-costs__total";
  const heading = document.createElement("h4");
  heading.textContent = title;
  const value = document.createElement("p");
  const note = sectionNote(section);
  if (note) {
    value.className = "console-keys__empty";
    value.textContent = note;
  } else {
    value.className = "telemetry-value console-costs__amount";
    value.textContent = formatMicros(Number(section.value!.minorUnits), section.value!.currency);
  }
  card.append(heading, value);
  return card;
}

/** Renders the dashboard into `container`, replacing whatever was there. */
export function renderConsoleCosts(container: HTMLElement, view: ConsoleCostsView): void {
  container.textContent = "";
  const root = document.createElement("div");
  root.className = "console-costs";
  const period = document.createElement("p");
  period.className = "console-costs__period";
  period.textContent = `${view.period.startsAt.slice(0, 10)} – ${view.period.endsAt.slice(0, 10)} (${view.period.timezone})`;
  const totals = document.createElement("div");
  totals.className = "console-costs__totals";
  totals.append(totalCard("This month", view.spend), totalCard("Prepaid credit", view.prepaidBalance));
  root.append(period, totals, costTable("By day", view.daily), costTable("By model", view.byModel), costTable("By API key", view.byApiKey));
  container.appendChild(root);
}
