export const SUPPORTED_LOCALES = ["en","vi","es","fr","de","it","pt","pt-BR","ja","ko","zh-CN","zh-TW","tr","uk"] as const;
export type Locale = typeof SUPPORTED_LOCALES[number];
export type MessageKey = "action.refresh"|"action.settings"|"action.quit"|"action.detach"|"action.attach"|"popover.title"|"status.stale"|"status.error"|"status.refreshing"|"settings.language"|"overlay.show"|"overlay.hide";
export type Messages = Record<MessageKey, string>;
