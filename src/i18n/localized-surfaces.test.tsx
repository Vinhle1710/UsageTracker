import { expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { I18nProvider, useI18n } from "./I18nProvider";
function Surface() { const { t } = useI18n(); return <button>{t("overlay.show")}</button>; }
it("switches every mounted surface after canonical config changes", () => { const { rerender } = render(<I18nProvider locale="en"><Surface /></I18nProvider>); expect(screen.getByRole("button").textContent).toBe("Show usage overlay"); rerender(<I18nProvider locale="vi"><Surface /></I18nProvider>); expect(screen.getByRole("button").textContent).toBe("Hiện lớp phủ mức sử dụng"); });
