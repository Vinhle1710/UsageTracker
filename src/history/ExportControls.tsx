import { useEffect, useRef, useState } from "react";
import { chooseHistoryExportPath, clearHistory, exportHistory } from "./api";
import type { HistoryQuery } from "./types";

const defaultQuery: HistoryQuery = { from: 0, to: Math.floor(Date.now() / 1000) + 1, provider: null, windowKind: null };

export function ExportControls({ query = defaultQuery, onCleared }: { query?: HistoryQuery; onCleared?: () => void }) {
  const [format, setFormat] = useState<"json" | "csv">("json");
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const clearRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);

  useEffect(() => { (confirm ? confirmRef : clearRef).current?.focus(); }, [confirm]);
  useEffect(() => {
    if (!confirm) return;
    const close = (event: KeyboardEvent) => { if (event.key === "Escape") setConfirm(false); };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [confirm]);

  const doExport = async () => {
    setBusy(true); setMessage("");
    try {
      const path = await chooseHistoryExportPath(format);
      if (!path) { setMessage("Export cancelled."); return; }
      await exportHistory(query, format, path);
      setMessage("Export complete.");
    } catch (error) {
      setMessage(`Export failed: ${String(error)}`);
    } finally { setBusy(false); }
  };
  const doClear = async () => {
    setBusy(true);
    try {
      await clearHistory();
      setConfirm(false);
      setMessage("History cleared.");
      onCleared?.();
    } catch (error) {
      setMessage(`Could not clear history: ${String(error)}`);
    } finally { setBusy(false); }
  };

  return <section className="history-utility surface-motion-item" aria-label="History export and clearing">
    <div className="history-utility__intro"><span>05 / Archive</span><h2>Archive tools</h2><p>Take a portable snapshot or clear locally stored samples.</p></div>
    <div className="history-utility__actions">
      <label>Format<select className="surface-control" value={format} onChange={(event) => setFormat(event.target.value as "json" | "csv")}><option value="json">JSON</option><option value="csv">CSV</option></select></label>
      <button className="history-action history-action--primary surface-control" type="button" onClick={() => void doExport()} disabled={busy}>Export history <span aria-hidden="true">↗</span></button>
      <button ref={clearRef} className="history-action history-action--danger surface-control" type="button" onClick={() => setConfirm(true)} disabled={busy}>Clear history</button>
    </div>
    {(busy || message) && <p className="history-utility__status telemetry-value" role="status">{busy ? "Working…" : message}</p>}
    {confirm && <div className="history-modal-backdrop"><div className="history-modal" role="dialog" aria-modal="true" aria-labelledby="clear-title"><span className="history-modal__signal" aria-hidden="true">!</span><p className="history-modal__eyebrow">Destructive action</p><h3 id="clear-title">Clear local history?</h3><p>This permanently removes every stored usage and billing sample from this device.</p><div><button ref={confirmRef} className="history-action history-action--danger surface-control" type="button" onClick={() => void doClear()} disabled={busy}>Confirm clear</button><button className="history-action surface-control" type="button" onClick={() => setConfirm(false)} disabled={busy}>Cancel</button></div></div></div>}
  </section>;
}

