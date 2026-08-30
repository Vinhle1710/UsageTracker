import { invoke } from "@tauri-apps/api/core";
import type { BillingAggregate, HistoryQuery, HistoryResult } from "./types";
export const queryHistory = (query: HistoryQuery) => invoke<HistoryResult>("query_history", { query });
export const queryBilling = (query: HistoryQuery) => invoke<BillingAggregate[]>("query_billing", { query });
export const clearHistory = () => invoke<void>("clear_history");
export const chooseHistoryExportPath = (format: "json" | "csv") => invoke<string | null>("choose_history_export_path", { format });
export const exportHistory = (query: HistoryQuery, exportHandle: string) => invoke<void>("export_history", { query, exportHandle });
