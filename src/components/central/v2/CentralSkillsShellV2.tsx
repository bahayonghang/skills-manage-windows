import type { ComponentProps, ReactNode } from "react";
import { ArrowUpDown, Download, FileJson, RefreshCw } from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import { CentralInstalledSkillsQuickFilter } from "@/components/central/CentralInstalledSkillsQuickFilter";
import { AiTagProgressBar, CentralUpdateProgressBar } from "@/components/central/CentralSkillProgressBars";
import { CentralSkillCategorizePanel } from "@/components/central/CentralSkillCategorizePanel";
import { CentralSkillDialogs } from "@/components/central/CentralSkillDialogs";
import { CentralSkillListContent } from "@/components/central/CentralSkillListContent";
import { CentralSearchBarV2 } from "@/components/central/v2/CentralSearchBarV2";
import { CentralSidebarV2 } from "@/components/central/v2/CentralSidebarV2";
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
  CentralQueryAst,
  SkillRepositoryWithStats,
  SkillTag,
  TagGroup,
} from "@/types";

/**
 * Central Skills V2 Shell。
 *
 * 设计原则：
 * - **复用** V1 的 list / dialogs / progress / categorize 渲染（不重写已稳定的部分）
 * - **替换** sidebar (CentralSidebarV2) 与 search bar (CentralSearchBarV2)
 * - 通过 viewState / setViewState 驱动新视图状态（多选 + URL state 同步）
 *
 * Beta 提示：右上角显示 Beta 徽章 + "切回经典布局" 链接。
 */

type ListContentProps = Omit<ComponentProps<typeof CentralSkillListContent>, "t">;
type CategorizePanelProps = Omit<ComponentProps<typeof CentralSkillCategorizePanel>, "t">;
type AiProgressProps = Omit<ComponentProps<typeof AiTagProgressBar>, "t">;
type UpdateProgressProps = Omit<ComponentProps<typeof CentralUpdateProgressBar>, "t">;
type DialogProps = Omit<ComponentProps<typeof CentralSkillDialogs>, "t">;
type InstalledSkillsFilterProps = Omit<ComponentProps<typeof CentralInstalledSkillsQuickFilter>, "t">;

export interface CentralSkillsShellV2Props {
  t: TFunction;

  // Layout / chrome
  centralSkillsDir: string;
  isLoading: boolean;
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
  shouldShowCategorizePanel: boolean;
  aiProgress: AiProgressProps;
  updateProgress: UpdateProgressProps;
  shouldShowUpdateProgress: boolean;
  dialogs: DialogProps;

  // Header 操作
  setIsGitHubImportOpen: (open: boolean) => void;
  setIsPlatformManageOpen: (open: boolean) => void;
  setIsPortabilityOpen: (open: boolean) => void;
  onRefresh: () => void;
  onUpdateSkills: (skillIds: string[]) => void;
  updateAvailableSkillCount: number;
  updateButton: { disabled: boolean; label: string; targetSkillIds: string[] };
  checkButton: { label: string; disabled: boolean; onClick: () => void };

  // Beta 切换回 V1（feature flag override）
  onSwitchToClassic?: () => void;
  /** 命令面板触发。M2 由父层传入完整实现。 */
  onOpenPalette?: () => void;
  /** Sidebar 顶部插槽（M2 加入，M3 后由父层渲染 CentralSidebarV2Header，包含 Saved Views + Tag Groups）。 */
  savedViewsSlot?: ReactNode;
}

export function CentralSkillsShellV2(props: CentralSkillsShellV2Props) {
  const {
    t,
    centralSkillsDir,
    isLoading,
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
    shouldShowCategorizePanel,
    aiProgress,
    updateProgress,
    shouldShowUpdateProgress,
    dialogs,
    setIsGitHubImportOpen,
    setIsPlatformManageOpen,
    setIsPortabilityOpen,
    onRefresh,
    onUpdateSkills,
    updateAvailableSkillCount,
    updateButton,
    checkButton,
    onSwitchToClassic,
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

  return (
    <div className="flex flex-col h-full" data-shell="v2">
      {/* Header ─────────────────────────────────────────────────────── */}
      <div className="border-b border-border px-6 py-4 flex items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold">{t("central.title")}</h1>
            <span
              data-testid="central-v2-beta-badge"
              className="rounded-full border border-primary/30 bg-primary/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-primary"
            >
              {t("central.v2.betaBadge")}
            </span>
            <Button
              variant="ghost"
              size="icon"
              onClick={onRefresh}
              disabled={isLoading}
              aria-label={t("central.refresh")}
            >
              <RefreshCw className={cn("size-4", isLoading && "animate-spin")} />
            </Button>
          </div>
          <p className="text-xs text-muted-foreground mt-1">{t("central.scope")}</p>
          <p className="text-sm text-muted-foreground mt-0.5">{centralSkillsDir}</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={checkButton.onClick} disabled={checkButton.disabled}>
            <RefreshCw className={cn("size-3.5", isCheckingUpdates && "animate-spin")} />
            {checkButton.label}
          </Button>
          {updateAvailableSkillCount > 0 && (
            <Button
              variant="default"
              onClick={() => onUpdateSkills(updateButton.targetSkillIds)}
              disabled={updateButton.disabled}
            >
              <Download className="size-3.5" />
              {updateButton.label}
            </Button>
          )}
          <Button variant="outline" onClick={() => setIsPlatformManageOpen(true)}>
            {t("central.platformManageButton")}
          </Button>
          <Button
            variant="outline"
            data-testid="central-portability-open"
            onClick={() => setIsPortabilityOpen(true)}
          >
            <FileJson className="size-3.5" />
            {t("central.portabilityOpen")}
          </Button>
          <Button variant="outline" onClick={() => setIsGitHubImportOpen(true)}>
            {t("marketplace.githubImportSecondaryCta")}
          </Button>
          {onSwitchToClassic && (
            <Button
              variant="ghost"
              size="sm"
              data-testid="central-v2-switch-classic"
              onClick={onSwitchToClassic}
            >
              {t("central.v2.switchToClassic")}
            </Button>
          )}
        </div>
      </div>

      {/* Search + sort ──────────────────────────────────────────────── */}
      <div className="px-6 py-3 border-b border-border">
        <div className="flex flex-col gap-3">
          <CentralSearchBarV2
            t={t}
            value={viewState.q}
            onChange={handleQueryChange}
            queryAst={queryAst}
            onRemoveFilter={handleRemoveFilter}
            onClear={handleQueryClear}
            onOpenPalette={onOpenPalette}
          />
          <div className="flex flex-wrap items-center gap-2">
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <ArrowUpDown className="size-3.5" />
              <span>{t("central.sortLabel")}</span>
            </div>
            <SegmentedToggle
              value={viewState.sortField}
              options={sortFieldOptions}
              onChange={(field) => setViewState({ ...viewState, sortField: field })}
              ariaLabel={t("central.sortFieldLabel")}
            />
            <SegmentedToggle
              value={viewState.sortDir}
              options={sortDirectionOptions}
              onChange={(dir) => setViewState({ ...viewState, sortDir: dir })}
              ariaLabel={t("central.sortDirectionLabel")}
            />
            {groupByOptions && groupByOptions.length > 1 && (
              <>
                <span
                  className="ml-2 text-xs text-muted-foreground"
                  data-testid="group-by-label"
                >
                  {t("central.v2.groupByLabel")}
                </span>
                <SegmentedToggle
                  value={viewState.group}
                  options={groupByOptions}
                  onChange={(group) => setViewState({ ...viewState, group })}
                  ariaLabel={t("central.v2.groupByLabel")}
                />
              </>
            )}
            <CentralInstalledSkillsQuickFilter
              {...installedSkillsFilter}
              t={t}
            />
          </div>
        </div>
      </div>

      {/* Body: sidebar + list + categorize ───────────────────────────── */}
      <div className="flex min-h-0 flex-1">
        <CentralSidebarV2
          t={t}
          width={filterSidebarWidth}
          startResize={startFilterSidebarResize}
          handleResizeKeyDown={handleFilterSidebarResizeKeyDown}
          facetCounts={facetCounts}
          repositories={repositories}
          tags={tags}
          tagGroups={tagGroups}
          onAssignTagToGroup={onAssignTagToGroup}
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
        {shouldShowCategorizePanel && (
          <CentralSkillCategorizePanel {...categorizePanel} t={t} />
        )}
      </div>

      <AiTagProgressBar {...aiProgress} t={t} />
      {shouldShowUpdateProgress && (
        <CentralUpdateProgressBar {...updateProgress} t={t} />
      )}
      <CentralSkillDialogs {...dialogs} t={t} />
    </div>
  );
}

interface GroupedListViewProps {
  listContent: ListContentProps;
  group: GroupByMode;
  t: TFunction;
}

/**
 * 把 V1 的 listContent props 适配成 CentralGroupedSkillList 所需，并按 group
 * 分桶。group="none" 不会走到这里，由父组件直接渲染 CentralSkillListContent。
 */
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

function SegmentedToggle<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
}: {
  value: T;
  options: Array<{ value: T; label: string }>;
  onChange: (value: T) => void;
  ariaLabel: string;
}) {
  return (
    <div role="group" aria-label={ariaLabel} className="flex rounded-xl bg-muted/40 p-1">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          aria-pressed={value === option.value}
          onClick={() => onChange(option.value)}
          className={cn(
            "h-7 rounded-lg px-3 text-xs font-medium transition-colors cursor-pointer",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
            value === option.value
              ? "bg-background/95 text-foreground shadow-sm"
              : "text-muted-foreground hover:bg-background/60 hover:text-foreground"
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

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
