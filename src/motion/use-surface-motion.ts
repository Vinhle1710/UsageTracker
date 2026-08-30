import { useEffect, type RefObject } from "react";
import { enhanceSurface } from "./surface-motion";

export function useSurfaceMotion(
  root: RefObject<HTMLElement | null>,
  revision: string,
  enhance: typeof enhanceSurface = enhanceSurface,
): void {
  useEffect(() => {
    if (!root.current) return;
    return enhance(root.current);
  }, [root, revision, enhance]);
}
