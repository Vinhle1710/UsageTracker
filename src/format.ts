export function formatPercent(percent: number): string {
  return `${Math.round(percent)}%`;
}

export function formatCountdown(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remainingSeconds = total % 60;
  return [hours, minutes, remainingSeconds].map((value) => String(value).padStart(2, "0")).join(":");
}

export function formatWeeklyReset(resetsAt: number): string {
  const date = new Date(resetsAt * 1000);
  const month = date.toLocaleString("en-US", { month: "short" });
  const day = String(date.getDate()).padStart(2, "0");
  return `${month} ${day}`;
}

export function formatCountdownUntilReset(resetsAt: number, now: number): string {
  const secondsUntil = Math.max(0, Math.floor(resetsAt - now));
  const days = Math.floor(secondsUntil / 86400);
  const hours = Math.floor((secondsUntil % 86400) / 3600);
  const minutes = Math.floor((secondsUntil % 3600) / 60);
  const seconds = secondsUntil % 60;
  const clock = [hours, minutes, seconds].map((value) => String(value).padStart(2, "0")).join(":");
  return days > 0 ? `${days}d ${clock}` : clock;
}

const FUN_RESET_MESSAGES = [
  "Recharging the quota…",
  "Reticulating tokens…",
  "Politely waiting its turn…",
  "Catching its breath…",
  "Warming up the limiter…",
  "Syncing with the mothership…",
  "Consulting the oracle…",
  "Running the cosmic clock…",
];

export function getFunPlaceholder(): string {
  return FUN_RESET_MESSAGES[Math.floor(Math.random() * FUN_RESET_MESSAGES.length)];
}

export function formatReset(label: string, resetsAt: number, now: number): string {
  if (!Number.isFinite(resetsAt) || resetsAt <= 0) return "reset time unavailable";
  if (/(hour|min)/i.test(label)) return `resets in ${formatCountdown(resetsAt - now)}`;
  return `${formatWeeklyReset(resetsAt)} · ${formatCountdownUntilReset(resetsAt, now)}`;
}

/** Provider costs arrive as micro-units — millionths of a currency unit. Rendering the raw
 *  integer ("1234567 μ") is unreadable, but rounding hard to two decimals turns real
 *  sub-cent API pricing into $0.00, so the window is two-to-four decimals: familiar money for
 *  ordinary amounts, enough precision for tiny ones. Locale is the user's own. */
export function formatMicros(amountMicros: number, currency = "USD"): string {
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 4,
  }).format(amountMicros / 1_000_000);
}
