import type { Config } from "../../types";
import { initialAppSnapshot } from "../store";
import { DisplaySettings } from "./DisplaySettings";
import { createI18n } from "../../i18n/i18n";

export function SettingsApp({ config = initialAppSnapshot().config, onChange }: { config?: Config; onChange?: (config: Config) => void }) {
  const tray = config.showTrayIndicator ?? true;
  const overlay = config.showScreenOverlay ?? true;
  const lastSurface = tray !== overlay;
  const { t } = createI18n(config.locale ?? "en");
  return <main id="app" data-window="settings" className="settings-window"><h1>{t("action.settings")}</h1>
    <label><input aria-label={t("action.settings")} type="checkbox" checked={tray} disabled={lastSurface && tray} onChange={(e) => onChange?.({ ...config, showTrayIndicator: e.currentTarget.checked })} />{t("popover.title")}</label>
    <label><input aria-label={t("overlay.show")} type="checkbox" checked={overlay} disabled={lastSurface && overlay} onChange={(e) => onChange?.({ ...config, showScreenOverlay: e.currentTarget.checked })} />{t("overlay.show")}</label>
    <DisplaySettings value={config} onChange={(next) => onChange?.(next)} />
  </main>;
}
