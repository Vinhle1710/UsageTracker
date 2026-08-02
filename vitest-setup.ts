import { expect } from "vitest";
import * as matchers from "vitest-axe/matchers";
import "vitest-axe/extend-expect";

expect.extend(matchers);

if (typeof HTMLCanvasElement !== "undefined") {
  HTMLCanvasElement.prototype.getContext = (() => ({ measureText: () => ({ width: 0 }) })) as unknown as typeof HTMLCanvasElement.prototype.getContext;
}
