export type HistoryRange = "5h" | "24h" | "7d" | "30d";
export interface HistoryPoint { provider:string; windowKind:string; sampledAt:number; usedPercent:number; model:string|null; apiCalls:number|null; estimatedCostMicros:number|null; overageCostMicros:number|null }
export interface BillingEntry { provider:string; periodStart:number; periodEnd:number; amountMicros:number; currency:string; source:string }
export interface BillingAggregate { provider:string; currency:string; source:string; amountMicros:number }
export interface HistoryQuery { from:number; to:number; provider?:string|null; windowKind?:string|null }
export interface HistoryResult { points:HistoryPoint[]; billing:BillingEntry[] }
