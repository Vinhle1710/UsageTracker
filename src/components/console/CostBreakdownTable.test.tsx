import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CostBreakdownTable } from "./CostBreakdownTable";
import type { MoneyMinorUnits } from "../../types";
describe("CostBreakdownTable",()=>it("uses semantic caption and preserves redacted labels",()=>{render(<CostBreakdownTable title="Spend by API key" points={[{key:"k",label:"Key …AB12",amount:{minorUnits:"125" as MoneyMinorUnits,currency:"USD"}}]}/>); expect(screen.getByRole("table",{name:"Spend by API key"})).toBeTruthy(); expect(screen.getByText("Key …AB12")).toBeTruthy();}));
