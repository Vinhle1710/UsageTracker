import { expect, it } from "vitest";
import indexHtml from "../index.html?raw";

it("keeps the controller-backed UI as the production entrypoint until the React migration is wired", () => {
  expect(indexHtml).toContain('src="/src/main.ts"');
});
