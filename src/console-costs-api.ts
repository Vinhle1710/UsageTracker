import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ConsoleCostsDashboard } from "./types";

export const CONSOLE_COSTS_EVENT = "console-costs-changed";
const decimal = /^(0|[1-9][0-9]*)$/;
function validSection(section: unknown): boolean {
  if (!section || typeof section !== "object") return false;
  const s = section as Record<string, unknown>;
  if (!["fresh", "stale", "unavailable", "error"].includes(String(s.state))) return false;
  if (typeof s.fetchedAt !== "number") return false;
  if (s.value === null) return true;
  if (!s.value || typeof s.value !== "object") return false;
  const values = Array.isArray(s.value) ? s.value : [s.value];
  return values.every((v) => { const r=v as Record<string,unknown>; const a=(r.amount ?? r) as Record<string,unknown>; return typeof a.minorUnits === "string" && decimal.test(a.minorUnits) && typeof a.currency === "string"; });
}
export function validateConsoleCosts(value: unknown): ConsoleCostsDashboard {
  if (!value || typeof value !== "object") throw new Error("Malformed Console costs response");
  const v=value as Record<string,unknown>;
  if (!v.period || !["spend","prepaidBalance","daily","byApiKey","byModel"].every((k)=>validSection(v[k]))) throw new Error("Malformed Console costs response");
  return value as ConsoleCostsDashboard;
}
export const getConsoleCosts = (accountId: string) => invoke<unknown>("get_console_costs", { accountId }).then(validateConsoleCosts);
export const refreshConsoleCosts = (accountId: string) => invoke<unknown>("refresh_console_costs", { accountId }).then(validateConsoleCosts);
export const listenConsoleCosts = (callback: (value: ConsoleCostsDashboard) => void): Promise<UnlistenFn> => listen<unknown>(CONSOLE_COSTS_EVENT, (event) => { try { callback(validateConsoleCosts(event.payload)); } catch { /* malformed IPC is ignored */ } });
