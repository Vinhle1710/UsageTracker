import {render,screen,cleanup} from "@testing-library/react";import {describe,it,expect,afterEach} from "vitest";import {Indicator} from "./Indicator";import type {DisplayModel} from "./model";
afterEach(cleanup);
const base:DisplayModel={provider:"openai",style:"compact",metrics:[{id:"session",usedPercent:42,displayPercent:42,label:"42% used",severity:"normal"},{id:"weekly",usedPercent:68,displayPercent:68,label:"68% used",severity:"normal"}]};
describe("Indicator",()=>{it.each(["battery","horizontal-progress","percentage","provider-icon-bar","compact"] as const)("renders %s accessibly",style=>{render(<Indicator model={{...base,style}}/>);expect(screen.getByTestId(`indicator-${style}`)).toBeTruthy();expect(screen.getAllByRole("progressbar")).toHaveLength(2);})});
