import {
  PackagePlus,
  Check,
  Link2,
  ArrowUpRight,
  Plus,
  ChevronRight,
  X,
  Loader2,
  Trash2,
  Download,
} from "lucide-react";
import { memo, useMemo, type MouseEventHandler, type ReactNode, type Ref } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { CompactCardMoreMenu } from "@/components/skill/CompactCardMoreMenu";
import { SkillCardMeta } from "@/components/skill/SkillCardMeta";
import { SourceIndicator } from "@/components/skill/SkillCardBadges";
import { useTextTruncation } from "@/hooks/useTextTruncation";
import type { AgentWithStatus, CentralSkillUpdateState, ClaudeSourceKind } from "@/types";
import { cn } from "@/lib/utils";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberIds,
  getPlatformTargetMemberNames,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";
import {
  selectSkillInventoryFlagsFromInventory,
  useUpdateCenterStore,
} from "@/stores/updateCenterStore";

// ─── Platform Toggle Icon (internal) ──────────────────────────────────────────

const PlatformToggleIcon = memo(function PlatformToggleIcon({
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
  const displayName = isUniversalPlatformTarget(agent)
    ? t("platformTargets.universalShortLabel")
    : agent.display_name;
  const isDisabled = isToggling || isLocked;
  const title = isLocked
    ? `${displayName} - ${t("platformTargets.alwaysIncluded")} - ${memberNames}`
    : displayName;

  return (
    <button
      className={cn(
        "p-1 rounded-md transition-colors cursor-pointer",
        isLocked
          ? "text-primary cursor-default"
          : isLinked
          ? "text-primary hover:bg-primary/15"
          : "text-muted-foreground/40 hover:bg-muted/60 hover:text-muted-foreground",
        isToggling && "animate-pulse pointer-events-none"
      )}
      title={title}
      aria-label={
        isLocked
          ? title
          : t("central.toggleInstallLabel", { platform: displayName, skill: skillName })
      }
      disabled={isDisabled}
      onClick={onToggle}
    >
      <PlatformIcon agentId={agent.id} className="size-4 shrink-0" size={16} />
    </button>
  );
});
PlatformToggleIcon.displayName = "PlatformToggleIcon";

// ─── Types ────────────────────────────────────────────────────────────────────

/**
 * 卡片视觉密度：
 * - `comfortable`（默认）：所有元素自适应、间距宽松；min-h 保底；描述 line-clamp-3 + fade mask
 * - `compact`：idle 隐藏动作 / 平台图标条收成"已链接 N"；hover 展开；描述 line-clamp-2 + fade mask
 * - `default`：旧别名，等同 `comfortable`，仅作向后兼容
 */
export type SkillCardDensity = "comfortable" | "compact" | "default";

export interface UnifiedSkillCardProps {
  /** Core data — always required. */
  name: string;
  description?: string;
  aiSummary?: string | null;
  summaryLabel?: string;
  className?: string;

  /**
   * 当前 skill 在中央库中的 id。设置后会从 `useUpdateCenterStore` 查询 inventory
   * 派生 badge（platform duplicate / orphan）。central / project 卡片应传入。
   */
  skillId?: string;

  /** Click the card itself (platform variant navigates to detail). */
  onClick?: () => void;

  // ── discover variant ──
  checkbox?: { checked: boolean; onChange: () => void };
  isCentral?: boolean;
  platformBadge?: { id: string; name: string };
  projectBadge?: string;
  originBadge?: { kind: "central" | "project" | string; label: string };

  // ── central variant ──
  platformIcons?: {
    agents: AgentWithStatus[];
    linkedAgents: string[];
    lockedAgentIds?: string[];
    skillId: string;
    onToggle: (skillId: string, agentId: string) => void;
    togglingAgentId: string | null;
  };

  // ── platform variant ──
  sourceType?: "symlink" | "copy" | "native";
  originKind?: ClaudeSourceKind | null;
  isReadOnly?: boolean;

  // ── marketplace variant ──
  isInstalled?: boolean;
  tags?: { key: string; label: string }[];
  publisher?: string;
  installLabel?: string;

  /**
   * 「近 30 天调用 N 次」徽章 —— 由 useSkillCallCounts 注入。
   * 仅当数值 > 0 时渲染；undefined 或 0 完全不出现，避免误导。
   */
  usageBadge?: number;

  // ── actions ──
  onDetail?: MouseEventHandler<HTMLButtonElement>;
  onInstallTo?: () => void;
  onUpdateCentral?: () => void;
  updateStatus?: CentralSkillUpdateState & { isUpdating?: boolean };
  onDeleteFromCentral?: () => void;
  onInstallToCentral?: () => void;
  onInstallToPlatform?: () => void;
  onUninstallFromPlatform?: () => void;
  uninstallFromLabel?: string;
  onInstall?: () => void;
  onRemove?: () => void;
  isLoading?: boolean;
  detailButtonRef?: Ref<HTMLButtonElement>;
  density?: SkillCardDensity;
}

// ─── UnifiedSkillCard ─────────────────────────────────────────────────────────

function UnifiedSkillCardComponent(props: UnifiedSkillCardProps) {
  const { t } = useTranslation();
  const {
    name,
    description,
    aiSummary,
    summaryLabel,
    className,
    skillId,
    onClick,
    checkbox,
    isCentral,
    platformBadge,
    projectBadge,
    originBadge,
    platformIcons,
    sourceType,
    originKind,
    isReadOnly,
    isInstalled,
    tags,
    publisher,
    installLabel,
    usageBadge,
    onDetail,
    onInstallTo,
    onUpdateCentral,
    updateStatus,
    onDeleteFromCentral,
    onInstallToCentral,
    onInstallToPlatform,
    onUninstallFromPlatform,
    uninstallFromLabel,
    onInstall,
    onRemove,
    isLoading,
    detailButtonRef,
    density: rawDensity = "comfortable",
  } = props;

  // "default" 是旧别名，归一化为 "comfortable"
  const density: Exclude<SkillCardDensity, "default"> =
    rawDensity === "default" ? "comfortable" : rawDensity;
  const isCompact = density === "compact";

  const inventorySkillId = skillId ?? platformIcons?.skillId ?? null;
  const updateCenterInventory = useUpdateCenterStore((state) => state.inventory);
  const inventoryFlags = useMemo(
    () =>
      inventorySkillId
        ? selectSkillInventoryFlagsFromInventory(
            updateCenterInventory,
            inventorySkillId,
          )
        : null,
    [inventorySkillId, updateCenterInventory],
  );

  const hasCheckbox = !!checkbox;
  const hasPlatformIcons = !!platformIcons;
  const hasActions = !!(
    onDetail ||
    onInstallTo ||
    onUpdateCentral ||
    onDeleteFromCentral ||
    onInstallToCentral ||
    onInstallToPlatform ||
    onUninstallFromPlatform ||
    onInstall ||
    onRemove
  );
  const trimmedAiSummary = aiSummary?.trim() ?? "";
  const hasAiSummary = trimmedAiSummary.length > 0;
  const summaryText = hasAiSummary ? trimmedAiSummary : description;
  const resolvedSummaryLabel = summaryLabel ?? t("common.aiSummaryLabel");
  const canUpdateCentral = updateStatus?.status === "update_available";

  const targetAgents = useMemo(
    () => platformIcons?.agents.filter((a) => a.id !== "central") ?? [],
    [platformIcons?.agents]
  );
  const linkedAgentSet = useMemo(
    () => new Set(platformIcons?.linkedAgents ?? []),
    [platformIcons?.linkedAgents]
  );
  const lockedAgentSet = useMemo(
    () => new Set(platformIcons?.lockedAgentIds ?? []),
    [platformIcons?.lockedAgentIds]
  );

  const summaryLineClamp: 2 | 3 = isCompact ? 2 : 3;
  const linkedTargetCount = useMemo(
    () =>
      targetAgents.filter((agent) =>
        getPlatformTargetMemberIds(agent).some((agentId) => linkedAgentSet.has(agentId))
      ).length,
    [targetAgents, linkedAgentSet]
  );
  const compactMoreMenuItems = isCompact && onDeleteFromCentral ? { onDeleteFromCentral } : null;

  // ── Platform variant: clickable card style ──
  if (onClick && !hasActions && !hasCheckbox && !hasPlatformIcons) {
    return (
      <button
        role="button"
        onClick={onClick}
        className={cn(
          "w-full h-full text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-xl group/skill-card",
          className
        )}
        aria-label={t("platform.searchSkillLabel", { name })}
      >
        <div className={cn(cardShellClass(false), "p-3.5 gap-2 hover:ring-primary/30 cursor-pointer")}>
          <div className="flex flex-1 items-start justify-between gap-3 min-w-0">
            <div className="min-w-0 flex-1 flex flex-col gap-2">
              <div className="truncate text-sm font-semibold tracking-[-0.01em] text-foreground transition-colors group-hover/skill-card:text-primary">
                {name}
              </div>
              {summaryText && (
                <SkillCardSummary
                  text={summaryText}
                  label={hasAiSummary ? resolvedSummaryLabel : undefined}
                  lineClamp={summaryLineClamp}
                />
              )}
              {sourceType && (
                <div className="flex flex-wrap items-center gap-1.5">
                  <SourceIndicator sourceType={sourceType} />
                </div>
              )}
            </div>
            <ChevronRight className="size-4 text-muted-foreground shrink-0 mt-0.5" />
          </div>
        </div>
      </button>
    );
  }

  // ── Default card style (central, discover, marketplace, collection) ──
  return (
    <div
      className={cn(
        cardShellClass(checkbox?.checked),
        "group/skill-card p-3.5 gap-2",
        isCompact ? "min-h-[148px]" : "min-h-[176px]",
        isLoading && "opacity-50",
        className
      )}
    >
      <div className="flex min-h-0 flex-1 items-start gap-2.5">
        {/* Optional checkbox (discover) */}
        {hasCheckbox && checkbox && (
          <div className="pt-0.5">
            <Checkbox
              checked={checkbox.checked}
              onCheckedChange={checkbox.onChange}
              aria-label={t("common.selectSkill")}
            />
          </div>
        )}

        {/* Main content */}
        <div className="flex h-full min-w-0 flex-1 flex-col gap-2">
          {/* Row 1: Name + icon actions */}
          <div className="flex items-center justify-between gap-2">
            {onDetail ? (
              <button
                ref={detailButtonRef}
                className="min-w-0 flex-1 truncate rounded-sm text-left text-sm font-semibold tracking-[-0.01em] text-foreground underline-offset-4 transition-colors hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                onClick={onDetail}
                aria-label={t("central.viewDetailsLabel", { name })}
              >
                {name}
              </button>
            ) : (
              <h3 className="min-w-0 flex-1 truncate text-sm font-semibold tracking-[-0.01em] text-foreground">
                {name}
              </h3>
            )}

            {hasActions && (
              <div
                className={cn(
                  "flex items-center gap-0.5 shrink-0",
                  isCompact &&
                    "opacity-0 transition-opacity duration-150 group-hover/skill-card:opacity-100 group-focus-within/skill-card:opacity-100",
                )}
              >
                {onInstallTo && (
                  <CardActionButton
                    onClick={onInstallTo}
                    disabled={isLoading}
                    title={t("central.installTo")}
                    ariaLabel={t("central.installLabel", { name })}
                    icon={<PackagePlus className="size-4" />}
                  />
                )}

                {onUpdateCentral && (
                  <CardActionButton
                    onClick={onUpdateCentral}
                    disabled={
                      isLoading || updateStatus?.isUpdating || !canUpdateCentral
                    }
                    title={
                      canUpdateCentral
                        ? t("central.updateSkill")
                        : updateStatus?.error ?? t("central.checkUpdatesFirst")
                    }
                    ariaLabel={t("central.updateSkillLabel", { name })}
                    icon={
                      updateStatus?.isUpdating ? (
                        <Loader2 className="size-4 animate-spin" />
                      ) : (
                        <Download className="size-4" />
                      )
                    }
                  />
                )}

                {/* Delete — compact 模式收进下面的 ⋯ popover，comfortable 保持一级图标。 */}
                {onDeleteFromCentral && !isCompact && (
                  <CardActionButton
                    onClick={onDeleteFromCentral}
                    disabled={isLoading}
                    title={t("central.deleteSkill")}
                    ariaLabel={t("central.deleteSkillLabel", { name })}
                    testId={`delete-central-skill-${name}`}
                    danger
                    icon={<Trash2 className="size-4" />}
                  />
                )}

                {onInstallToCentral && !isCentral && (
                  <CardActionButton
                    onClick={onInstallToCentral}
                    disabled={isLoading}
                    title={t("common.installToCentral")}
                    ariaLabel={t("common.installToCentral")}
                    icon={
                      isLoading ? (
                        <Loader2 className="size-4 animate-spin" />
                      ) : (
                        <ArrowUpRight className="size-4" />
                      )
                    }
                  />
                )}

                {onInstallToPlatform && (
                  <CardActionButton
                    onClick={onInstallToPlatform}
                    disabled={isLoading}
                    title={t("common.installToPlatform")}
                    ariaLabel={t("common.installToPlatform")}
                    icon={
                      isLoading ? (
                        <Loader2 className="size-4 animate-spin" />
                      ) : (
                        <Plus className="size-4" />
                      )
                    }
                  />
                )}

                {onUninstallFromPlatform && (
                  <InlineConfirmAction
                    onConfirm={onUninstallFromPlatform}
                    isLoading={isLoading}
                    idleTitle={uninstallFromLabel ?? t("common.uninstall")}
                    idleAriaLabel={uninstallFromLabel ?? t("common.uninstall")}
                    confirmLabel={t("common.confirmDelete")}
                    icon={<X className="size-4" />}
                  />
                )}

                {onInstall && (
                  <button
                    onClick={isInstalled ? undefined : onInstall}
                    disabled={isInstalled || isLoading}
                    title={
                      isInstalled
                        ? t("marketplace.installed")
                        : installLabel ?? t("marketplace.install")
                    }
                    aria-label={
                      isInstalled
                        ? t("marketplace.installed")
                        : installLabel ?? t("marketplace.install")
                    }
                    className={cn(
                      "inline-flex h-7 w-7 items-center justify-center rounded-md transition-colors",
                      isInstalled
                        ? "text-primary cursor-default"
                        : "text-muted-foreground hover:bg-accent/40 hover:text-primary disabled:opacity-50 disabled:cursor-default"
                    )}
                  >
                    {isLoading ? (
                      <Loader2 className="size-4 animate-spin" />
                    ) : isInstalled ? (
                      <Check className="size-4" />
                    ) : (
                      <Download className="size-4" />
                    )}
                  </button>
                )}

                {onRemove && (
                  <InlineConfirmAction
                    onConfirm={onRemove}
                    isLoading={isLoading}
                    idleTitle={t("collection.removeSkillLabel", { name })}
                    idleAriaLabel={t("collection.removeSkillLabel", { name })}
                    confirmLabel={t("common.confirmDelete")}
                    icon={<X className="size-4" />}
                  />
                )}

                {compactMoreMenuItems && (
                  <CompactCardMoreMenu
                    skillName={name}
                    isLoading={Boolean(isLoading)}
                    onDeleteFromCentral={compactMoreMenuItems.onDeleteFromCentral}
                  />
                )}
              </div>
            )}
          </div>

          {/* Row 2: Description */}
          {summaryText && (
            <SkillCardSummary
              text={summaryText}
              label={hasAiSummary ? resolvedSummaryLabel : undefined}
              lineClamp={summaryLineClamp}
            />
          )}

          {/* Row 3: Meta badges — 统一一行展示，flex-wrap */}
          <SkillCardMeta
            originKind={originKind ?? undefined}
            isReadOnly={isReadOnly}
            sourceType={sourceType}
            originBadge={originBadge}
            usageBadge={usageBadge}
            isCentral={isCentral}
            platformBadge={platformBadge}
            projectBadge={projectBadge}
            publisher={publisher}
            updateStatus={updateStatus}
            inventoryFlags={inventoryFlags ?? undefined}
            inventorySkillId={inventorySkillId ?? undefined}
            tags={tags}
          />

          {/* Row 4: Platform toggle icons (central) */}
          {hasPlatformIcons && platformIcons && targetAgents.length > 0 && (
            <div className="mt-auto pt-1">
              {isCompact && (
                <span
                  data-testid={`skill-card-linked-summary-${platformIcons.skillId}`}
                  className={cn(
                    "inline-flex items-center gap-1 rounded-full border border-border/70 bg-muted/40 px-2 py-0.5 text-[10px] font-medium text-muted-foreground",
                    "group-hover/skill-card:hidden group-focus-within/skill-card:hidden"
                  )}
                >
                  <Link2 className="size-3" aria-hidden />
                  {linkedTargetCount > 0
                    ? t("common.skillCardLinkedSummary", { count: linkedTargetCount })
                    : t("common.skillCardLinkedSummaryNone")}
                </span>
              )}
              <div
                className={cn(
                  "flex items-center gap-1 flex-wrap",
                  isCompact &&
                    "hidden group-hover/skill-card:flex group-focus-within/skill-card:flex"
                )}
              >
                {targetAgents.map((agent) => (
                  <PlatformToggleIcon
                    key={agent.id}
                    agent={agent}
                    skillName={name}
                    isLinked={getPlatformTargetMemberIds(agent).some((agentId) => linkedAgentSet.has(agentId))}
                    isToggling={getPlatformTargetMemberIds(agent).some((agentId) => platformIcons.togglingAgentId === agentId)}
                    isLocked={getPlatformTargetMemberIds(agent).some((agentId) => lockedAgentSet.has(agentId))}
                    onToggle={() => {
                      const [agentId] = getPlatformTargetInstallAgentIds(agent);
                      if (agentId) {
                        platformIcons.onToggle(platformIcons.skillId, agentId);
                      }
                    }}
                  />
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export const UnifiedSkillCard = memo(UnifiedSkillCardComponent);
UnifiedSkillCard.displayName = "UnifiedSkillCard";

// ─── Card Shell Styles ────────────────────────────────────────────────────────

function cardShellClass(selected?: boolean): string {
  return cn(
    "h-full flex flex-col rounded-xl bg-card ring-1 ring-border/70 transition-all duration-150",
    "shadow-[0_1px_2px_rgba(0,0,0,0.04),0_4px_12px_-6px_rgba(0,0,0,0.08)]",
    "hover:ring-primary/25 hover:shadow-[0_2px_4px_rgba(0,0,0,0.05),0_10px_28px_-10px_color-mix(in_srgb,var(--primary)_24%,transparent)]",
    selected && "ring-primary/45 bg-primary/[0.04]",
  );
}

// ─── Card Action Button (internal) ────────────────────────────────────────────

function CardActionButton({
  onClick,
  disabled,
  title,
  ariaLabel,
  icon,
  testId,
  danger,
}: {
  onClick: () => void;
  disabled?: boolean;
  title: string;
  ariaLabel: string;
  icon: ReactNode;
  testId?: string;
  danger?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={ariaLabel}
      data-testid={testId}
      className={cn(
        "inline-flex h-7 w-7 items-center justify-center rounded-md transition-colors text-muted-foreground disabled:opacity-50 disabled:cursor-default",
        danger
          ? "hover:bg-destructive/10 hover:text-destructive"
          : "hover:bg-accent/40 hover:text-primary",
      )}
    >
      {icon}
    </button>
  );
}

// ─── Skill Card Summary (description / aiSummary with fade mask) ─────────────

function SkillCardSummary({
  text,
  label,
  lineClamp = 2,
}: {
  text: string;
  label?: string;
  lineClamp?: 2 | 3;
}) {
  const { ref, isTruncated } = useTextTruncation<HTMLParagraphElement>(text);
  return (
    <div className="relative">
      {label && (
        <span className="mr-1.5 inline-flex align-baseline rounded-full border border-primary/15 bg-primary/8 px-1.5 py-0.5 text-[9px] font-medium leading-none text-primary/85">
          {label}
        </span>
      )}
      <p
        ref={ref}
        data-truncated={isTruncated ? "true" : "false"}
        title={text}
        className={cn(
          "text-xs leading-relaxed text-muted-foreground/90",
          lineClamp === 3 ? "line-clamp-3" : "line-clamp-2",
          label && "inline",
        )}
      >
        {text}
      </p>
      <span
        aria-hidden
        data-truncated={isTruncated ? "true" : "false"}
        className={cn(
          "pointer-events-none absolute inset-x-0 bottom-0 h-5",
          "bg-gradient-to-t from-card to-transparent",
          "opacity-0 transition-opacity duration-150",
          "data-[truncated=true]:opacity-100",
        )}
      />
    </div>
  );
}
