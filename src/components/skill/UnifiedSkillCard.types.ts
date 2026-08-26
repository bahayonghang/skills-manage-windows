import type { MouseEventHandler, Ref } from "react";

import type {
  AgentWithStatus,
  CentralSkillUpdateState,
  ClaudeSourceKind,
  SkillsCliPlacement,
} from "@/types";

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
  /** Backend/view-model origin; CLI junctions are not Central. */
  installOrigin?: "central" | "standalone" | "skillsCli";
  isReadOnly: boolean;
  publisher?: string;
  /** 「近 30 天调用 N 次」徽章；仅当数值 > 0 时渲染。 */
  usageBadge?: number;
  /** 全历史名次。undefined = 未就绪不渲染；rank null = 无记录。 */
  lifetimeUsage?: { rank: number | null; count: number };
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

/** Skills CLI 全局技能卡片：dense-row 布局，卸载确认由页面完成。 */
export interface SkillsCliSkillCardProps extends SkillCardCoreProps {
  variant: "skillsCli";
  layout: "denseRow";
  path?: string | null;
  placements: readonly SkillsCliPlacement[];
  checkbox?: SkillCardCheckbox;
  updateAvailable?: boolean;
  onDetail: MouseEventHandler<HTMLButtonElement>;
  onManageLinks?: () => void;
  onUninstall: () => void;
  isLoading?: boolean;
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
  | CollectionSkillCardProps
  | SkillsCliSkillCardProps;
