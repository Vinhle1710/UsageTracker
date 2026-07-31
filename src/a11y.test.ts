import { axe } from "vitest-axe";
import { afterEach, describe, expect, it } from "vitest";
import { renderLayer } from "./components/layer";
import type { UsageSnapshot } from "./types";

const snap: UsageSnapshot = {
  windows: [
    { label: "5 hour", used_percent: 12, resets_at: 1_003_600 },
    { label: "Weekly", used_percent: 48, resets_at: 1_259_200 },
  ],
  fetched_at: 999_940,
  state: "fresh",
};

describe("accessibility", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  for (const state of ["fresh", "stale", "error"] as const) {
    it(`has no violations in the ${state} state`, async () => {
      const host = document.createElement("main");
      host.appendChild(renderLayer("Claude", { ...snap, state }, 1_000_000));
      document.body.appendChild(host);
      expect((await axe(host)).violations).toEqual([]);
    });
  }
  it("has no violations when a provider reports no windows", async () => {
    const host = document.createElement("main");
    host.appendChild(renderLayer("Codex", { ...snap, windows: [] }, 1_000_000));
    document.body.appendChild(host);
    expect((await axe(host)).violations).toEqual([]);
  });
});
