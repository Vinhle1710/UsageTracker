import { describe, expect, it } from "vitest";
import { GeometryRequestScheduler, type GeometryRequestCounts } from "./geometry-scheduler";

interface TestGeometryRequest extends GeometryRequestCounts {
  id: "A" | "B" | "C" | "empty";
}

function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve = (): void => undefined;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

describe("GeometryRequestScheduler", () => {
  it("serializes rapid A to B to C updates and applies only A then latest C", async () => {
    const completions = new Map<string, ReturnType<typeof deferred>>();
    const started: string[] = [];
    const cStarted = deferred();
    let inFlight = 0;
    let maximumInFlight = 0;
    const scheduler = new GeometryRequestScheduler<TestGeometryRequest>((request) => {
      started.push(request.id);
      inFlight += 1;
      maximumInFlight = Math.max(maximumInFlight, inFlight);
      const completion = deferred();
      completions.set(request.id, completion);
      if (request.id === "C") cStarted.resolve();
      return completion.promise.finally(() => {
        inFlight -= 1;
      });
    });
    const request = (id: TestGeometryRequest["id"]): TestGeometryRequest => ({
      id,
      expandedProviderCount: 1,
      bubbleCount: 0,
    });
    const a = request("A");
    const c = request("C");

    const appliedA = scheduler.enqueue(a);
    const appliedB = scheduler.enqueue(request("B"));
    const appliedC = scheduler.enqueue(c);

    expect(started).toEqual(["A"]);
    expect(maximumInFlight).toBe(1);
    completions.get("A")?.resolve();
    await cStarted.promise;
    expect(started).toEqual(["A", "C"]);
    expect(maximumInFlight).toBe(1);
    expect(scheduler.lastGeometry).toBe("");

    completions.get("C")?.resolve();
    await Promise.all([appliedA, appliedB, appliedC]);
    expect(started).toEqual(["A", "C"]);
    expect(scheduler.lastGeometry).toBe(JSON.stringify(c));
  });

  it("does not invoke native geometry for an empty provider state", async () => {
    const started: TestGeometryRequest[] = [];
    const scheduler = new GeometryRequestScheduler<TestGeometryRequest>(async (request) => {
      started.push(request);
    });

    await scheduler.enqueue({
      id: "empty",
      expandedProviderCount: 0,
      bubbleCount: 0,
    });

    expect(started).toEqual([]);
    expect(scheduler.lastGeometry).toBe("");
  });
});
