/**
 * 把 Date / 毫秒时间戳格式化为简短的相对时间，与 skilled CLI 的 timeAgo 等价。
 *
 * 返回："now" / "Nm" / "Nh" / "Nd" / "Nmo"
 */
export function timeAgo(input: Date | number | null | undefined): string {
  if (input === null || input === undefined) return "—";
  const ms = typeof input === "number" ? input : input.getTime();
  if (!Number.isFinite(ms) || ms <= 0) return "—";
  const diff = Date.now() - ms;
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "now";
  if (mins < 60) return `${mins}m`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d`;
  const months = Math.floor(days / 30);
  return `${months}mo`;
}

/** 取 path 最后一段作为短名（与 skilled `projectShort` 等价）。Windows + POSIX 兼容。 */
export function projectShort(path: string): string {
  if (!path) return "";
  // 同时支持 / 与 \\ 分隔符
  const parts = path.replace(/[\\/]+$/, "").split(/[\\/]+/);
  return parts[parts.length - 1] || path;
}
