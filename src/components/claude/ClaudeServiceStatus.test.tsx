import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ClaudeServiceStatus } from "./ClaudeServiceStatus";
describe("ClaudeServiceStatus", () => { it("announces state and safe incident links", () => { render(<ClaudeServiceStatus status={{ indicator: "Degraded", description: "Issues", incidents: [{ name: "Incident", status: "investigating", url: "https://status.claude.com/incidents/a" }] }} />); expect(screen.getByRole("status").textContent).toContain("Degraded"); expect(screen.getByRole("link").getAttribute("rel")).toContain("noreferrer"); }); });
