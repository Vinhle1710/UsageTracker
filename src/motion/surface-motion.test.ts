import { describe, expect, it, vi } from "vitest";
import { enhanceSurface, type SurfaceMotionAdapters } from "./surface-motion";

function fixture() {
  const root = document.createElement("main");
  root.innerHTML = `
    <div data-smooth-scroll></div>
    <button role="tab" data-nav-index="0" aria-controls="first" aria-selected="true">First</button>
    <button role="tab" data-nav-index="2" aria-controls="second" aria-selected="false">Second</button>
    <section id="first"></section><section id="second"></section>`;
  return root;
}

function adapters() {
  const scroller = { raf: vi.fn(), destroy: vi.fn() };
  const value: SurfaceMotionAdapters = {
    createScroller: vi.fn(() => scroller),
    addTicker: vi.fn(),
    removeTicker: vi.fn(),
    animateEntrance: vi.fn(() => vi.fn()),
    animatePanel: vi.fn(),
  };
  return { value, scroller };
}

describe("enhanceSurface", () => {
  it("owns one scoped scroller and releases every resource", () => {
    const root = fixture();
    const { value, scroller } = adapters();
    const cleanup = enhanceSurface(root, { adapters: value });

    expect(value.createScroller).toHaveBeenCalledWith(root.querySelector("[data-smooth-scroll]"));
    expect(value.addTicker).toHaveBeenCalledOnce();
    expect(value.animateEntrance).toHaveBeenCalledWith(root);

    cleanup();
    expect(scroller.destroy).toHaveBeenCalledOnce();
    expect(value.removeTicker).toHaveBeenCalledWith(vi.mocked(value.addTicker).mock.calls[0][0]);
    expect(vi.mocked(value.animateEntrance).mock.results[0].value).toHaveBeenCalledOnce();
  });

  it("animates tab panels in navigation order", () => {
    const root = fixture();
    const { value } = adapters();
    const cleanup = enhanceSurface(root, { adapters: value });

    root.querySelector<HTMLButtonElement>('[aria-controls="second"]')!.click();
    expect(value.animatePanel).toHaveBeenCalledWith(root.querySelector("#second"), 1);

    cleanup();
    root.querySelector<HTMLButtonElement>('[aria-controls="first"]')!.click();
    expect(value.animatePanel).toHaveBeenCalledTimes(1);
  });
});
