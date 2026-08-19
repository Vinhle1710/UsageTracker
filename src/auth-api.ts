import { invoke } from "@tauri-apps/api/core";
import type { AccountSummary } from "./types";
export const listAnthropicAccounts = () => invoke<AccountSummary[]>("list_anthropic_accounts");
export const saveManualAnthropicCredential = (credential: string) => invoke<AccountSummary>("save_manual_anthropic_credential", { credential });
export const deleteAnthropicAccount = (accountId: string) => invoke<void>("delete_anthropic_account", { accountId });
export const startClaudeAiLogin = () => invoke<string>("start_claude_ai_login");
export const cancelClaudeAiLogin = () => invoke<void>("cancel_claude_ai_login");
