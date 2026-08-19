import type { ConsoleCostsDashboard } from "./types";
export interface ConsoleCostsState { accountId: string | null; snapshot: ConsoleCostsDashboard | null; loading: boolean; error: string | null; }
export const initialConsoleCostsState: ConsoleCostsState = { accountId: null, snapshot: null, loading: false, error: null };
export type ConsoleCostsAction = {type:"loadStarted";accountId:string}|{type:"snapshotReceived";accountId:string;snapshot:ConsoleCostsDashboard}|{type:"loadFailed";accountId:string;error:string}|{type:"accountChanged";accountId:string|null};
export function consoleCostsReducer(state: ConsoleCostsState, action: ConsoleCostsAction): ConsoleCostsState {
  if (action.type === "accountChanged") return { accountId: action.accountId, snapshot: null, loading: !!action.accountId, error: null };
  if (action.accountId !== state.accountId) return state;
  if (action.type === "loadStarted") return {...state, loading:true, error:null};
  if (action.type === "snapshotReceived") return {...state, snapshot:action.snapshot, loading:false, error:null};
  return {...state, loading:false, error:action.error};
}
