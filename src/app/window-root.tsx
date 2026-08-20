export type WindowRoot = "overlay" | "settings" | "edge-tab";

export function rootForWindow(label: string): WindowRoot {
  if (label === "main") return "overlay";
  if (label === "settings") return "settings";
  if (label === "edge-tab") return "edge-tab";
  throw new Error(`Unsupported window label: ${label}`);
}
