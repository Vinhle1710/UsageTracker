import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    setupFiles: ["./vitest-setup.ts"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      exclude: ["src/main.ts", "src/types.ts", "src/**/*.test.ts", "scripts/**", "node_modules/**"],
      thresholds: { lines: 80, functions: 80, branches: 80 },
    },
  },
});
