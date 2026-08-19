import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClaudeUsageDetails } from "./ClaudeUsageDetails";
describe("ClaudeUsageDetails", () => { it("renders unknown models and omits absent money", () => { render(<ClaudeUsageDetails details={{ limits: { value: [{ modelKey: "claude-next-x", displayName: "Future", utilizationPercent: 120, resetsAt: null }], fetchedAt: 1, state: "fresh", errorCode: null }, extra: { value: {}, fetchedAt: 1, state: "fresh", errorCode: null } }} />); expect(screen.getByText("Future")).toBeTruthy(); expect(screen.getByText("120%")) .toBeTruthy(); expect(screen.queryByText(/spend/i)).toBeNull(); }); });
