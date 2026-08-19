import { expect, it, vi } from "vitest";
import { createAppStore } from "../app/store";
import { UsageController } from "./usage-controller";

it("start and stop are idempotent across a restart", async () => {
  const unlisten = vi.fn();
  const listen = vi.fn().mockResolvedValue(unlisten);
  const controller = new UsageController(createAppStore(), { listen, invoke: vi.fn().mockResolvedValue({ sources: { claude: false, openai: false }, usage: [] }) });
  await controller.start(); await controller.start(); await controller.stop(); await controller.stop(); await controller.start();
  expect(listen).toHaveBeenCalledTimes(4);
  expect(unlisten).toHaveBeenCalledTimes(2);
});
