import { expect, it, vi } from "vitest";
import { createAppStore } from "./store";

it("publishes immutable bootstrap and usage snapshots", () => {
  const store = createAppStore();
  const listener = vi.fn();
  store.subscribe(listener);
  store.dispatch({ type: "bootstrap", payload: { sources: { claude: true, openai: false }, usage: [] } });
  const first = store.getSnapshot();
  store.dispatch({ type: "usage", payload: { provider: "claude", snapshot: { windows: [], fetched_at: 1, state: "fresh" } } });
  expect(store.getSnapshot()).not.toBe(first);
  expect(listener).toHaveBeenCalledTimes(2);
});
