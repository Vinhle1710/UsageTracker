import { useSyncExternalStore } from "react";
import {
  createProviderState,
  updateProviderCollapsed,
  updateProviderSources,
  updateProviderUsage,
  visibleLayers,
} from "../state";
import type { ActiveSources, AppSnapshot, BootstrapPayload, Config, Provider, ProviderUsageEvent } from "../types";

const defaultConfig: Config = {
  monitorId: null, corner: "bottom-right", scale: 1, cardOpacity: .98, theme: "frosted", backgroundColor: "#07101f",
  layout: "stacked-compact", alwaysOnTop: true, offscreenPeek: false, launchAtStartup: true, pollIntervalSec: 60,
  detectIntervalSec: 5, showTrayIndicator: true, showScreenOverlay: true,
  valueMode: "used", indicatorStyle: "compact", enabledMetrics: ["session", "weekly", "api"], metricOrder: ["session", "weekly", "api"], colorMode: "multicolor", displayColors: {session:"#22c55e",weekly:"#f59e0b",api:"#60a5fa",single:"#60a5fa",background:"#07101f",text:"#f9fafb"}, adaptToSystemTheme: true, glowEnabled: false,
};

export type AppAction =
  | { type: "bootstrap"; payload: BootstrapPayload }
  | { type: "usage"; payload: ProviderUsageEvent }
  | { type: "sources"; payload: ActiveSources }
  | { type: "config"; payload: Config }
  | { type: "collapsed"; provider: Provider; collapsed: boolean };

export interface AppStore {
  getSnapshot(): AppSnapshot;
  subscribe(listener: () => void): () => void;
  dispatch(action: AppAction): void;
}

export function initialAppSnapshot(): AppSnapshot {
  const sources = { claude: false, openai: false };
  return { config: defaultConfig, sources, providers: createProviderState(sources), visibility: { enabled: true, providerAvailable: false, userHidden: false } };
}

function reduceAppSnapshot(snapshot: AppSnapshot, action: AppAction): AppSnapshot {
  switch (action.type) {
    case "bootstrap": {
      const sources = { ...action.payload.sources };
      let providers = updateProviderSources(snapshot.providers, sources);
      for (const usage of action.payload.usage) providers = updateProviderUsage(providers, usage);
      return { ...snapshot, sources, providers, visibility: { ...snapshot.visibility, providerAvailable: visibleLayers(sources).length > 0 } };
    }
    case "usage":
      return { ...snapshot, providers: updateProviderUsage(snapshot.providers, action.payload) };
    case "sources":
      return { ...snapshot, sources: { ...action.payload }, providers: updateProviderSources(snapshot.providers, action.payload), visibility: { ...snapshot.visibility, providerAvailable: visibleLayers(action.payload).length > 0 } };
    case "config":
      return { ...snapshot, config: { ...action.payload } };
    case "collapsed":
      return { ...snapshot, providers: updateProviderCollapsed(snapshot.providers, action.provider, action.collapsed) };
  }
}

export function createAppStore(initial = initialAppSnapshot()): AppStore {
  let snapshot = initial;
  const listeners = new Set<() => void>();
  return {
    getSnapshot: () => snapshot,
    subscribe: (listener) => { listeners.add(listener); return () => listeners.delete(listener); },
    dispatch(action) { snapshot = reduceAppSnapshot(snapshot, action); listeners.forEach((listener) => listener()); },
  };
}

export function useAppSnapshot(store: AppStore): AppSnapshot {
  return useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot);
}
