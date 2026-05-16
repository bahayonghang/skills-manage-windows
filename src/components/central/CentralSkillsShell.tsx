import type { ComponentProps, ReactNode } from "react";
import {
  ActivityIcon,
  Download,
  RefreshCw,
} from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import type { CentralInstalledSkillsQuickFilterProps } from "@/components/central/CentralInstalledSkillsQuickFilter";
import {
  ToolbarSortMenu,
  ToolbarViewMenu,
  ToolbarMoreMenu,
} from "@/components/central/CentralSkillsShellMenus";
import {
  CentralProgressTopLine,
} from "@/components/central/CentralProgressTopLine";
import { countActiveCentralTasks } from "@/components/central/centralTaskCenterHelpers";
import {
  TaskCenterDrawer,
} from "@/components/central/TaskCenterDrawer";
import { BulkActionBar } from "@/components/central/BulkActionBar";
import { CategorizeDrawer } from "@/components/central/CategorizeDrawer";
import type { CentralSkillCategorizePanelProps } from "@/components/central/CentralSkillCategorizePanel";
import { CentralSkillDialogs } from "@/components/central/CentralSkillDialogs";
import { CentralSkillListContent } from "@/components/central/CentralSkillListContent";
import { CentralSearchBar } from "@/components/central/CentralSearchBar";
import { CentralSidebar } from "@/components/central/CentralSidebar";
import { cn } from "@/lib/utils";
import { useMemo } from "react";
import { groupSkillsByMode } from "@/lib/centralGrouping";
import type { FacetCounts } from "@/lib/centralFacetCounts";
import type { CentralViewState, GroupByMode } from "@/lib/centralViewState";
import { CentralGroupedSkillList } from "./CentralGroupedSkillList";
import type {
  CentralSortDirection,
  CentralSortField,
} from "@/pages/centralSkillsViewModel";
import type {
  AiTagJob,
  CentralQueryAst,
  CentralSkillUpdateJob,
  SkillRepositoryWithStats,
  SkillTag,
  SkillportStatePortabilityJob,
  TagGroup,
} from "@/types";

/**
 * Central Skills V2 Shell。
 *
 * 设计原则：
 * - **复用** V1 的 list / dialogs / progress / categorize 渲染（不重写已稳定的部分）
 * - **替换** sidebar (CentralSidebar) 与 search bar (CentralSearchBar)
 * - 通过 viewState / setViewState 驱动新视图状态（多选 + URL state 同步）
 */

type ListContentProps = Omit<ComponentProps<typeof CentralSkillListContent>, "t">;
type CategorizePanelProps = Omit<CentralSkillCategorizePanelProps, "t">;
type DialogProps = Omit<ComponentProps<typeof CentralSkillDialogs>, "t">;
type InstalledSkillsFilterProps = Omit<CentralInstalledSkillsQuickFilterProps, "t">;

export interface CentralSkillsShellProps {
  t: TFunction;

  // Layout / chrome
  centralSkillsDir: string;
  isCheckingUpdates: boolean;
  filterSidebarWidth: number;
  startFilterSidebarResize: (event: React.PointerEvent<HTMLElement>) => void;
  handleFilterSidebarResizeKeyDown: (event: React.KeyboardEvent<HTMLElement>) => void;

  // V2 view state（来自 useCentralViewState 或 Shell 外维护的 hook）
  viewState: CentralViewState;
  setViewState: (next: CentralViewState) => void;
  queryAst: CentralQueryAst;

  // Facet 数据
  facetCounts: FacetCounts;
  repositories: readonly SkillRepositoryWithStats[];
  tags: readonly SkillTag[];
  /** 标签分组（M3）。 */
  tagGroups?: readonly TagGroup[];
  /** 分配 tag 到分组（M6）。 */
  onAssignTagToGroup?: (tagId: string, groupId: string | null) => void;

  // 排序选项（与 V1 保持一致）
  sortFieldOptions: Array<{ value: CentralSortField; label: string }>;
  sortDirectionOptions: Array<{ value: CentralSortDirection; label: string }>;
  /** Group-by 选项（M4）。 */
  groupByOptions?: Array<{ value: GroupByMode; label: string }>;

  // V1 渲染单元（复用）
  installedSkillsFilter: InstalledSkillsFilterProps;
  listContent: ListContentProps;
  categorizePanel: CategorizePanelProps;
  dialogs: DialogProps;

  // Bulk + categorize 抽屉触发（M5）
  bulkBar: {
    selectedCount: number;
    isInstalling: boolean;
    isDeleting: boolean;
    isAiBusy: boolean;
    aiTaggingAvailable: boolean;
    onBatchInstall: () => void;
    onBatchDelete: () => void;
    onClearSelection: () => void;
  };
  categorizeDrawer: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onOpenManual: () => void;
    onOpenAiSuggest: () => void;
  };

  // 任务中心（M6）：顶部 1px 进度线 + 抽屉聚合三类任务
  taskCenter: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    aiTagJob: AiTagJob;
    updateJob: CentralSkillUpdateJob;
    portabilityJob: SkillportStatePortabilityJob;
    onCancelAiTag: () => void;
    onCancelUpdate: () => void;
    onCancelPortability: () => void;
    onViewAiReviews: () => void;
  };

  // Header 操作
  setIsGitHubImportOpen: (open: boolean) => void;
  setIsPlatformManageOpen: (open: boolean) => void;
  setIsPortabilityOpen: (open: boolean) => void;
  onUpdateSkills: (skillIds: string[]) => void;
  updateAvailableSkillCount: number;
  updateButton: { disabled: boolean; label: string; targetSkillIds: string[] };
  checkButton: { label: string; disabled: boolean; onClick: () => void };

  /** 仓库删除（M4 回填）。 */
  onDeleteRepository?: (repository: SkillRepositoryWithStats) => void;

  /** 命令面板触发。 */
  onOpenPalette?: () => void;
  /** Sidebar 顶部插槽。 */
  savedViewsSlot?: ReactNode;
}

export function CentralSkillsShell(props: CentralSkillsShellProps) {
  const {
    t,
    centralSkillsDir,
    isCheckingUpdates,
    filterSidebarWidth,
    startFilterSidebarResize,
    handleFilterSidebarResizeKeyDown,
    viewState,
    setViewState,
    queryAst,
    facetCounts,
    repositories,
    tags,
    tagGroups,
    onAssignTagToGroup,
    sortFieldOptions,
    sortDirectionOptions,
    groupByOptions,
    installedSkillsFilter,
    listContent,
    categorizePanel,
    dialogs,
    bulkBar,
    categorizeDrawer,
    taskCenter,
    setIsGitHubImportOpen,
    setIsPlatformManageOpen,
    setIsPortabilityOpen,
    onUpdateSkills,
    updateAvailableSkillCount,
    updateButton,
    checkButton,
    onDeleteRepository,
    onOpenPalette,
    savedViewsSlot,
  } = props;

  const handleToggleRepo = (repoId: string) => {
    const next = toggleId(viewState.repos, repoId);
    setViewState({ ...viewState, repos: next });
  };

  const handleToggleTag = (tagId: string) => {
    const next = toggleId(viewState.tags, tagId);
    setViewState({ ...viewState, tags: next });
  };

  const handleClearAll = () => {
    setViewState({ ...viewState, repos: [], tags: [] });
  };

  const handleQueryChange = (q: string) => {
    setViewState({ ...viewState, q });
  };

  const handleQueryClear = () => {
    setViewState({ ...viewState, q: "" });
  };

  const handleRemoveFilter = (filterIndex: number) => {
    const removed = queryAst.filters[filterIndex];
    if (!removed) return;
    const tokenToStrip = filterToToken(removed);
    if (!tokenToStrip) return;
    const nextQ = stripFirstOccurrence(viewState.q, tokenToStrip);
    setViewState({ ...viewState, q: nextQ });
  };

  const handleRemoveRepo = (repoId: string) => {
    setViewState({
      ...viewState,
      repos: viewState.repos.filter((id) => id !== repoId),
    });
  };

  const handleRemoveTag = (tagId: string) => {
    setViewState({
      ...viewState,
      tags: viewState.tags.filter((id) => id !== tagId),
    });
  };

  const activeTaskCount = countActiveCentralTasks(
    taskCenter.aiTagJob,
    taskCenter.updateJob,
    taskCenter.portabilityJob
  );

  return (
    <div className="relative flex flex-col h-full" data-shell="v2">
      <CentralProgressTopLine
        aiTagJob={taskCenter.aiTagJob}
        updateJob={taskCenter.updateJob}
        portabilityJob={taskCenter.portabilityJob}
      />
      {/* Header ─────────────────────────────────────────────────────── */}
      <div className="border-b border-border px-6 py-3 flex items-center justify-between gap-4">
        <div className="min-w-0">
          <h1 className="text-xl font-semibold leading-tight">{t("central.title")}</h1>
          <p className="text-[11px] text-muted-foreground/70 mt-0.5 truncate" title={centralSkillsDir}>
            {centralSkillsDir}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {/* 主 CTA：检查更新 ────────────────────────────────────── */}
          <Button
            variant="default"
            onClick={checkButton.onClick}
            disabled={checkButton.disabled}
            data-testid="central-check-updates"
          >
            <RefreshCw className={cn("size-3.5", isCheckingUpdates && "animate-spin")} />
            {checkButton.label}
            {updateAvailableSkillCount > 0 && (
              <span
                data-testid="central-update-count-chip"
                className="ml-1.5 inline-flex items-center rounded-full bg-amber-500/15 px-1.5 py-0.5 text-[10px] font-semibold text-amber-700 ring-1 ring-amber-500/30 dark:text-amber-300 dark:bg-amber-500/20"
              >
                +{updateAvailableSkillCount}
              </span>
            )}
          </Button>

          {/* 可更新数 > 0 时露出独立的批量更新按钮 ────────────── */}
          {updateAvailableSkillCount > 0 && (
            <Button
              variant="outline"
              onClick={() => onUpdateSkills(updateButton.targetSkillIds)}
              disabled={updateButton.disabled}
            >
              <Download className="size-3.5" />
              {updateButton.label}
            </Button>
          )}

          {/* 更多 dropdown：平台管理 / 状态导出 / GitHub 导入 ───── */}
          <ToolbarMoreMenu
            t={t}
            activeTaskCount={activeTaskCount}
            onOpenTaskCenter={() => taskCenter.onOpenChange(true)}
            onOpenPlatformManage={() => setIsPlatformManageOpen(true)}
            onOpenPortability={() => setIsPortabilityOpen(true)}
            onOpenGitHubImport={() => setIsGitHubImportOpen(true)}
          />
        </div>
      </div>

      {/* Search + toolbar ──────────────────────────────────────────── */}
      <div className="px-6 py-3 border-b border-border">
        <div className="flex items-center gap-2">
          <div className="flex-1 min-w-0">
            <CentralSearchBar
              t={t}
              value={viewState.q}
              onChange={handleQueryChange}
              queryAst={queryAst}
              onRemoveFilter={handleRemoveFilter}
              onClear={handleQueryClear}
              onOpenPalette={onOpenPalette}
              selectedRepoIds={viewState.repos}
              selectedTagIds={viewState.tags}
              repositories={repositories}
              tags={tags}
              onRemoveRepoFilter={handleRemoveRepo}
              onRemoveTagFilter={handleRemoveTag}
              installedFilterValue={installedSkillsFilter.value}
              installedFilterMatchCount={installedSkillsFilter.filteredCount}
              availableInstallAgents={installedSkillsFilter.availableInstallAgents}
              onClearInstalledFilter={installedSkillsFilter.onClear}
            />
          </div>
          <ToolbarSortMenu
            t={t}
            sortField={viewState.sortField}
            sortDir={viewState.sortDir}
            sortFieldOptions={sortFieldOptions}
            sortDirectionOptions={sortDirectionOptions}
            onChange={(next) =>
              setViewState({
                ...viewState,
                sortField: next.field,
                sortDir: next.dir,
              })
            }
          />
          <ToolbarViewMenu
            t={t}
            viewState={viewState}
            setViewState={setViewState}
            groupByOptions={groupByOptions}
            installedSkillsFilter={installedSkillsFilter}
          />
          {activeTaskCount > 0 && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-9 shrink-0 gap-1.5"
              onClick={() => taskCenter.onOpenChange(true)}
              aria-label={t("central.taskCenterOpenLabel")}
              data-testid="central-toolbar-task-center-chip"
            >
              <ActivityIcon className="size-3.5 animate-pulse text-primary" />
              <span className="text-xs">
                {t("central.taskCenterRunningChip", { count: activeTaskCount })}
              </span>
            </Button>
          )}
        </div>
      </div>

      {/* Body: sidebar + list + categorize ───────────────────────────── */}
      <div className="flex min-h-0 flex-1">
        <CentralSidebar
          t={t}
          width={filterSidebarWidth}
          startResize={startFilterSidebarResize}
          handleResizeKeyDown={handleFilterSidebarResizeKeyDown}
          facetCounts={facetCounts}
          repositories={repositories}
          tags={tags}
          tagGroups={tagGroups}
          onAssignTagToGroup={onAssignTagToGroup}
          onDeleteRepository={onDeleteRepository}
          selectedRepos={viewState.repos}
          selectedTags={viewState.tags}
          onToggleRepo={handleToggleRepo}
          onToggleTag={handleToggleTag}
          onClearAll={handleClearAll}
          onSelectSmartView={() => {
            /* M1：智能视图除 all 都映射到 tag facet；all 在 onClearAll 中处理 */
          }}
          savedViewsSlot={savedViewsSlot}
        />
        {viewState.group === "none" ? (
          <CentralSkillListContent {...listContent} t={t} />
        ) : (
          <GroupedListView listContent={listContent} group={viewState.group} t={t} />
        )}
      </div>

      <CentralSkillDialogs {...dialogs} t={t} />

      <BulkActionBar
        selectedCount={bulkBar.selectedCount}
        isInstalling={bulkBar.isInstalling}
        isDeleting={bulkBar.isDeleting}
        isAiBusy={bulkBar.isAiBusy}
        aiTaggingAvailable={bulkBar.aiTaggingAvailable}
        t={t}
        onBatchInstall={bulkBar.onBatchInstall}
        onBatchDelete={bulkBar.onBatchDelete}
        onOpenCategorize={categorizeDrawer.onOpenManual}
        onOpenAiSuggest={categorizeDrawer.onOpenAiSuggest}
        onClearSelection={bulkBar.onClearSelection}
      />
      <CategorizeDrawer
        open={categorizeDrawer.open}
        onOpenChange={categorizeDrawer.onOpenChange}
        t={t}
        panelProps={categorizePanel}
      />
      <TaskCenterDrawer
        open={taskCenter.open}
        onOpenChange={taskCenter.onOpenChange}
        t={t}
        aiTagJob={taskCenter.aiTagJob}
        updateJob={taskCenter.updateJob}
        portabilityJob={taskCenter.portabilityJob}
        onCancelAiTag={taskCenter.onCancelAiTag}
        onCancelUpdate={taskCenter.onCancelUpdate}
        onCancelPortability={taskCenter.onCancelPortability}
        onViewAiReviews={taskCenter.onViewAiReviews}
      />
    </div>
  );
}

interface GroupedListViewProps {
  listContent: ListContentProps;
  group: GroupByMode;
  t: TFunction;
}

function GroupedListView({ listContent, group, t }: GroupedListViewProps) {
  const groups = useMemo(
    () =>
      groupSkillsByMode(listContent.sortedSkills, group, {
        updateStatuses: listContent.updateStatuses,
        labels: {
          all: t("central.v2.groupByAll"),
          uncategorized: t("central.v2.groupByUncategorized"),
          unknownOwner: t("central.v2.groupByUnknownOwner"),
          localRepos: t("central.v2.groupByLocalRepos"),
          statusUpToDate: t("central.v2.groupByStatusUpToDate"),
          statusNeedsUpdate: t("central.v2.groupByStatusNeedsUpdate"),
          statusUnknown: t("central.v2.groupByStatusUnknown"),
        },
      }),
    [group, listContent.sortedSkills, listContent.updateStatuses, t],
  );

  return (
    <CentralGroupedSkillList
      contentRef={listContent.contentRef}
      groups={groups}
      availableInstallAgents={listContent.availableInstallAgents}
      selectedSkillIdSet={listContent.selectedSkillIdSet}
      updateStatuses={listContent.updateStatuses}
      updatingSkillIds={listContent.updatingSkillIds}
      togglingAgentId={listContent.togglingAgentId}
      setDetailButtonRef={listContent.setDetailButtonRef}
      onToggleSelection={listContent.onToggleSelection}
      onDetail={listContent.onDetail}
      onInstallTo={listContent.onInstallTo}
      onUpdateCentral={listContent.onUpdateCentral}
      onDelete={listContent.onDelete}
      onTogglePlatform={listContent.onTogglePlatform}
      t={t}
    />
  );
}

// ─── Utilities ─────────────────────────────────────────────────────
function toggleId(arr: readonly string[], id: string): string[] {
  const set = new Set(arr);
  if (set.has(id)) set.delete(id);
  else set.add(id);
  return [...set];
}

function filterToToken(filter: CentralQueryAst["filters"][number]): string | null {
  const prefix = filter.negated ? "-" : "";
  if (filter.kind === "tag" || filter.kind === "repo" || filter.kind === "owner" || filter.kind === "source" || filter.kind === "has" || filter.kind === "platform") {
    return `${prefix}${filter.kind}:${filter.value}`;
  }
  if (filter.kind === "created" || filter.kind === "updated") {
    return `${prefix}${filter.kind}:${filter.op}${filter.value}`;
  }
  return null;
}

function stripFirstOccurrence(query: string, token: string): string {
  const tokens = query.split(/\s+/).filter((tok) => tok.length > 0);
  const idx = tokens.findIndex((tok) => tok.toLowerCase() === token.toLowerCase());
  if (idx < 0) return query;
  tokens.splice(idx, 1);
  return tokens.join(" ");
}
