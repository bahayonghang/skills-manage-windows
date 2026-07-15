export function formatUsageRelativeTime(
  timestampMs: number,
  locale?: string,
): string {
  if (!Number.isFinite(timestampMs) || timestampMs <= 0) return "—";
  const elapsedMs = Math.max(0, Date.now() - timestampMs);
  const minutes = Math.floor(elapsedMs / 60_000);
  if (minutes < 1) return locale?.toLowerCase().startsWith("zh") ? "刚刚" : "now";

  const formatter = new Intl.RelativeTimeFormat(locale || undefined, {
    numeric: "always",
    style: "narrow",
  });
  if (minutes < 60) return formatter.format(-minutes, "minute");
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return formatter.format(-hours, "hour");
  const days = Math.floor(hours / 24);
  if (days < 30) return formatter.format(-days, "day");
  return formatter.format(-Math.floor(days / 30), "month");
}
