import { memo, useMemo } from "react";
import { useTranslation } from "react-i18next";

import { PlatformIcon } from "@/components/platform/PlatformIcon";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetLabel,
  getPlatformTargetMemberIds,
  getPlatformTargetMemberNames,
} from "@/lib/platformTargetGroups";
import { cn } from "@/lib/utils";
import type { AgentWithStatus } from "@/types";

export const PlatformToggleIcon = memo(function PlatformToggleIcon({
  agent,
  skillName,
  isLinked,
  isToggling,
  isLocked,
  onToggle,
}: {
  agent: AgentWithStatus;
  skillName: string;
  isLinked: boolean;
  isToggling: boolean;
  isLocked: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();
  const memberNames = getPlatformTargetMemberNames(agent).join(", ");
  const displayName = getPlatformTargetLabel(agent, t, "short");
  const isDisabled = isToggling || isLocked;
  const title = isLocked
    ? `${displayName} - ${t("platformTargets.alwaysIncluded")} - ${memberNames}`
    : displayName;

  return (
    <button
      className={cn(
        "focus-ring grid size-8 place-items-center rounded-lg transition-[scale,background-color,color] active:not-disabled:scale-[0.96]",
        isLocked
          ? "text-primary cursor-default ring-1 ring-primary/30"
          : isLinked
            ? "text-primary ring-1 ring-primary/30 hover:bg-primary/15"
            : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
        isToggling && "animate-pulse pointer-events-none",
      )}
      title={title}
      aria-label={
        isLocked
          ? title
          : t("central.toggleInstallLabel", {
              platform: displayName,
              skill: skillName,
            })
      }
      disabled={isDisabled}
      onClick={onToggle}
    >
      <PlatformIcon agentId={agent.id} className="size-4 shrink-0" size={16} />
    </button>
  );
});

export const UnifiedSkillCardFooter = memo(function UnifiedSkillCardFooter({
  repoName,
  repoColor,
  usageBadge,
  agents,
  linkedAgents,
  lockedAgentIds,
  skillId,
  togglingAgentId,
  skillName,
  onToggle,
}: {
  repoName?: string;
  repoColor?: string;
  usageBadge?: number;
  agents: AgentWithStatus[];
  linkedAgents: string[];
  lockedAgentIds?: string[];
  skillId?: string;
  togglingAgentId: string | null;
  skillName: string;
  onToggle?: (skillId: string, agentId: string) => void;
}) {
  const { t } = useTranslation();
  const targetAgents = useMemo(
    () => agents.filter((agent) => agent.id !== "central"),
    [agents],
  );
  const linkedAgentSet = useMemo(() => new Set(linkedAgents), [linkedAgents]);
  const lockedAgentSet = useMemo(
    () => new Set(lockedAgentIds ?? []),
    [lockedAgentIds],
  );

  return (
    <div
      data-testid="card-footer"
      className="mt-auto flex items-center justify-between gap-2 border-t border-border/50 pt-2"
    >
      <div className="flex min-w-0 items-center gap-1.5 text-ui-meta text-muted-foreground">
        {repoName && (
          <span className="flex min-w-0 items-center gap-1">
            <span
              aria-hidden
              className="size-2 shrink-0 rounded-sm"
              style={{
                backgroundColor: repoColor ?? "var(--muted-foreground)",
              }}
            />
            <span className="truncate">{repoName}</span>
          </span>
        )}
        {typeof usageBadge === "number" && usageBadge > 0 && (
          <>
            <span aria-hidden className="text-muted-foreground/40">
              ·
            </span>
            <span className="shrink-0 tabular-nums">
              {t("skillUsage.badge.countShort", { count: usageBadge })}
            </span>
          </>
        )}
      </div>
      {targetAgents.length > 0 && skillId && onToggle && (
        <div className="flex shrink-0 items-center gap-1">
          {targetAgents.map((agent) => (
            <PlatformToggleIcon
              key={agent.id}
              agent={agent}
              skillName={skillName}
              isLinked={getPlatformTargetMemberIds(agent).some((agentId) =>
                linkedAgentSet.has(agentId),
              )}
              isToggling={getPlatformTargetMemberIds(agent).some(
                (agentId) => togglingAgentId === agentId,
              )}
              isLocked={getPlatformTargetMemberIds(agent).some((agentId) =>
                lockedAgentSet.has(agentId),
              )}
              onToggle={() => {
                const [agentId] = getPlatformTargetInstallAgentIds(agent);
                if (agentId) onToggle(skillId, agentId);
              }}
            />
          ))}
        </div>
      )}
    </div>
  );
});
