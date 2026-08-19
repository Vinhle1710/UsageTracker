import { expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { PopoverApp } from "./PopoverApp";
import { rootForWindow } from "../window-root";
it("routes popover and renders current indicators", () => {
 expect(rootForWindow("popover")).toBe("popover");
 render(<PopoverApp snapshot={undefined} detached={false} />);
 expect(screen.getByRole("dialog", { name: "Usage" })).toBeTruthy();
});
