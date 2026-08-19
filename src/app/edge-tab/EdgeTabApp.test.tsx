import { render, screen } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { EdgeTabApp } from "./EdgeTabApp";

it("labels the action and reverses the arrow", () => {
  const { rerender } = render(<EdgeTabApp side="right" hidden={false} reducedMotion={false} onToggle={vi.fn()} />);
  expect(screen.getByRole("button", { name: "Hide usage overlay" })).toHaveAttribute("data-direction", "right");
  rerender(<EdgeTabApp side="right" hidden reducedMotion={false} onToggle={vi.fn()} />);
  expect(screen.getByRole("button", { name: "Show usage overlay" })).toHaveAttribute("data-direction", "left");
});
