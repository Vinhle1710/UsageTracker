import { afterEach, describe, expect, it } from "vitest";
import { trapFocus } from "./focus-trap";

function dialogWith(...labels: string[]): HTMLElement {
  const dialog = document.createElement("div");
  for (const label of labels) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = label;
    dialog.appendChild(button);
  }
  document.body.appendChild(dialog);
  return dialog;
}
const buttonNamed = (root: HTMLElement, label: string) =>
  Array.from(root.querySelectorAll("button")).find((button) => button.textContent === label)!;
const tab = (target: HTMLElement, shiftKey = false) =>
  target.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", shiftKey, bubbles: true, cancelable: true }));

describe("trapFocus", () => {
  afterEach(() => { document.body.innerHTML = ""; });

  it("wraps Tab from the last control back to the first", () => {
    const dialog = dialogWith("Cancel", "Confirm");
    trapFocus(dialog);
    buttonNamed(dialog, "Confirm").focus();

    tab(buttonNamed(dialog, "Confirm"));

    expect(document.activeElement).toBe(buttonNamed(dialog, "Cancel"));
  });

  it("wraps Shift+Tab from the first control back to the last", () => {
    const dialog = dialogWith("Cancel", "Confirm");
    trapFocus(dialog);
    buttonNamed(dialog, "Cancel").focus();

    tab(buttonNamed(dialog, "Cancel"), true);

    expect(document.activeElement).toBe(buttonNamed(dialog, "Confirm"));
  });

  it("leaves Tab alone in the middle of the dialog", () => {
    const dialog = dialogWith("One", "Two", "Three");
    trapFocus(dialog);
    buttonNamed(dialog, "Two").focus();

    const notCancelled = tab(buttonNamed(dialog, "Two"));

    expect(notCancelled).toBe(true);
    expect(document.activeElement).toBe(buttonNamed(dialog, "Two"));
  });

  it("returns focus to whatever opened it once released", () => {
    const opener = document.createElement("button");
    document.body.appendChild(opener);
    opener.focus();
    const dialog = dialogWith("Confirm");
    const release = trapFocus(dialog);
    buttonNamed(dialog, "Confirm").focus();

    release();

    expect(document.activeElement).toBe(opener);
  });

  it("returns focus to an explicitly named element instead of the previous one", () => {
    const opener = document.createElement("button");
    const preferred = document.createElement("button");
    document.body.append(opener, preferred);
    opener.focus();
    const dialog = dialogWith("Confirm");

    trapFocus(dialog, preferred)();

    expect(document.activeElement).toBe(preferred);
  });

  it("stops trapping once released", () => {
    const dialog = dialogWith("Cancel", "Confirm");
    trapFocus(dialog)();
    buttonNamed(dialog, "Confirm").focus();

    const notCancelled = tab(buttonNamed(dialog, "Confirm"));

    expect(notCancelled).toBe(true);
  });
});
