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
import { memo, useMemo, type MouseEventHandler, type ReactNode, type Ref } from "react";
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
import { useTextTruncation } from "@/hooks/useTextTruncation";
import type { AgentWithStatus, CentralSkillUpdateState, ClaudeSourceKind } from "@/types";
import { cn } from "@/lib/utils";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberIds,
} from "@/lib/platformTargetGroups";
import {
  selectSkillInventoryFlagsFromInventory,
  useUpdateCenterStore,
} from "@/stores/updateCenterStore";

// ─── Types ────────────────────────────────────────────────────────────────────

/**
 * 卡片视觉密度：
 * - `comfortable`（默认）：所有元素自适应、间距宽松；min-h 保底；描述 line-clamp-3 + fade mask
 * - `compact`：idle 隐藏动作 / 平台图标条收成"已链接 N"；hover 展开；描述 line-clamp-2 + fade mask
 * - `default`：旧别名，等同 `comfortable`，仅作向后兼容
 */
export type SkillCardDensity = "comfortable" | "compact" | "default";

export interface SkillCardCheckbox {
  checked: boolean;
  onChange: () => void;
}

export interface SkillCardPlatformIcons {
  agents: AgentWithStatus[];
  linkedAgents: string[];
  lockedAgentIds?: string[];
  skillId: string;
  onToggle: (skillId: string, agentId: string) => void;
  togglingAgentId: string | null;
}

/**
 * 可编辑标签行（central 专用）。
 * - tags：已赋标签（含可选 color）
 * - allTags：可选已存在标签（供 + 添加选择）
 * - onAdd：选中一个已存在 tag
 * - onCreate：输入新名创建并赋值
 * - onRemove：移除一个 tag
 */
export interface SkillCardEditableTags {
  tags: { id: string; name: string; color?: string | null }[];
  allTags: { id: string; name: string; color?: string | null }[];
  onAdd: (tagId: string) => void;
  onCreate: (name: string) => void;
  onRemove: (tagId: string) => void;
}

/** footer 分隔区（central 专用）：左 = repo 色块+名 + usage；右 = 平台点。 */
export interface SkillCardFooter {
  repoName?: string;
  repoColor?: string;
}

/** 所有场景共享的核心数据。 */
interface SkillCardCoreProps {
  name: string;
  description?: string;
  aiSummary?: string | null;
  className?: string;
}

/** 中央技能库卡片：全功能管理面（列表/网格两种模式，网格附 platformIcons + footer）。 */
export interface CentralSkillCardProps extends SkillCardCoreProps {
  variant: "central";
  /**
   * 中央库 skill id。设置后从 `useUpdateCenterStore` 查询 inventory 派生 badge
   * （platform duplicate / orphan）；网格模式下 `platformIcons.skillId` 兜底。
   */
  skillId?: string;
  checkbox: SkillCardCheckbox;
  /** 左侧状态竖条强度：warning=可更新，error=源缺失/错误；不传=无竖条。 */
  statusAccent?: "warning" | "error";
  /** 行 1 名称右侧的状态 chip 文案（如“可更新”/“源缺失”）；不传=不显示。 */
  statusChipLabel?: string;
  /** 只读 tags（无 editableTags 时由 SkillCardMeta 渲染）。 */
  tags?: { key: string; label: string }[];
  publisher?: string;
  /** 「近 30 天调用 N 次」徽章；仅当数值 > 0 时渲染。 */
  usageBadge?: number;
  updateStatus?: CentralSkillUpdateState & { isUpdating?: boolean };
  onDetail: MouseEventHandler<HTMLButtonElement>;
  onInstallTo: () => void;
  onUninstallFromPlatforms: () => void;
  onUpdateCentral: () => void;
  onDeleteFromCentral: () => void;
  detailButtonRef?: Ref<HTMLButtonElement>;
  editableTags?: SkillCardEditableTags;
  density?: SkillCardDensity;
  platformIcons?: SkillCardPlatformIcons;
  footer?: SkillCardFooter;
}

/** 平台技能视图卡片：某平台已安装技能（来源类型 + 装/卸该平台）。 */
export interface PlatformSkillCardProps extends SkillCardCoreProps {
  variant: "platform";
  sourceType: "symlink" | "copy" | "native";
  originKind: ClaudeSourceKind | null;
  isReadOnly: boolean;
  publisher?: string;
  /** 「近 30 天调用 N 次」徽章；仅当数值 > 0 时渲染。 */
  usageBadge?: number;
  /** 只读行（native 等）不出现多选框。 */
  checkbox?: SkillCardCheckbox;
  isLoading?: boolean;
  onDetail: MouseEventHandler<HTMLButtonElement>;
  onInstallTo?: () => void;
  onUninstallFromPlatform?: () => void;
  uninstallFromLabel: string;
  detailButtonRef?: Ref<HTMLButtonElement>;
}

/** 项目技能卡片：项目目录内已启用 agent 的技能（来源徽章 + 卸载）。 */
export interface ProjectSkillCardProps extends SkillCardCoreProps {
  variant: "project";
  sourceType?: "symlink" | "copy" | "native";
  originBadge: { kind: string; label: string };
  platformBadge: { id: string; name: string };
  onUninstallFromPlatform: () => void;
  uninstallFromLabel: string;
  isLoading?: boolean;
}

/** 导入候选卡片：Obsidian vault 导入场景（原 discover 场景簇）。 */
export interface ImportSkillCardProps extends SkillCardCoreProps {
  variant: "import";
  isCentral: boolean;
  platformBadge: { id: string; name: string };
  projectBadge?: string;
  onDetail: MouseEventHandler<HTMLButtonElement>;
  detailButtonRef?: Ref<HTMLButtonElement>;
  onInstallToCentral: () => void;
  onInstallToPlatform: () => void;
  isLoading?: boolean;
}

/** 技能市场卡片：远程技能浏览与安装（推荐 Tab 无安装动作）。 */
export interface MarketplaceSkillCardProps extends SkillCardCoreProps {
  variant: "marketplace";
  publisher?: string;
  tags?: { key: string; label: string }[];
  onDetail: MouseEventHandler<HTMLButtonElement>;
  onInstall?: () => void;
  installLabel?: string;
  isLoading?: boolean;
}

/** 集合成员卡片：集合内技能（详情 / 安装到平台 / 移出集合）。 */
export interface CollectionSkillCardProps extends SkillCardCoreProps {
  variant: "collection";
  onDetail: MouseEventHandler<HTMLButtonElement>;
  detailButtonRef?: Ref<HTMLButtonElement>;
  onInstallTo: () => void;
  onRemove: () => void;
}

/**
 * 唯一技能卡片实现的显式场景 interface：调用方声明 `variant` + 该场景的窄 props，
 * 场景间互斥的 props 在编译期被拒绝（判别联合 + excess property check）。
 */
export type UnifiedSkillCardProps =
  | CentralSkillCardProps
  | PlatformSkillCardProps
  | ProjectSkillCardProps
  | ImportSkillCardProps
  | MarketplaceSkillCardProps
  | CollectionSkillCardProps;

// ─── Internal render model ────────────────────────────────────────────────────

/** 模块私有渲染模型：各场景归一化后的扁平字段，渲染代码只面向它。 */
interface SkillCardModel {
  name: string;
  description?: string;
  aiSummary?: string | null;
  className?: string;
  skillId?: string;
  checkbox?: SkillCardCheckbox;
  isCentral?: boolean;
  platformBadge?: { id: string; name: string };
  projectBadge?: string;
  originBadge?: { kind: "central" | "project" | string; label: string };
  platformIcons?: SkillCardPlatformIcons;
  sourceType?: "symlink" | "copy" | "native";
  originKind?: ClaudeSourceKind | null;
  isReadOnly?: boolean;
  tags?: { key: string; label: string }[];
  publisher?: string;
  installLabel?: string;
  usageBadge?: number;
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
  isLoading?: boolean;
  detailButtonRef?: Ref<HTMLButtonElement>;
  density?: SkillCardDensity;
  statusAccent?: "warning" | "error";
  statusChipLabel?: string;
  editableTags?: SkillCardEditableTags;
  footer?: SkillCardFooter;
}

/** 场景 → 渲染模型的唯一映射点：每分支只拷贝该场景合法字段。 */
function toModel(props: UnifiedSkillCardProps): SkillCardModel {
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
  }
}

// ─── UnifiedSkillCard ─────────────────────────────────────────────────────────

function UnifiedSkillCardComponent(props: UnifiedSkillCardProps) {
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
    isLoading,
    detailButtonRef,
    density: rawDensity = "comfortable",
    statusAccent,
    statusChipLabel,
    editableTags,
    footer,
  } = toModel(props);

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
    onUninstallFromPlatforms ||
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

  return (
    <div
      className={cn(
        cardShellClass(checkbox?.checked),
        "group/skill-card relative overflow-hidden gap-2 p-3.5",
        isCompact ? "min-h-[168px]" : "min-h-[188px]",
        isLoading && "opacity-50",
        className
      )}
    >
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
          {/* Row 1: Name + icon actions */}
          <div className="flex items-center justify-between gap-2">
            {onDetail ? (
              <button
                ref={detailButtonRef}
                className="min-h-7 min-w-0 flex-1 truncate rounded-md text-left text-sm font-semibold tracking-[-0.01em] text-foreground underline-offset-4 transition-[color,box-shadow] hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
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
                  "flex shrink-0 items-center gap-0.5",
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
            <div className="flex h-5 items-center overflow-hidden [&>div]:flex-nowrap">
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

// ─── Card Shell Styles ────────────────────────────────────────────────────────

function cardShellClass(selected?: boolean): string {
  return cn(
    "central-skill-card-surface flex h-full flex-col rounded-xl bg-card",
    selected && "central-skill-card-selected bg-primary/[0.04]",
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
        "focus-ring inline-flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground transition-[scale,background-color,color] active:not-disabled:scale-[0.96] disabled:cursor-default disabled:opacity-50",
        danger
          ? "hover:bg-destructive/10 hover:text-destructive-text"
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
        <span className="mr-1.5 inline-flex align-baseline rounded-full border border-primary/15 bg-primary/8 px-1.5 py-0.5 text-xs font-medium leading-none text-primary-text">
          {label}
        </span>
      )}
      <p
        ref={ref}
        data-truncated={isTruncated ? "true" : "false"}
        title={text}
        className={cn(
          "text-pretty break-words text-xs leading-relaxed text-muted-foreground",
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
