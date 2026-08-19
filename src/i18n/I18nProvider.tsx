import { createContext, useContext, useMemo } from "react";
import { createI18n } from "./i18n";
import type { Locale } from "./types";
type Value = ReturnType<typeof createI18n> & { numberFormat: Intl.NumberFormat; relativeTimeFormat: Intl.RelativeTimeFormat };
const Context = createContext<Value | null>(null);
export function I18nProvider({ locale, children }: { locale: Locale | string; children: React.ReactNode }) { const value = useMemo(() => ({ ...createI18n(locale), numberFormat: new Intl.NumberFormat(locale), relativeTimeFormat: new Intl.RelativeTimeFormat(locale, { numeric: "auto" }) }), [locale]); return <Context.Provider value={value}>{children}</Context.Provider>; }
export function useI18n(): Value { const value = useContext(Context); if (!value) throw new Error("I18nProvider is required"); return value; }
