import { catalog, SUPPORTED_LOCALES } from "./catalog";
import type { Locale, MessageKey } from "./types";
export function createI18n(input: string): { locale: Locale; t: (key: MessageKey, values?: Record<string, string | number>) => string } {
 const normalized = input.replace(/_/g, "-"); const exact = SUPPORTED_LOCALES.find((l) => l.toLowerCase() === normalized.toLowerCase()); const base = SUPPORTED_LOCALES.find((l) => l.toLowerCase() === normalized.split("-")[0].toLowerCase()); const locale = exact ?? base ?? "en";
 return { locale, t: (key, values = {}) => Object.entries(values).reduce((text, [name, value]) => text.replace(new RegExp(`\\{${name}\\}`, "g"), String(value).replace(/[&<>\"']/g, "")), catalog[locale][key] ?? catalog.en[key]) };
}
