import { describe, expect, it } from "vitest";
import { catalog, SUPPORTED_LOCALES } from "./catalog";
import { createI18n } from "./i18n";

describe("locale catalog", () => {
  it("ships every required locale with every English key", () => {
    for (const locale of SUPPORTED_LOCALES) expect(Object.keys(catalog[locale]).sort()).toEqual(Object.keys(catalog.en).sort());
  });
  it("falls back by locale then English", () => {
    expect(createI18n("pt-BR").t("action.refresh")).toBe("Atualizar");
    expect(createI18n("xx").t("action.quit")).toBe("Quit");
  });
});
