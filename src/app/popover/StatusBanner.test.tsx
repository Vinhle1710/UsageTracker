import { describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach } from "vitest";
import { StatusBanner, statusBannerState } from "./StatusBanner";
import { createI18n } from "../../i18n/i18n";
describe("status banner", () => {
 afterEach(cleanup);
 it.each([["refreshing","Refreshing…"],["error","Could not refresh usage"],["stale","Usage data may be outdated"]] as const)("renders %s", (state, text) => { render(<StatusBanner state={state} t={createI18n("en").t} />); expect(screen.getByRole("status").textContent).toContain(text); });
 it("uses refreshing over an old stale snapshot", () => expect(statusBannerState({ refreshing: true, state: "stale" })).toBe("refreshing"));
});
