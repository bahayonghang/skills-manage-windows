import type { CentralSkillUpdateStatus } from "@/types";

/** 更新状态 → 卡片左侧状态竖条强度（仅可更新/源缺失/错误上色）。 */
export function statusAccentOf(
  status: CentralSkillUpdateStatus | undefined,
): "warning" | "error" | undefined {
  if (status === "update_available") return "warning";
  if (status === "remote_missing" || status === "error") return "error";
  return undefined;
}
