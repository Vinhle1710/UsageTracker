import { useRef } from "react";
import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useSurfaceMotion } from "./use-surface-motion";

describe("useSurfaceMotion", () => {
  it("rebinds on a visual revision and cleans up every enhancement", () => {
    const cleanups = [vi.fn(), vi.fn()];
    const enhance = vi.fn().mockReturnValueOnce(cleanups[0]).mockReturnValueOnce(cleanups[1]);
    function Harness({ revision }: { revision: string }) {
      const root = useRef<HTMLDivElement>(null);
      useSurfaceMotion(root, revision, enhance);
      return <div ref={root}><div data-smooth-scroll/></div>;
    }
    const view = render(<Harness revision="loading"/>);
    expect(enhance).toHaveBeenCalledOnce();
    view.rerender(<Harness revision="ready"/>);
    expect(cleanups[0]).toHaveBeenCalledOnce();
    expect(enhance).toHaveBeenCalledTimes(2);
    view.unmount();
    expect(cleanups[1]).toHaveBeenCalledOnce();
  });
});
