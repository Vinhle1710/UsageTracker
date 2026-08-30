import { useEffect, useRef, useState } from "react";
import { chooseHistoryExportPath, clearHistory, exportHistory } from "./api";
import { trapFocus } from "../focus-trap";
import type { HistoryQuery } from "./types";

const defaultQuery: HistoryQuery = { from: 0, to: Math.floor(Date.now() / 1000) + 1, provider: null, windowKind: null };

export function ExportControls({ query = defaultQuery, onCleared }: { query?: HistoryQuery; onCleared?: () => void }) {
  const [format, setFormat] = useState<"json" | "csv">("json");
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const clearRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  const wasConfirming = useRef(false);

  useEffect(() => {
    if (confirm) {
      wasConfirming.current = true;
      confirmRef.current?.focus();
    } else if (wasConfirming.current) {
      wasConfirming.current = false;
      clearRef.current?.focus();
    }
  }, [confirm]);
  useEffect(() => {
    if (!confirm) return;
    const close = (event: KeyboardEvent) => { if (event.key === "Escape") setConfirm(false); };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [confirm]);
  // Without this, Tab walks straight out of the confirmation and into the page behind it,
  // which is still fully interactive. Focus restoration is handled by the effect above, so
  // the trap is released without moving focus itself.
  useEffect(() => {
    if (!confirm || !dialogRef.current) return;
    const release = trapFocus(dialogRef.current, null);
    return () => { release(); };
  }, [confirm]);

  const doExport = async () => {
    setBusy(true); setMessage("");
    try {
      const exportHandle = await chooseHistoryExportPath(format);
      if (!exportHandle) { setMessage("Export cancelled."); return; }
      await exportHistory(query, exportHandle);
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

  return <section className="history-utility surface-motion-item" aria-labelledby="export-title">
    <div className="history-utility__intro"><h2 id="export-title">Export</h2><p>Save a copy of this data, or remove what is stored on this device.</p></div>
    <div className="history-utility__actions">
      <label>Format<select className="surface-control" value={format} onChange={(event) => setFormat(event.target.value as "json" | "csv")}><option value="json">JSON</option><option value="csv">CSV</option></select></label>
      <button className="history-action history-action--primary surface-control" type="button" onClick={() => void doExport()} disabled={busy}>Export history</button>
      <button ref={clearRef} className="history-action history-action--danger surface-control" type="button" onClick={() => setConfirm(true)} disabled={busy}>Clear history</button>
    </div>
    {(busy || message) && <p className="history-utility__status" role="status">{busy ? "Working…" : message}</p>}
    {confirm && <div className="history-modal-backdrop"><div ref={dialogRef} className="history-modal" role="dialog" aria-modal="true" aria-labelledby="clear-title" aria-describedby="clear-detail"><span className="history-modal__signal" aria-hidden="true">!</span><h3 id="clear-title">Clear local history?</h3><p id="clear-detail">This permanently removes every stored usage and billing sample from this device. It cannot be undone.</p><div><button className="history-action surface-control" type="button" onClick={() => setConfirm(false)} disabled={busy}>Cancel</button><button ref={confirmRef} className="history-action history-action--danger surface-control" type="button" onClick={() => void doClear()} disabled={busy}>Confirm clear</button></div></div></div>}
  </section>;
}
