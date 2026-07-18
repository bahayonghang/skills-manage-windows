import type { ComponentProps, ReactNode } from "react";
import {
  ActivityIcon,
  Download,
  ListChecks,
  MapPin,
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
import { SkillImportLauncher } from "@/components/central/SkillImportLauncher";
import { LocalArchiveImportWizard } from "@/components/central/LocalArchiveImportWizard";
import { CentralProgressTopLine } from "@/components/central/CentralProgressTopLine";
import { countActiveCentralTasks } from "@/components/central/centralTaskCenterHelpers";
import { TaskCenterDrawer } from "@/components/central/TaskCenterDrawer";
import { BulkActionBar } from "@/components/central/BulkActionBar";
import { CategorizeDrawer } from "@/components/central/CategorizeDrawer";
import type { CentralSkillCategorizePanelProps } from "@/components/central/CentralSkillCategorizePanel";
import { CentralSkillDialogs } from "@/components/central/CentralSkillDialogs";
import { CentralSkillListContent } from "@/components/central/CentralSkillListContent";
import { CentralSearchBar } from "@/components/central/CentralSearchBar";
import { CentralSidebar } from "@/components/central/CentralSidebar";
import {
  CentralTopFilters,
  type SourceFilterValue,
} from "@/components/central/CentralTopFilters";
import { useUpdateCenterStore } from "@/stores/updateCenterStore";
import { cn } from "@/lib/utils";
import { useMemo, useState } from "react";
import { groupSkillsByMode } from "@/lib/centralGrouping";
import type { FacetCounts } from "@/lib/centralFacetCounts";
import { sanitizeSelectedTagIds } from "@/lib/centralTags";
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
} from "@/types";

/**
 * Central Skills V2 Shell。
 *
 * 设计原则：
 * - **复用** V1 的 list / dialogs / progress / categorize 渲染（不重写已稳定的部分）
 * - **替换** sidebar (CentralSidebar) 与 search bar (CentralSearchBar)
 * - 通过 viewState / setViewState 驱动新视图状态（repo 单选、tag 多选 + URL state 同步）
 */

type ListContentProps = Omit<
  ComponentProps<typeof CentralSkillListContent>,
  "t" | "selectedCount"
>;
type CategorizePanelProps = Omit<CentralSkillCategorizePanelProps, "t">;
type DialogProps = Omit<ComponentProps<typeof CentralSkillDialogs>, "t">;
type InstalledSkillsFilterProps = Omit<
  CentralInstalledSkillsQuickFilterProps,
  "t"
>;

export interface CentralSkillsShellProps {
  t: TFunction;

  // Layout / chrome
  centralSkillsDir: string;
  isCheckingUpdates: boolean;
  filterSidebarWidth: number;
  startFilterSidebarResize: (event: React.PointerEvent<HTMLElement>) => void;
  handleFilterSidebarResizeKeyDown: (
    event: React.KeyboardEvent<HTMLElement>,
  ) => void;

  // V2 view state（来自 useCentralViewState 或 Shell 外维护的 hook）
  viewState: CentralViewState;
  setViewState: (next: CentralViewState) => void;
  queryAst: CentralQueryAst;

  // Facet 数据
  facetCounts: FacetCounts;
  repositories: readonly SkillRepositoryWithStats[];
  /** repo.id → 可更新 skill 数（侧栏 repo 行角标）。 */
  repoUpdateCounts?: Record<string, number>;
  tags: readonly SkillTag[];
  /** 顶部筛选「更多▾」内的 Tag Groups 管理 UI。 */
  topFiltersTagGroups?: ReactNode;

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
    isUninstalling: boolean;
    isDeleting: boolean;
    isAiBusy: boolean;
    aiTaggingAvailable: boolean;
    onBatchInstall: () => void;
    onBatchUninstall: () => void;
    onBatchDelete: () => void;
    onClearSelection: () => void;
  };
  selectionControls: {
    selectedCount: number;
    currentResultCount: number;
    onSelectCurrentResults: () => void;
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
  centralStoreLocation: {
    disabled: boolean;
    onOpen: () => void;
    disabledReason?: string;
  };
  setIsGitHubImportOpen: (open: boolean) => void;
  setIsPlatformManageOpen: (open: boolean) => void;
  setIsPortabilityOpen: (open: boolean) => void;
  onUpdateSkills: (skillIds: string[]) => void;
  updateAvailableSkillCount: number;
  updateButton: { disabled: boolean; label: string; targetSkillIds: string[] };
  checkModeControl?: ReactNode;
  checkButton: { label: string; disabled: boolean; onClick: () => void };

  /** 仓库删除（M4 回填）。 */
  onDeleteRepository?: (repository: SkillRepositoryWithStats) => void;
  onToggleRepositoryPin?: (repository: SkillRepositoryWithStats) => void;

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
    repoUpdateCounts,
    tags,
    topFiltersTagGroups,
    sortFieldOptions,
    sortDirectionOptions,
    groupByOptions,
    installedSkillsFilter,
    listContent,
    categorizePanel,
    dialogs,
    bulkBar,
    selectionControls,
    categorizeDrawer,
    taskCenter,
    centralStoreLocation,
    setIsGitHubImportOpen,
    setIsPlatformManageOpen,
    setIsPortabilityOpen,
    onUpdateSkills,
    updateAvailableSkillCount,
    updateButton,
    checkModeControl,
    checkButton,
    onDeleteRepository,
    onToggleRepositoryPin,
    onOpenPalette,
    savedViewsSlot,
  } = props;

  const openUpdateCenter = useUpdateCenterStore((s) => s.openDialog);
  const [isLocalArchiveImportOpen, setIsLocalArchiveImportOpen] = useState(false);
  const selectedTagIds = useMemo(
    () =>
      tags.length > 0 ? sanitizeSelectedTagIds(viewState.tags, tags) : viewState.tags,
    [tags, viewState.tags],
  );
  const updateCenterRefreshContext = useMemo(() => {
    const selectedRepoId =
      viewState.repos.length === 1 ? viewState.repos[0] : null;
    const selectedRepo = selectedRepoId
      ? repositories.find((repo) => repo.id === selectedRepoId)
      : null;
    const repositoryIds =
      selectedRepo?.source_type === "github" && !selectedRepo.is_unknown
        ? [selectedRepo.id]
        : [];
    return {
      repositoryIds,
      skillIds: listContent.sortedSkills.map((skill) => skill.id),
    };
  }, [listContent.sortedSkills, repositories, viewState.repos]);

  const handleToggleRepo = (repoId: string) => {
    const next = viewState.repos[0] === repoId ? [] : [repoId];
    setViewState({ ...viewState, repos: next });
  };

  const handleToggleTag = (tagId: string) => {
    const next = toggleId(selectedTagIds, tagId);
    setViewState({ ...viewState, tags: next });
  };

  // 来源筛选走 query AST 的 source: token（与 chip 渲染共享）。
  const sourceFilter = queryAst.filters.find(
    (f) => f.kind === "source" && !f.negated,
  );
  const activeSource: SourceFilterValue | null =
    sourceFilter?.kind === "source" ? sourceFilter.value : null;

  const handleToggleSource = (value: SourceFilterValue | null) => {
    let q = viewState.q;
    for (const f of queryAst.filters) {
      if (f.kind === "source") {
        const tok = filterToToken(f);
        if (tok) q = stripFirstOccurrence(q, tok);
      }
    }
    const nextQ = value ? `${q} source:${value}`.trim() : q.trim();
    setViewState({ ...viewState, q: nextQ });
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
      tags: selectedTagIds.filter((id) => id !== tagId),
    });
  };

  const activeTaskCount = countActiveCentralTasks(
    taskCenter.aiTagJob,
    taskCenter.updateJob,
    taskCenter.portabilityJob,
  );

  return (
    <div className="relative flex flex-col h-full" data-shell="v2">
      <CentralProgressTopLine
        aiTagJob={taskCenter.aiTagJob}
        updateJob={taskCenter.updateJob}
        portabilityJob={taskCenter.portabilityJob}
      />
      {/* Header ─────────────────────────────────────────────────────── */}
      <div className="flex flex-col items-start gap-3 border-b border-border/65 bg-background/95 px-4 py-3 shadow-[0_1px_0_color-mix(in_srgb,var(--foreground)_4%,transparent)] sm:px-6 xl:flex-row xl:items-center xl:justify-between">
        <div className="w-full min-w-0 xl:w-auto">
          <div className="flex items-center gap-2">
            <h1 className="text-balance text-xl font-semibold leading-tight">
              {t("central.title")}
            </h1>
            <span
              data-testid="central-result-count"
              className="shrink-0 rounded-full bg-muted/70 px-2 py-0.5 text-xs font-medium tabular-nums text-muted-foreground ring-1 ring-border/60"
            >
              {listContent.sortedSkills.length}
            </span>
          </div>
          <div className="mt-0.5 flex min-w-0 items-center gap-2">
            <p
              className="truncate text-ui-meta text-muted-foreground"
              title={centralSkillsDir}
            >
              {centralSkillsDir}
            </p>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-7 shrink-0 gap-1 rounded-lg px-2 text-ui-meta transition-[scale,background-color,color] active:scale-[0.96]"
              disabled={centralStoreLocation.disabled}
              onClick={centralStoreLocation.onOpen}
              title={centralStoreLocation.disabledReason}
              data-testid="central-store-location-open"
            >
              <MapPin className="size-3" aria-hidden="true" />
              {t("central.storeLocation.changeButton")}
            </Button>
          </div>
        </div>
        <div className="flex w-full min-w-0 flex-col items-stretch gap-2 min-[521px]:flex-row min-[521px]:flex-wrap min-[521px]:items-center xl:w-auto xl:justify-end">
          {/* 统一入口：添加技能（GitHub / 本地 ZIP）───────────── */}
          <SkillImportLauncher
            t={t}
            isRemoteTarget={dialogs.activeTarget.kind !== "local"}
            onOpenIntent={(intent) => {
              if (intent === "github") {
                setIsGitHubImportOpen(true);
              } else if (intent === "local_zip") {
                setIsLocalArchiveImportOpen(true);
              }
            }}
          />

          {/* 入口：更新中心（聚合可更新 / 新增 / 已删除 / 平台冗余）─── */}
          <Button
            variant="outline"
            className="h-9 rounded-xl pl-3 pr-3.5 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
            onClick={() =>
              openUpdateCenter(undefined, updateCenterRefreshContext)
            }
            data-testid="central-open-update-center"
          >
            <ListChecks className="size-3.5" />
            {t("central.updateCenter.openButton")}
          </Button>

          {checkModeControl}

          {/* 主 CTA：检查更新 ────────────────────────────────────── */}
          <Button
            variant="default"
            className="h-9 rounded-xl pl-3 pr-3.5 shadow-[0_0_0_1px_color-mix(in_srgb,var(--primary)_22%,transparent),0_8px_18px_-14px_color-mix(in_srgb,var(--primary)_70%,transparent)] transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
            onClick={checkButton.onClick}
            disabled={checkButton.disabled}
            data-testid="central-check-updates"
          >
            <RefreshCw
              className={cn("size-3.5", isCheckingUpdates && "animate-spin")}
            />
            {checkButton.label}
            {updateAvailableSkillCount > 0 && (
              <span
                data-testid="central-update-count-chip"
                className="ml-1.5 inline-flex items-center rounded-full bg-warning/15 px-1.5 py-0.5 text-ui-micro font-semibold text-warning-foreground ring-1 ring-warning/30"
              >
                +{updateAvailableSkillCount}
              </span>
            )}
          </Button>

          {/* 可更新数 > 0 时露出独立的批量更新按钮 ────────────── */}
          {updateAvailableSkillCount > 0 && (
            <Button
              variant="outline"
              className="h-9 rounded-xl pl-3 pr-3.5 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
              onClick={() => onUpdateSkills(updateButton.targetSkillIds)}
              disabled={updateButton.disabled}
            >
              <Download className="size-3.5" />
              {updateButton.label}
            </Button>
          )}

          {/* 更多 dropdown：任务中心 / 平台管理 / 状态导入导出 ─────── */}
          <ToolbarMoreMenu
            t={t}
            activeTaskCount={activeTaskCount}
            onOpenTaskCenter={() => taskCenter.onOpenChange(true)}
            onOpenPlatformManage={() => setIsPlatformManageOpen(true)}
            onOpenPortability={() => setIsPortabilityOpen(true)}
          />
        </div>
      </div>

      {/* Search + toolbar ──────────────────────────────────────────── */}
      <div className="border-b border-border/60 bg-background/80 px-4 py-3 shadow-[0_1px_0_color-mix(in_srgb,var(--foreground)_3%,transparent)] sm:px-6">
        <div className="flex flex-wrap items-center gap-2">
          <div className="min-w-[240px] flex-1">
            <CentralSearchBar
              t={t}
              value={viewState.q}
              onChange={handleQueryChange}
              queryAst={queryAst}
              onRemoveFilter={handleRemoveFilter}
              onClear={handleQueryClear}
              onOpenPalette={onOpenPalette}
              selectedRepoIds={viewState.repos}
              selectedTagIds={selectedTagIds}
              repositories={repositories}
              tags={tags}
              onRemoveRepoFilter={handleRemoveRepo}
              onRemoveTagFilter={handleRemoveTag}
              installedFilterValue={installedSkillsFilter.value}
              installedFilterMatchCount={installedSkillsFilter.filteredCount}
              availableInstallAgents={
                installedSkillsFilter.availableInstallAgents
              }
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
              className="h-9 shrink-0 gap-1.5 rounded-xl transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
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
          {selectionControls.selectedCount > 0 && (
            <div
              data-testid="central-selection-summary"
              className="flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground"
            >
              <span className="font-medium text-foreground">
                {t("central.selectionInlineSummary", {
                  count: selectionControls.selectedCount,
                })}
              </span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs"
                disabled={selectionControls.currentResultCount === 0}
                onClick={selectionControls.onSelectCurrentResults}
                data-testid="central-select-current-results"
              >
                {t("central.selectCurrentResults")}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs"
                onClick={selectionControls.onClearSelection}
                data-testid="central-clear-selection"
              >
                {t("central.clearSelection")}
              </Button>
            </div>
          )}
        </div>
      </div>

      <CentralTopFilters
        t={t}
        tags={tags}
        selectedTagIds={selectedTagIds}
        onToggleTag={handleToggleTag}
        facetCounts={facetCounts}
        activeSource={activeSource}
        onToggleSource={handleToggleSource}
        tagGroupsSlot={topFiltersTagGroups}
      />

      {/* Body: sidebar + list + categorize ───────────────────────────── */}
      <div className="flex min-h-0 min-w-0 flex-1">
        <CentralSidebar
          t={t}
          width={filterSidebarWidth}
          startResize={startFilterSidebarResize}
          handleResizeKeyDown={handleFilterSidebarResizeKeyDown}
          facetCounts={facetCounts}
          repositories={repositories}
          repoUpdateCounts={repoUpdateCounts}
          onDeleteRepository={onDeleteRepository}
          onToggleRepositoryPin={onToggleRepositoryPin}
          onSyncNewSource={() => setIsGitHubImportOpen(true)}
          selectedRepos={viewState.repos}
          selectedTags={selectedTagIds}
          onToggleRepo={handleToggleRepo}
          onToggleTag={handleToggleTag}
          onClearAll={handleClearAll}
          onSelectSmartView={() => {
            /* M1：智能视图除 all 都映射到 tag facet；all 在 onClearAll 中处理 */
          }}
          savedViewsSlot={savedViewsSlot}
        />
        {viewState.group === "none" ? (
          <CentralSkillListContent
            {...listContent}
            selectedCount={bulkBar.selectedCount}
            t={t}
          />
        ) : (
          <GroupedListView
            listContent={listContent}
            selectedCount={bulkBar.selectedCount}
            group={viewState.group}
            t={t}
          />
        )}
      </div>

      <CentralSkillDialogs {...dialogs} t={t} />

      <LocalArchiveImportWizard
        open={isLocalArchiveImportOpen}
        onOpenChange={setIsLocalArchiveImportOpen}
        t={t}
        onAfterImportSuccess={async () => {
          await dialogs.loadCentralSkills();
        }}
      />

      <BulkActionBar
        selectedCount={bulkBar.selectedCount}
        isInstalling={bulkBar.isInstalling}
        isUninstalling={bulkBar.isUninstalling}
        isDeleting={bulkBar.isDeleting}
        isAiBusy={bulkBar.isAiBusy}
        aiTaggingAvailable={bulkBar.aiTaggingAvailable}
        t={t}
        onBatchInstall={bulkBar.onBatchInstall}
        onBatchUninstall={bulkBar.onBatchUninstall}
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
  selectedCount: number;
  group: GroupByMode;
  t: TFunction;
}

function GroupedListView({
  listContent,
  selectedCount,
  group,
  t,
}: GroupedListViewProps) {
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
      selectedCount={selectedCount}
      selectedSkillIdSet={listContent.selectedSkillIdSet}
      updateStatuses={listContent.updateStatuses}
      updatingSkillIds={listContent.updatingSkillIds}
      togglingAgentId={listContent.togglingAgentId}
      setDetailButtonRef={listContent.setDetailButtonRef}
      onToggleSelection={listContent.onToggleSelection}
      onDetail={listContent.onDetail}
      onInstallTo={listContent.onInstallTo}
      onUninstallFromPlatforms={listContent.onUninstallFromPlatforms}
      onUpdateCentral={listContent.onUpdateCentral}
      onDelete={listContent.onDelete}
      onTogglePlatform={listContent.onTogglePlatform}
      onAddSkillTag={listContent.onAddSkillTag}
      onCreateSkillTag={listContent.onCreateSkillTag}
      onRemoveSkillTag={listContent.onRemoveSkillTag}
      tags={listContent.tags}
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

function filterToToken(
  filter: CentralQueryAst["filters"][number],
): string | null {
  const prefix = filter.negated ? "-" : "";
  if (
    filter.kind === "tag" ||
    filter.kind === "repo" ||
    filter.kind === "owner" ||
    filter.kind === "source" ||
    filter.kind === "has" ||
    filter.kind === "platform"
  ) {
    return `${prefix}${filter.kind}:${filter.value}`;
  }
  if (filter.kind === "created" || filter.kind === "updated") {
    return `${prefix}${filter.kind}:${filter.op}${filter.value}`;
  }
  return null;
}

function stripFirstOccurrence(query: string, token: string): string {
  const tokens = query.split(/\s+/).filter((tok) => tok.length > 0);
  const idx = tokens.findIndex(
    (tok) => tok.toLowerCase() === token.toLowerCase(),
  );
  if (idx < 0) return query;
  tokens.splice(idx, 1);
  return tokens.join(" ");
}
