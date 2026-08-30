const FOCUSABLE = 'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** Keeps Tab inside a hand-rolled dialog and hands focus back where it came from.
 *
 *  A native `<dialog>` opened with `showModal()` already does both, so only the
 *  dialogs built out of plain elements need this — without it, Tab walks straight
 *  out of the dialog into the page behind, which is still fully interactive.
 *
 *  Returns the release function: call it when the dialog closes, however it closes
 *  (confirm, cancel, or Escape), so focus never strands on a removed node. */
export function trapFocus(container: HTMLElement, returnTo?: HTMLElement | null): () => void {
  const previous = returnTo ?? (document.activeElement as HTMLElement | null);
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "Tab") return;
    const items = Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE));
    if (!items.length) return;
    const edge = event.shiftKey ? items[0] : items[items.length - 1];
    if (document.activeElement !== edge) return;
    event.preventDefault();
    (event.shiftKey ? items[items.length - 1] : items[0]).focus();
  };
  container.addEventListener("keydown", onKeyDown);
  return () => {
    container.removeEventListener("keydown", onKeyDown);
    previous?.focus?.();
  };
}
