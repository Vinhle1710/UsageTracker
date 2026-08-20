import { describe, expect, it } from "vitest";
import { rootForWindow } from "./window-root";

describe("rootForWindow", () => {
  it.each([["main", "overlay"], ["settings", "settings"], ["edge-tab", "edge-tab"]] as const)("routes %s", (label, expected) => {
    expect(rootForWindow(label)).toBe(expected);
  });
  it("rejects an unknown label", () => expect(() => rootForWindow("other")).toThrow("Unsupported window label: other"));
});
