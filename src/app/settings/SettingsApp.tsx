import type { Config } from "../../types";
import { initialAppSnapshot } from "../store";
import { AnthropicAccounts } from "../../components/settings/AnthropicAccounts";
import { ConsoleCostsDashboard } from "../../components/console/ConsoleCostsDashboard";
import type { ConsoleCostsDashboard as Dashboard } from "../../types";

const unavailable: Dashboard = { period: { startsAt: "", endsAt: "", timezone: "UTC" }, spend: { value: null, fetchedAt: 0, state: "unavailable", errorCode: "noCredential" }, prepaidBalance: { value: null, fetchedAt: 0, state: "unavailable", errorCode: "noCredential" }, daily: { value: null, fetchedAt: 0, state: "unavailable", errorCode: "noCredential" }, byApiKey: { value: null, fetchedAt: 0, state: "unavailable", errorCode: "noCredential" }, byModel: { value: null, fetchedAt: 0, state: "unavailable", errorCode: "noCredential" } };

export function SettingsApp({ config = initialAppSnapshot().config, onChange }: { config?: Config; onChange?: (config: Config) => void }) {
  const tray = config.showTrayIndicator ?? true;
  const overlay = config.showScreenOverlay ?? true;
  const lastSurface = tray !== overlay;
  return <main id="app" data-window="settings" className="settings-window"><h1>Settings</h1>
    <label><input aria-label="Show tray indicator" type="checkbox" checked={tray} disabled={lastSurface && tray} onChange={(e) => onChange?.({ ...config, showTrayIndicator: e.currentTarget.checked })} />Show tray indicator</label>
    <label><input aria-label="Show screen overlay" type="checkbox" checked={overlay} disabled={lastSurface && overlay} onChange={(e) => onChange?.({ ...config, showScreenOverlay: e.currentTarget.checked })} />Show screen overlay</label>
    <AnthropicAccounts /><ConsoleCostsDashboard dashboard={unavailable} />
  </main>;
}
