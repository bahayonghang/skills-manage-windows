import {
  PackagePlus,
  Link2,
  Unlink,
  ArrowUpRight,
  Plus,
  X,
  Loader2,
  Trash2,
  Download,
} from "lucide-react";
import { memo, useMemo, type MouseEventHandler, type Ref } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { CompactCardMoreMenu } from "@/components/skill/CompactCardMoreMenu";
import {
  PlatformToggleIcon,
  UnifiedSkillCardFooter,
} from "@/components/skill/UnifiedSkillCardFooter";
import { CardTagEditor } from "@/components/skill/CardTagEditor";
import { SkillCardMeta } from "@/components/skill/SkillCardMeta";
import type { CentralSkillUpdateState, ClaudeSourceKind } from "@/types";
import { cn } from "@/lib/utils";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberIds,
} from "@/lib/platformTargetGroups";
import {
  selectSkillInventoryFlagsFromInventory,
  useUpdateCenterStore,
} from "@/stores/updateCenterStore";
import {
  CardActionButton,
  SkillCardSummary,
} from "./UnifiedSkillCardParts";
import { cardShellClass } from "./UnifiedSkillCard.styles";
import type * as SkillCardTypes from "./UnifiedSkillCard.types";

export type * from "./UnifiedSkillCard.types";

// ─── Internal render model ────────────────────────────────────────────────────

/** 模块私有渲染模型：各场景归一化后的扁平字段，渲染代码只面向它。 */
interface SkillCardModel {
  name: string;
  description?: string;
  aiSummary?: string | null;
  className?: string;
  skillId?: string;
  checkbox?: SkillCardTypes.SkillCardCheckbox;
  isCentral?: boolean;
  platformBadge?: { id: string; name: string };
  projectBadge?: string;
  originBadge?: { kind: "central" | "project" | string; label: string };
  platformIcons?: SkillCardTypes.SkillCardPlatformIcons;
  sourceType?: "symlink" | "copy" | "native";
  originKind?: ClaudeSourceKind | null;
  isReadOnly?: boolean;
  tags?: { key: string; label: string }[];
  publisher?: string;
  installLabel?: string;
  usageBadge?: number;
  lifetimeUsage?: { rank: number | null; count: number };
  originSurface?: "plugin" | "central" | "skillsCli";
  installOrigin?: "central" | "standalone" | "skillsCli";
  onDetail?: MouseEventHandler<HTMLButtonElement>;
  onInstallTo?: () => void;
  onUninstallFromPlatforms?: () => void;
  onUpdateCentral?: () => void;
  updateStatus?: CentralSkillUpdateState & { isUpdating?: boolean };
  onDeleteFromCentral?: () => void;
  onInstallToCentral?: () => void;
  onInstallToPlatform?: () => void;
  onUninstallFromPlatform?: () => void;
  uninstallFromLabel?: string;
  onInstall?: () => void;
  onRemove?: () => void;
  onUninstall?: () => void;
  uninstallLabel?: string;
  isLoading?: boolean;
  detailButtonRef?: Ref<HTMLButtonElement>;
  density?: SkillCardTypes.SkillCardDensity;
  statusAccent?: "warning" | "error";
  statusChipLabel?: string;
  editableTags?: SkillCardTypes.SkillCardEditableTags;
  footer?: SkillCardTypes.SkillCardFooter;
}

/** 场景 → 渲染模型的唯一映射点：每分支只拷贝该场景合法字段。 */
function toModel(props: SkillCardTypes.UnifiedSkillCardProps): SkillCardModel {
  const core = {
    name: props.name,
    description: props.description,
    aiSummary: props.aiSummary,
    className: props.className,
  };
  switch (props.variant) {
    case "central":
      return {
        ...core,
        skillId: props.skillId,
        checkbox: props.checkbox,
        statusAccent: props.statusAccent,
        statusChipLabel: props.statusChipLabel,
        tags: props.tags,
        publisher: props.publisher,
        usageBadge: props.usageBadge,
        updateStatus: props.updateStatus,
        onDetail: props.onDetail,
        onInstallTo: props.onInstallTo,
        onUninstallFromPlatforms: props.onUninstallFromPlatforms,
        onUpdateCentral: props.onUpdateCentral,
        onDeleteFromCentral: props.onDeleteFromCentral,
        detailButtonRef: props.detailButtonRef,
        editableTags: props.editableTags,
        density: props.density,
        platformIcons: props.platformIcons,
        footer: props.footer,
      };
    case "platform":
      return {
        ...core,
        sourceType: props.sourceType,
        originKind: props.originKind,
        isReadOnly: props.isReadOnly,
        publisher: props.publisher,
        usageBadge: props.usageBadge,
        lifetimeUsage: props.lifetimeUsage,
        installOrigin: props.installOrigin,
        originSurface:
          props.originKind === "plugin"
            ? "plugin"
            : props.installOrigin === "skillsCli"
              ? "skillsCli"
              : props.installOrigin === "central" ||
                  (props.installOrigin === undefined && props.sourceType === "symlink")
                ? "central"
                : undefined,
        checkbox: props.checkbox,
        isLoading: props.isLoading,
        onDetail: props.onDetail,
        onInstallTo: props.onInstallTo,
        onUninstallFromPlatform: props.onUninstallFromPlatform,
        uninstallFromLabel: props.uninstallFromLabel,
        detailButtonRef: props.detailButtonRef,
      };
    case "project":
      return {
        ...core,
        sourceType: props.sourceType,
        originBadge: props.originBadge,
        platformBadge: props.platformBadge,
        onUninstallFromPlatform: props.onUninstallFromPlatform,
        uninstallFromLabel: props.uninstallFromLabel,
        isLoading: props.isLoading,
      };
    case "import":
      return {
        ...core,
        isCentral: props.isCentral,
        platformBadge: props.platformBadge,
        projectBadge: props.projectBadge,
        onDetail: props.onDetail,
        detailButtonRef: props.detailButtonRef,
        onInstallToCentral: props.onInstallToCentral,
        onInstallToPlatform: props.onInstallToPlatform,
        isLoading: props.isLoading,
      };
    case "marketplace":
      return {
        ...core,
        publisher: props.publisher,
        tags: props.tags,
        onDetail: props.onDetail,
        onInstall: props.onInstall,
        installLabel: props.installLabel,
        isLoading: props.isLoading,
      };
    case "collection":
      return {
        ...core,
        onDetail: props.onDetail,
        detailButtonRef: props.detailButtonRef,
        onInstallTo: props.onInstallTo,
        onRemove: props.onRemove,
      };
    case "skillsCli":
      return {
        ...core,
        publisher: props.source ?? undefined,
        tags: props.agents.map((agent) => ({ key: agent, label: agent })),
        onUninstall: props.onUninstall,
        uninstallFromLabel: props.uninstallLabel,
        isLoading: props.isLoading,
      };
    default: {
      const _exhaustive: never = props;
      return _exhaustive;
    }
  }
}

// ─── UnifiedSkillCard ─────────────────────────────────────────────────────────

function UnifiedSkillCardComponent(props: SkillCardTypes.UnifiedSkillCardProps) {
  const { t } = useTranslation();
  const {
    name,
    description,
    aiSummary,
    className,
    skillId,
    checkbox,
    isCentral,
    platformBadge,
    projectBadge,
    originBadge,
    platformIcons,
    sourceType,
    originKind,
    isReadOnly,
    tags,
    publisher,
    installLabel,
    usageBadge,
    lifetimeUsage,
    originSurface,
    installOrigin,
    onDetail,
    onInstallTo,
    onUninstallFromPlatforms,
    onUpdateCentral,
    updateStatus,
    onDeleteFromCentral,
    onInstallToCentral,
    onInstallToPlatform,
    onUninstallFromPlatform,
    uninstallFromLabel,
    onInstall,
    onRemove,
    onUninstall,
    isLoading,
    detailButtonRef,
    density: rawDensity = "comfortable",
    statusAccent,
    statusChipLabel,
    editableTags,
    footer,
  } = toModel(props);

  // "default" 是旧别名，归一化为 "comfortable"
  const density: Exclude<SkillCardTypes.SkillCardDensity, "default"> =
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
    onUninstallFromPlatforms ||
    onUpdateCentral ||
    onDeleteFromCentral ||
    onInstallToCentral ||
    onInstallToPlatform ||
    onUninstallFromPlatform ||
    onInstall ||
    onRemove ||
    onUninstall
  );
  const trimmedAiSummary = aiSummary?.trim() ?? "";
  const hasAiSummary = trimmedAiSummary.length > 0;
  const summaryText = hasAiSummary ? trimmedAiSummary : description;
  const resolvedSummaryLabel = t("common.aiSummaryLabel");
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
  const hasLifetimeRank =
    lifetimeUsage != null &&
    lifetimeUsage.rank != null &&
    lifetimeUsage.rank >= 1;

  return (
    <div
      className={cn(
        cardShellClass(checkbox?.checked),
        "@container group/skill-card relative overflow-hidden gap-2 p-3.5",
        isCompact ? "min-h-[168px]" : "min-h-[188px]",
        originSurface === "plugin" && "bg-warning/5",
        originSurface === "skillsCli" && "bg-primary/5",
        isLoading && "opacity-50",
        className
      )}
    >
      {originSurface && (
        <div
          aria-hidden
          className={cn(
            "absolute inset-y-0 left-0 w-[3px]",
            originSurface === "plugin" ? "bg-warning" : "bg-info",
          )}
        />
      )}
      {lifetimeUsage && (
        <div
          data-testid="usage-rank"
          className="absolute bottom-3.5 right-3.5 text-xs tabular-nums"
          title={
            hasLifetimeRank
              ? t("platform.usageRank.tooltip", {
                  rank: lifetimeUsage.rank,
                  count: lifetimeUsage.count,
                })
              : t("platform.usageRank.tooltipNoRecord")
          }
          aria-label={
            hasLifetimeRank
              ? t("platform.usageRank.aria", {
                  rank: lifetimeUsage.rank,
                  count: lifetimeUsage.count,
                })
              : t("platform.usageRank.ariaNoRecord")
          }
        >
          {hasLifetimeRank ? (
            <>
              <span className="font-medium text-foreground">
                #{lifetimeUsage.rank}
              </span>
              <span className="text-muted-foreground">
                {" "}
                · {lifetimeUsage.count}
              </span>
            </>
          ) : (
            <span className="text-muted-foreground">
              {t("platform.usageRank.noRecord")}
            </span>
          )}
        </div>
      )}
      <div className="flex min-h-0 flex-1 items-start gap-2.5">
        {/* Optional checkbox (central / platform) */}
        {hasCheckbox && checkbox && (
          <div className="pt-0.5">
            <Checkbox
              checked={checkbox.checked}
              onCheckedChange={checkbox.onChange}
              aria-label={t("common.selectSkill")}
              className="relative size-5 after:absolute after:left-1/2 after:top-1/2 after:size-10 after:-translate-x-1/2 after:-translate-y-1/2 after:content-['']"
            />
          </div>
        )}

        {/* Main content */}
        <div className="flex h-full min-w-0 flex-1 flex-col gap-2">
          {/* Row 1: Name + icon actions（卡片 <22rem 时图标降为 hover/focus-within 揭示，让位给技能名；hover 与 focus 成对保证键盘可达） */}
          <div className="flex items-center justify-between gap-2">
            {onDetail ? (
              <button
                ref={detailButtonRef}
                className="min-h-7 min-w-0 flex-1 truncate rounded-md text-left text-sm font-semibold tracking-[-0.01em] text-foreground underline-offset-4 transition-[color,box-shadow] hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                onClick={onDetail}
                aria-label={t("central.viewDetailsLabel", { name })}
                title={name}
              >
                {name}
              </button>
            ) : (
              <h3 title={name} className="min-w-0 flex-1 truncate text-sm font-semibold tracking-[-0.01em] text-foreground">
                {name}
              </h3>
            )}

            {statusChipLabel && (
              <span
                className={cn(
                  "shrink-0 rounded-full px-1.5 py-0.5 text-xs font-medium ring-1",
                  statusAccent === "error"
                    ? "bg-destructive/10 text-destructive-text ring-destructive/30"
                    : "bg-warning/10 text-warning-foreground ring-warning/30"
                )}
              >
                {statusChipLabel}
              </span>
            )}

            {hasActions && (
              <div
                className={cn(
                  "flex shrink-0 items-center gap-2 @max-[22rem]:hidden @max-[22rem]:group-hover/skill-card:flex @max-[22rem]:group-focus-within/skill-card:flex",
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

                {onUninstallFromPlatforms && (
                  <CardActionButton
                    onClick={onUninstallFromPlatforms}
                    disabled={isLoading}
                    title={t("central.uninstallFromPlatforms")}
                    ariaLabel={t("central.uninstallFromPlatformsLabel", {
                      name,
                    })}
                    testId={`uninstall-platforms-skill-${name}`}
                    icon={<Unlink className="size-4" />}
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
                    onClick={onInstall}
                    disabled={isLoading}
                    title={installLabel ?? t("marketplace.install")}
                    aria-label={installLabel ?? t("marketplace.install")}
                    className="focus-ring inline-flex h-8 w-8 items-center justify-center rounded-lg transition-[scale,background-color,color] active:not-disabled:scale-[0.96] text-muted-foreground hover:bg-accent/40 hover:text-primary disabled:opacity-50 disabled:cursor-default"
                  >
                    {isLoading ? (
                      <Loader2 className="size-4 animate-spin" />
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

                {onUninstall && (
                  <CardActionButton
                    onClick={onUninstall}
                    disabled={isLoading}
                    title={uninstallFromLabel ?? t("common.uninstall")}
                    ariaLabel={uninstallFromLabel ?? t("common.uninstall")}
                    testId={`uninstall-skills-cli-${name}`}
                    danger
                    icon={<Trash2 className="size-4" />}
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
            installOrigin={installOrigin}
            originBadge={originBadge}
            usageBadge={footer ? undefined : usageBadge}
            isCentral={isCentral}
            platformBadge={platformBadge}
            projectBadge={projectBadge}
            publisher={footer ? undefined : publisher}
            updateStatus={updateStatus}
            inventoryFlags={inventoryFlags ?? undefined}
            inventorySkillId={inventorySkillId ?? undefined}
            tags={editableTags ? undefined : tags}
          />

          {/* Row 末: 彩色标签行（central 专用）— 收于 footer 上方，固定单行 */}
          {editableTags && (
            <div className="flex h-6 items-center overflow-hidden [&>div]:flex-nowrap">
              <CardTagEditor {...editableTags} />
            </div>
          )}

          {/* Row 4: Footer (central 方案C) or Platform toggle icons */}
          {footer ? (
            <UnifiedSkillCardFooter
              repoName={footer.repoName}
              repoColor={footer.repoColor}
              usageBadge={usageBadge}
              agents={platformIcons?.agents ?? []}
              linkedAgents={platformIcons?.linkedAgents ?? []}
              lockedAgentIds={platformIcons?.lockedAgentIds}
              skillId={platformIcons?.skillId}
              togglingAgentId={platformIcons?.togglingAgentId ?? null}
              skillName={name}
              onToggle={platformIcons?.onToggle}
            />
          ) : (
            hasPlatformIcons &&
            platformIcons &&
            targetAgents.length > 0 && (
              <div className="mt-auto pt-1">
                {isCompact && (
                  <span
                    data-testid={`skill-card-linked-summary-${platformIcons.skillId}`}
                    className={cn(
                      "inline-flex items-center gap-1 rounded-full border border-border/70 bg-muted/40 px-2 py-0.5 text-xs font-medium tabular-nums text-muted-foreground",
                      "group-hover/skill-card:hidden group-focus-within/skill-card:hidden",
                    )}
                  >
                    <Link2 className="size-3" aria-hidden />
                    {linkedTargetCount > 0
                      ? t("common.skillCardLinkedSummary", {
                          count: linkedTargetCount,
                        })
                      : t("common.skillCardLinkedSummaryNone")}
                  </span>
                )}
                <div
                  className={cn(
                    "flex items-center gap-1 flex-wrap",
                    isCompact &&
                      "hidden group-hover/skill-card:flex group-focus-within/skill-card:flex",
                  )}
                >
                  {targetAgents.map((agent) => (
                    <PlatformToggleIcon
                      key={agent.id}
                      agent={agent}
                      skillName={name}
                      isLinked={getPlatformTargetMemberIds(agent).some(
                        (agentId) => linkedAgentSet.has(agentId),
                      )}
                      isToggling={getPlatformTargetMemberIds(agent).some(
                        (agentId) =>
                          platformIcons.togglingAgentId === agentId,
                      )}
                      isLocked={getPlatformTargetMemberIds(agent).some(
                        (agentId) => lockedAgentSet.has(agentId),
                      )}
                      onToggle={() => {
                        const [agentId] =
                          getPlatformTargetInstallAgentIds(agent);
                        if (agentId) {
                          platformIcons.onToggle(
                            platformIcons.skillId,
                            agentId,
                          );
                        }
                      }}
                    />
                  ))}
                </div>
              </div>
            )
          )}
        </div>
      </div>
    </div>
  );
}

export const UnifiedSkillCard = memo(UnifiedSkillCardComponent);
UnifiedSkillCard.displayName = "UnifiedSkillCard";
