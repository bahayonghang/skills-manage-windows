import type {
  UnusedAgentInstall,
  UnusedPlatformInstall,
  UnusedSkillEntry,
} from "@/types/usage";

/**
 * Unlink 弹窗的逐项禁用原因。Central 条目只可能出现前两种；平台条目
 * 可能出现后四种（sourceKind / rowId 是 observation 行级字段）。
 */
export type UnlinkDisabledReason =
  | "disabledPendingRecovery"
  | "disabledSharedRoot"
  | "disabledReadOnly"
  | "disabledSourceKind"
  | "disabledNoRow";

/** 弹窗内统一的目标模型：Central 取 entry.agents，平台取 entry.installs 全量。 */
export interface UnlinkTarget {
  skillId: string;
  agentId: string;
  rowId: string | null;
  disabledReason: UnlinkDisabledReason | null;
}

function centralDisabledReason(
  install: UnusedAgentInstall,
): UnlinkDisabledReason | null {
  if (install.hasPendingRecovery) return "disabledPendingRecovery";
  if (install.agentId === "central" || install.linkType === "native") {
    return "disabledSharedRoot";
  }
  return null;
}

/** Central 条目归一：per-agent 安装行，rowId 恒为 null（按 skillId+agentId unlink）。 */
export function centralTargets(entry: UnusedSkillEntry): UnlinkTarget[] {
  return entry.agents.map((install) => ({
    skillId: entry.skillId ?? "",
    agentId: install.agentId,
    rowId: null,
    disabledReason: centralDisabledReason(install),
  }));
}

/** 平台条目的逐项禁用原因（从旧行内 PlatformUnlinkAction 迁移）。 */
export function platformUnlinkDisabledReason(
  install: UnusedPlatformInstall,
): UnlinkDisabledReason | null {
  if (install.hasPendingRecovery) return "disabledPendingRecovery";
  if (install.isReadOnly) return "disabledReadOnly";
  if (install.sourceKind !== "user") return "disabledSourceKind";
  if (install.rowId === null) return "disabledNoRow";
  return null;
}

/** 平台条目归一：列出该技能在所有 Agent 上的 observation 全量（跨小节 Agent）。 */
export function platformTargets(entry: UnusedSkillEntry): UnlinkTarget[] {
  return entry.installs.map((install) => ({
    skillId: install.skillId,
    agentId: install.agentId,
    rowId: install.rowId,
    disabledReason: platformUnlinkDisabledReason(install),
  }));
}
