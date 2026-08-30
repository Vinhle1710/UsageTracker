import type {HistoryRange} from "./types";
const seconds={"5h":18000,"24h":86400,"7d":604800,"30d":2592000} as const;
export const historyBounds=(range:HistoryRange,now:number)=>({from:now-seconds[range],to:now+1});
