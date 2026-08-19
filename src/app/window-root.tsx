export type WindowRoot = "overlay" | "settings" | "edge-tab" | "popover";

export function rootForWindow(label: string): WindowRoot {
  if (label === "main") return "overlay";
  if (label === "settings") return "settings";
  if (label === "edge-tab") return "edge-tab";
  if (label === "popover") return "popover";
  throw new Error(`Unsupported window label: ${label}`);
}
