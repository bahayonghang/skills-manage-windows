import { useState, type ReactNode } from "react";
import {
  ChevronsDownUp,
  ChevronsUpDown,
  CircleAlert,
  FolderGit2,
  Inbox,
  Layers,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  Search,
  Sparkles,
  X,
} from "lucide-react";
import type { TFunction } from "i18next";

import { FacetItem } from "@/components/central/FacetItem";
import { FacetSection } from "@/components/central/FacetSection";
import { SidebarExpansionProvider } from "@/components/central/SidebarExpansionProvider";
import type { SidebarExpansionSignal } from "@/components/central/sidebarExpansionContext";
import {
  filterRepositorySectionsForSearch,
  groupRepositoriesForSidebar,
} from "@/lib/centralRepositoryGroups";
import { cn } from "@/lib/utils";
import type { FacetCounts } from "@/lib/centralFacetCounts";
import type { SkillRepositoryWithStats } from "@/types";
import { RepositorySectionBlock } from "@/components/central/CentralSidebarBlocks";

/**
 * Central Skills sidebar：折叠优先（M4）。
 *
 * 状态：
 * - `pinned=false`（默认）：占用 48px 宽度的图标轨。鼠标 hover 或键盘 focus 进入侧栏时，
 *   内部 panel 以 `absolute` 浮起到 280px 宽度并覆盖在主列表上方（不挤压列表布局）。
 * - `pinned=true`：内部 panel 静态展开到 `width`，与原有 resizable 宽度行为一致。
 *
 * pin 状态通过 `localStorage["central.sidebarPinned"]` 持久化，默认未 pin。
 */

const SIDEBAR_PINNED_STORAGE_KEY = "central.sidebarPinned";
const SIDEBAR_COLLAPSED_RAIL_PX = 48;

function readPinnedFromStorage(): boolean {
  if (typeof window === "undefined") return false;
  try {
    return window.localStorage.getItem(SIDEBAR_PINNED_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

function writePinnedToStorage(value: boolean) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(SIDEBAR_PINNED_STORAGE_KEY, String(value));
  } catch {
    // ignore
  }
}

interface CommonSidebarProps {
  t: TFunction;
  width: number;
  startResize: (event: React.PointerEvent<HTMLElement>) => void;
  handleResizeKeyDown: (event: React.KeyboardEvent<HTMLElement>) => void;

  facetCounts: FacetCounts;
  repositories: readonly SkillRepositoryWithStats[];
  /** repo.id → 该 repo 下可更新 skill 数（用于 repo 行角标）。 */
  repoUpdateCounts?: Record<string, number>;

  selectedRepos: readonly string[];
  selectedTags: readonly string[];

  onToggleRepo: (repoId: string) => void;
  /** 智能视图（未分类/可更新/AI 复核）仍走 tag facet 特殊值。 */
  onToggleTag: (tagId: string) => void;
  onClearAll: () => void;
  /** 智能视图改写为 tag facet 中的特殊值。 */
  onSelectSmartView: (
    view: "all" | "uncategorized" | "updates" | "ai-review",
  ) => void;
  /** 删除仓库（M4 回填）。 */
  onDeleteRepository?: (repository: SkillRepositoryWithStats) => void;
  onToggleRepositoryPin?: (repository: SkillRepositoryWithStats) => void;
  /** 「同步新源」入口（打开 GitHub 导入对话框）。 */
  onSyncNewSource?: () => void;
  /** Saved Views 插槽（下沉到 Repositories 之后）。 */
  savedViewsSlot?: ReactNode;
}

export function CentralSidebar({
  t,
  width,
  startResize,
  handleResizeKeyDown,
  facetCounts,
  repositories,
  repoUpdateCounts,
  selectedRepos,
  selectedTags,
  onToggleRepo,
  onToggleTag,
  onClearAll,
  onSelectSmartView,
  onDeleteRepository,
  onToggleRepositoryPin,
  onSyncNewSource,
  savedViewsSlot,
}: CommonSidebarProps) {
  const [isPinned, setIsPinned] = useState<boolean>(() =>
    readPinnedFromStorage(),
  );
  const [isHovered, setIsHovered] = useState(false);
  const [isFocused, setIsFocused] = useState(false);
  const [bulkExpansionSignal, setBulkExpansionSignal] =
    useState<SidebarExpansionSignal | null>(null);
  const [bulkExpanded, setBulkExpanded] = useState(true);
  const [repositorySearchQuery, setRepositorySearchQuery] = useState("");

  const repoSelectionSet = new Set(selectedRepos);
  const tagSelectionSet = new Set(selectedTags);
  const hasAnySelection = selectedRepos.length > 0 || selectedTags.length > 0;
  const isOverlayExpanded = !isPinned && (isHovered || isFocused);
  const isExpanded = isPinned || isOverlayExpanded;

  const repositorySections = groupRepositoriesForSidebar(repositories);

  const handleToggleAllGroups = () => {
    const nextExpanded = !bulkExpanded;
    setBulkExpanded(nextExpanded);
    setBulkExpansionSignal((current) => ({
      expanded: nextExpanded,
      token: (current?.token ?? 0) + 1,
    }));
  };

  const handleTogglePin = () => {
    const next = !isPinned;
    setIsPinned(next);
    writePinnedToStorage(next);
    if (next) {
      // 进入 pinned 状态时清掉 overlay 触发状态，避免 hover 残留。
      setIsHovered(false);
      setIsFocused(false);
    }
  };

  const handleBlur = (event: React.FocusEvent<HTMLDivElement>) => {
    if (isPinned) return;
    if (!event.currentTarget.contains(event.relatedTarget as Node)) {
      setIsFocused(false);
    }
  };

  const handleFocus = () => {
    if (isPinned) return;
    setIsFocused(true);
  };

  const handleMouseEnter = () => {
    if (isPinned) return;
    setIsHovered(true);
  };

  const handleMouseLeave = () => {
    if (isPinned) return;
    setIsHovered(false);
  };

  const BulkIcon = bulkExpanded ? ChevronsDownUp : ChevronsUpDown;

  const outerWidth = isPinned ? width : SIDEBAR_COLLAPSED_RAIL_PX;

  return (
    <aside
      data-testid="central-sidebar-v2"
      data-pinned={isPinned}
      data-expanded={isExpanded}
      className="relative hidden min-h-0 shrink-0 md:flex md:flex-col"
      style={{ width: outerWidth }}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      <div
        onFocus={handleFocus}
        onBlur={handleBlur}
        className={cn(
          "flex h-full flex-col border-r border-border/60 bg-muted/15 transition-[width,box-shadow,background-color] duration-150",
          isPinned
            ? "w-full"
            : isExpanded
              ? "absolute inset-y-0 left-0 z-30 w-[280px] bg-background shadow-[0_0_0_1px_color-mix(in_srgb,var(--foreground)_8%,transparent),0_24px_60px_-28px_color-mix(in_srgb,var(--background)_92%,transparent)]"
              : "absolute inset-y-0 left-0 w-12",
        )}
      >
        {isExpanded ? (
          <ExpandedSidebarContent
            t={t}
            isPinned={isPinned}
            isOverlay={isOverlayExpanded}
            onTogglePin={handleTogglePin}
            bulkExpanded={bulkExpanded}
            BulkIcon={BulkIcon}
            handleToggleAllGroups={handleToggleAllGroups}
            bulkExpansionSignal={bulkExpansionSignal}
            savedViewsSlot={savedViewsSlot}
            facetCounts={facetCounts}
            hasAnySelection={hasAnySelection}
            onSelectSmartView={onSelectSmartView}
            onClearAll={onClearAll}
            onToggleTag={onToggleTag}
            tagSelectionSet={tagSelectionSet}
            repositorySections={repositorySections}
            repositorySearchQuery={repositorySearchQuery}
            onRepositorySearchQueryChange={setRepositorySearchQuery}
            repoSelectionSet={repoSelectionSet}
            repoUpdateCounts={repoUpdateCounts}
            onToggleRepo={onToggleRepo}
            onDeleteRepository={onDeleteRepository}
            onToggleRepositoryPin={onToggleRepositoryPin}
            onSyncNewSource={onSyncNewSource}
            selectedReposCount={selectedRepos.length}
            selectedTagsCount={selectedTags.length}
          />
        ) : (
          <CollapsedSidebarRail
            t={t}
            onTogglePin={handleTogglePin}
            hasAnySelection={hasAnySelection}
            smartViewActive={
              tagSelectionSet.has("uncategorized") ||
              tagSelectionSet.has("updates") ||
              tagSelectionSet.has("ai-review")
            }
            reposActive={selectedRepos.length > 0}
          />
        )}
      </div>

      {isPinned && (
        <div
          role="separator"
          aria-label={t("central.resizeFilterSidebar")}
          aria-orientation="vertical"
          tabIndex={0}
          onPointerDown={startResize}
          onKeyDown={handleResizeKeyDown}
          className="absolute right-0 top-0 z-40 h-full w-1.5 translate-x-1/2 cursor-col-resize rounded-full bg-transparent transition-colors hover:bg-primary/30 focus:outline-none focus-visible:bg-primary/40"
        />
      )}
    </aside>
  );
}

// ─── Expanded content ──────────────────────────────────────────────────────────

interface ExpandedSidebarContentProps {
  t: TFunction;
  isPinned: boolean;
  isOverlay: boolean;
  onTogglePin: () => void;
  bulkExpanded: boolean;
  BulkIcon: typeof ChevronsDownUp;
  handleToggleAllGroups: () => void;
  bulkExpansionSignal: SidebarExpansionSignal | null;
  savedViewsSlot?: ReactNode;
  facetCounts: FacetCounts;
  hasAnySelection: boolean;
  onSelectSmartView: (
    view: "all" | "uncategorized" | "updates" | "ai-review",
  ) => void;
  onClearAll: () => void;
  onToggleTag: (tagId: string) => void;
  tagSelectionSet: Set<string>;
  repositorySections: ReturnType<typeof groupRepositoriesForSidebar>;
  repositorySearchQuery: string;
  onRepositorySearchQueryChange: (query: string) => void;
  repoSelectionSet: Set<string>;
  repoUpdateCounts?: Record<string, number>;
  onToggleRepo: (id: string) => void;
  onDeleteRepository?: (repository: SkillRepositoryWithStats) => void;
  onToggleRepositoryPin?: (repository: SkillRepositoryWithStats) => void;
  onSyncNewSource?: () => void;
  selectedReposCount: number;
  selectedTagsCount: number;
}

function ExpandedSidebarContent({
  t,
  isPinned,
  isOverlay,
  onTogglePin,
  bulkExpanded,
  BulkIcon,
  handleToggleAllGroups,
  bulkExpansionSignal,
  savedViewsSlot,
  facetCounts,
  hasAnySelection,
  onSelectSmartView,
  onClearAll,
  onToggleTag,
  tagSelectionSet,
  repositorySections,
  repositorySearchQuery,
  onRepositorySearchQueryChange,
  repoSelectionSet,
  repoUpdateCounts,
  onToggleRepo,
  onDeleteRepository,
  onToggleRepositoryPin,
  onSyncNewSource,
  selectedReposCount,
  selectedTagsCount,
}: ExpandedSidebarContentProps) {
  const filteredRepositorySections = filterRepositorySectionsForSearch(
    repositorySections,
    repositorySearchQuery,
  );
  const isRepositorySearchActive = repositorySearchQuery.trim().length > 0;

  return (
    <>
      <div className="scrollbar-subtle min-h-0 flex-1 overflow-y-auto px-3 pb-6 pr-2">
        {/* Pin / bulk toggle row ─────────────────────────────────────────── */}
        <div className="flex items-center gap-2 pb-3 pt-3">
          <button
            type="button"
            data-testid="sidebar-pin-toggle"
            aria-pressed={isPinned}
            aria-label={
              isPinned
                ? t("central.v2.sidebarUnpin")
                : t("central.v2.sidebarPin")
            }
            title={
              isPinned
                ? t("central.v2.sidebarUnpin")
                : t("central.v2.sidebarPin")
            }
            onClick={onTogglePin}
            className={cn(
              "focus-ring grid size-9 shrink-0 place-items-center rounded-xl border transition-[scale,background-color,border-color,color] active:scale-[0.96]",
              isPinned
                ? "border-primary/30 bg-primary/10 text-primary"
                : "border-border/80 bg-background text-muted-foreground hover:border-primary/30 hover:text-primary",
            )}
          >
            {isPinned ? (
              <PanelLeftClose className="size-4" />
            ) : (
              <PanelLeftOpen className="size-4" />
            )}
          </button>
          <button
            type="button"
            data-testid="sidebar-bulk-expansion-toggle"
            aria-label={
              bulkExpanded
                ? t("central.v2.sidebarCollapseAllGroupsAria")
                : t("central.v2.sidebarExpandAllGroupsAria")
            }
            onClick={handleToggleAllGroups}
            className={cn(
              "focus-ring group flex min-h-10 flex-1 items-center gap-2 rounded-xl border px-3 py-2 text-left text-xs font-semibold shadow-sm ring-1 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]",
              bulkExpanded
                ? "border-primary/25 bg-primary/10 text-primary ring-primary/10 hover:bg-primary/15"
                : "border-border/90 bg-background text-foreground ring-border/40 hover:border-primary/30 hover:bg-muted/40",
            )}
          >
            <span className="grid size-7 shrink-0 place-items-center rounded-[10px] bg-background/85 text-primary ring-1 ring-primary/20 transition-[background-color] group-hover:bg-background">
              <BulkIcon className="size-3.5" />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate">
                {bulkExpanded
                  ? t("central.v2.sidebarCollapseAllGroups")
                  : t("central.v2.sidebarExpandAllGroups")}
              </span>
              <span className="mt-0.5 block truncate text-ui-meta font-medium text-muted-foreground">
                {isOverlay
                  ? t("central.v2.sidebarHoverHint")
                  : t("central.v2.sidebarBulkExpansionHint")}
              </span>
            </span>
          </button>
        </div>

        <SidebarExpansionProvider signal={bulkExpansionSignal}>
          {/* Smart views ─────────────────────────────────────────────────── */}
          <FacetSection
            title={t("central.v2.smartViews")}
            icon={<Sparkles className="size-3.5" />}
            testId="sidebar-section-smart"
          >
            <FacetItem
              label={t("central.v2.allSkills")}
              active={!hasAnySelection}
              count={facetCounts.smartViews.all}
              icon={<Layers className="size-3.5" />}
              testId="smart-view-all"
              onClick={() => {
                onSelectSmartView("all");
                onClearAll();
              }}
            />
            <FacetItem
              label={t("central.v2.uncategorized")}
              active={tagSelectionSet.has("uncategorized")}
              count={facetCounts.smartViews.uncategorized}
              icon={<Inbox className="size-3.5" />}
              testId="smart-view-uncategorized"
              onClick={() => onToggleTag("uncategorized")}
            />
            <FacetItem
              label={t("central.v2.updates")}
              active={tagSelectionSet.has("updates")}
              count={facetCounts.smartViews.updates}
              icon={<CircleAlert className="size-3.5" />}
              testId="smart-view-updates"
              onClick={() => onToggleTag("updates")}
            />
            <FacetItem
              label={t("central.v2.aiReview")}
              active={tagSelectionSet.has("ai-review")}
              count={facetCounts.smartViews.aiReview}
              icon={<Sparkles className="size-3.5" />}
              testId="smart-view-ai-review"
              onClick={() => onToggleTag("ai-review")}
            />
          </FacetSection>

          {/* Repositories ────────────────────────────────────────────────── */}
          <div className="mt-4">
            <FacetSection
              title={t("central.v2.repositories")}
              icon={<FolderGit2 className="size-3.5" />}
              testId="sidebar-section-repos"
            >
              <div className="px-1 pb-1">
                <div className="relative">
                  <Search className="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
                  <input
                    type="search"
                    data-testid="sidebar-repository-search"
                    value={repositorySearchQuery}
                    onChange={(event) =>
                      onRepositorySearchQueryChange(event.currentTarget.value)
                    }
                    onKeyDown={(event) => {
                      if (event.key === "Escape" && repositorySearchQuery) {
                        event.preventDefault();
                        onRepositorySearchQueryChange("");
                      }
                    }}
                    aria-label={t("central.v2.repositorySearchLabel")}
                    placeholder={t("central.v2.repositorySearchPlaceholder")}
                    className="h-8 w-full rounded-lg border border-border/70 bg-background py-1 pl-7 pr-8 text-xs text-foreground outline-none transition-[background-color,border-color,box-shadow] placeholder:text-muted-foreground hover:border-border focus:border-primary/50 focus:ring-2 focus:ring-primary/15"
                  />
                  {isRepositorySearchActive && (
                    <button
                      type="button"
                      data-testid="sidebar-repository-search-clear"
                      aria-label={t("central.v2.repositorySearchClear")}
                      onClick={() => onRepositorySearchQueryChange("")}
                      className="focus-ring absolute right-0.5 top-1/2 grid size-7 -translate-y-1/2 place-items-center rounded-lg text-muted-foreground transition-[scale,background-color,color] hover:bg-muted hover:text-foreground active:scale-[0.96]"
                    >
                      <X className="size-3.5" />
                    </button>
                  )}
                </div>
              </div>
              {filteredRepositorySections.length === 0 ? (
                <p className="px-2 text-ui-meta text-muted-foreground">
                  {t(
                    isRepositorySearchActive
                      ? "central.v2.repositorySearchEmpty"
                      : "central.v2.facetEmpty",
                  )}
                </p>
              ) : (
                filteredRepositorySections.map((section) => (
                  <RepositorySectionBlock
                    key={section.kind}
                    section={section}
                    facetCounts={facetCounts}
                    selectionSet={repoSelectionSet}
                    repoUpdateCounts={repoUpdateCounts}
                    onToggleRepo={onToggleRepo}
                    onDeleteRepository={onDeleteRepository}
                    onToggleRepositoryPin={onToggleRepositoryPin}
                    t={t}
                  />
                ))
              )}
              {onSyncNewSource && (
                <button
                  type="button"
                  data-testid="sidebar-sync-new-source"
                  onClick={onSyncNewSource}
                  className="focus-ring mt-1 flex min-h-9 w-full items-center gap-2 rounded-lg border border-dashed border-border/70 px-2 py-1.5 text-left text-xs text-muted-foreground transition-[scale,background-color,border-color,color] hover:border-primary/40 hover:bg-background/60 hover:text-primary active:scale-[0.96]"
                >
                  <Plus className="size-3.5 shrink-0" />
                  <span className="min-w-0 flex-1 truncate font-medium">
                    {t("central.v2.sidebarSyncNewSource")}
                  </span>
                </button>
              )}
            </FacetSection>
          </div>

          {/* Saved Views（下沉到底部） ──────────────────────────────────── */}
          {savedViewsSlot ? <div className="mt-4">{savedViewsSlot}</div> : null}
        </SidebarExpansionProvider>

        {/* Quick clear footer */}
        {hasAnySelection && (
          <div className="sticky bottom-0 -mx-3 mt-4 border-t border-border/60 bg-background/90 px-4 py-2 backdrop-blur-md">
            <button
              type="button"
              onClick={onClearAll}
              className="focus-ring w-full rounded-lg border border-border/70 bg-background px-2.5 py-1.5 text-xs font-medium text-muted-foreground transition-[scale,background-color,border-color,color] hover:border-border hover:bg-muted/40 hover:text-foreground active:scale-[0.96]"
            >
              {t("central.v2.selectionClear")}
            </button>
            <p className="mt-1.5 text-center text-ui-meta text-muted-foreground">
              {t("central.v2.selectionApplied", {
                count: selectedReposCount + selectedTagsCount,
              })}
            </p>
          </div>
        )}
      </div>
    </>
  );
}

// ─── Collapsed rail ────────────────────────────────────────────────────────────

interface CollapsedSidebarRailProps {
  t: TFunction;
  onTogglePin: () => void;
  hasAnySelection: boolean;
  smartViewActive: boolean;
  reposActive: boolean;
}

function CollapsedSidebarRail({
  t,
  onTogglePin,
  hasAnySelection,
  smartViewActive,
  reposActive,
}: CollapsedSidebarRailProps) {
  return (
    <div
      data-testid="central-sidebar-rail"
      className="flex h-full w-full flex-col items-center gap-2 py-3"
    >
      <button
        type="button"
        data-testid="sidebar-pin-toggle"
        aria-pressed={false}
        aria-label={t("central.v2.sidebarPin")}
        title={t("central.v2.sidebarPin")}
        onClick={onTogglePin}
        className="focus-ring grid size-9 place-items-center rounded-xl border border-border/70 bg-background text-muted-foreground transition-[scale,border-color,color] hover:border-primary/30 hover:text-primary active:scale-[0.96]"
      >
        <PanelLeftOpen className="size-4" />
      </button>
      <div className="my-1 h-px w-6 bg-border/40" />
      <RailIndicator
        icon={<Sparkles className="size-4" />}
        active={smartViewActive}
        title={t("central.v2.sidebarSmartViewsShort")}
      />
      <RailIndicator
        icon={<FolderGit2 className="size-4" />}
        active={reposActive}
        title={t("central.v2.sidebarReposShort")}
      />
      {hasAnySelection && (
        <span
          aria-hidden
          className="mt-1 size-1.5 rounded-full bg-primary"
          data-testid="sidebar-rail-selection-dot"
        />
      )}
    </div>
  );
}

function RailIndicator({
  icon,
  active,
  title,
}: {
  icon: ReactNode;
  active: boolean;
  title: string;
}) {
  return (
    <span
      title={title}
      className={cn(
        "grid size-9 place-items-center rounded-xl text-muted-foreground transition-[background-color,color]",
        active && "text-primary",
      )}
    >
      {icon}
    </span>
  );
}
