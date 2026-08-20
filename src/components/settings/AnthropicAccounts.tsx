import { useEffect, useState } from "react";
import type { AccountSummary } from "../../types";
import * as api from "../../auth-api";

export function AnthropicAccounts() {
  const [accounts, setAccounts] = useState<AccountSummary[]>([]);
  const [credential, setCredential] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("Loading accounts…");
  useEffect(() => { api.listAnthropicAccounts().then(setAccounts).catch(() => setStatus("Accounts unavailable")); }, []);
  async function save(e: React.FormEvent) { e.preventDefault(); if (!credential.trim() || busy) return; setBusy(true); setStatus("Saving securely…"); try { const account = await api.saveManualAnthropicCredential(credential); setAccounts((old) => [...old.filter((x) => x.id !== account.id), account]); setStatus("Credential saved (unverified)"); } catch { setStatus("Could not save credential"); } finally { setCredential(""); setBusy(false); } }
  return <section aria-labelledby="anthropic-accounts-title"><h2 id="anthropic-accounts-title">Anthropic accounts</h2><p>Google sign-in stays on Anthropic’s hosted login page.</p><div aria-live="polite">{status}</div><ul>{accounts.map((a) => <li key={a.id}>{a.email ?? a.kind} — {a.status}{a.credentialHint ? ` ••••${a.credentialHint}` : ""}<button type="button" onClick={() => api.deleteAnthropicAccount(a.id).then(() => setAccounts((old) => old.filter((x) => x.id !== a.id)))}>Delete</button></li>)}</ul><button type="button" disabled={busy} onClick={() => api.startClaudeAiLogin().catch(() => setStatus("Sign-in unavailable"))}>Sign in to Claude.ai</button><form onSubmit={save}><label htmlFor="anthropic-credential">Add Console credential</label><input id="anthropic-credential" type="password" value={credential} onChange={(e) => setCredential(e.target.value)} autoComplete="off"/><button type="submit" disabled={busy || !credential.trim()}>Save credential</button></form></section>;
}
