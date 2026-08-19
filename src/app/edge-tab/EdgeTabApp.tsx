import { createI18n } from "../../i18n/i18n";
export function EdgeTabApp({ side = "right", hidden = false, reducedMotion = false, onToggle = () => undefined, locale = "en" }: { side?: "left" | "right"; hidden?: boolean; reducedMotion?: boolean; onToggle?: () => void; locale?: string }) {
  const { t } = createI18n(locale);
  const direction = hidden ? (side === "right" ? "left" : "right") : side;
  return <main id="app" data-window="edge-tab"><button type="button" aria-label={hidden ? t("overlay.show") : t("overlay.hide")} data-direction={direction} style={{ width: 24, height: 48, transition: reducedMotion ? "none" : undefined }} onClick={onToggle}><span aria-hidden="true">{hidden ? "‹" : "›"}</span></button></main>;
}
