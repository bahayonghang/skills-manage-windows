import type { ComponentProps } from "react";
import { ArrowUpDown, Download, FileJson, LayoutGrid, RefreshCw, Search } from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { CentralInstalledSkillsQuickFilter } from "@/components/central/CentralInstalledSkillsQuickFilter";
import { AiTagProgressBar, CentralUpdateProgressBar } from "@/components/central/CentralSkillProgressBars";
import { CentralSkillCategorizePanel } from "@/components/central/CentralSkillCategorizePanel";
import { CentralSkillDialogs } from "@/components/central/CentralSkillDialogs";
import { CentralSkillFilterSidebar } from "@/components/central/CentralSkillFilterSidebar";
import { CentralSkillListContent } from "@/components/central/CentralSkillListContent";
import { CentralSkillTagSearch } from "@/components/central/CentralSkillTagSearch";
import { cn } from "@/lib/utils";
import type {
  CentralSortDirection,
  CentralSortField,
} from "@/pages/centralSkillsViewModel";
import type { SkillTag } from "@/types";

type FilterSidebarProps = Omit<ComponentProps<typeof CentralSkillFilterSidebar>, "t">;
type ListContentProps = Omit<ComponentProps<typeof CentralSkillListContent>, "t">;
type CategorizePanelProps = Omit<ComponentProps<typeof CentralSkillCategorizePanel>, "t">;
type AiProgressProps = Omit<ComponentProps<typeof AiTagProgressBar>, "t">;
type UpdateProgressProps = Omit<ComponentProps<typeof CentralUpdateProgressBar>, "t">;
type DialogProps = Omit<ComponentProps<typeof CentralSkillDialogs>, "t">;
type InstalledSkillsFilterProps = Omit<ComponentProps<typeof CentralInstalledSkillsQuickFilter>, "t">;
type TagSearchProps = Omit<
  ComponentProps<typeof CentralSkillTagSearch>,
  "t" | "tagFilter" | "setTagFilter" | "tags" | "updateAvailableSkillCount"
>;

export function CentralSkillsShell({
  centralSkillsDir,
  dialogs,
  filterSidebar,
  isCheckingUpdates,
  isLoading,
  installedSkillsFilter,
  listContent,
  searchQuery,
  setIsGitHubImportOpen,
  setIsPlatformManageOpen,
  setIsPortabilityOpen,
  setRepositoryFilter,
  setSearchQuery,
  setSortDirection,
  setSortField,
  setTagFilter,
  shouldShowCategorizePanel,
  shouldShowUpdateProgress,
  sortDirection,
  sortDirectionOptions,
  sortField,
  sortFieldOptions,
  tagFilter,
  tagSearch,
  tags,
  t,
  updateAvailableSkillCount,
  updateButton,
  checkButton,
  aiProgress,
  categorizePanel,
  updateProgress,
  onRefresh,
  onUpdateSkills,
  onSwitchToNew,
}: {
  centralSkillsDir: string;
  dialogs: DialogProps;
  filterSidebar: FilterSidebarProps;
  isCheckingUpdates: boolean;
  isLoading: boolean;
  installedSkillsFilter: InstalledSkillsFilterProps;
  listContent: ListContentProps;
  searchQuery: string;
  setIsGitHubImportOpen: (open: boolean) => void;
  setIsPlatformManageOpen: (open: boolean) => void;
  setIsPortabilityOpen: (open: boolean) => void;
  setRepositoryFilter: (filter: string) => void;
  setSearchQuery: (query: string) => void;
  setSortDirection: (direction: CentralSortDirection) => void;
  setSortField: (field: CentralSortField) => void;
  setTagFilter: (filter: string) => void;
  shouldShowCategorizePanel: boolean;
  shouldShowUpdateProgress: boolean;
  sortDirection: CentralSortDirection;
  sortDirectionOptions: Array<{ value: CentralSortDirection; label: string }>;
  sortField: CentralSortField;
  sortFieldOptions: Array<{ value: CentralSortField; label: string }>;
  tagFilter: string;
  tagSearch: TagSearchProps;
  tags: SkillTag[];
  t: TFunction;
  updateAvailableSkillCount: number;
  updateButton: {
    disabled: boolean;
    label: string;
    targetSkillIds: string[];
  };
  checkButton: {
    label: string;
    disabled: boolean;
    onClick: () => void;
  };
  aiProgress: AiProgressProps;
  categorizePanel: CategorizePanelProps;
  updateProgress: UpdateProgressProps;
  onRefresh: () => void;
  onUpdateSkills: (skillIds: string[]) => void;
  onSwitchToNew?: () => void;
}) {
  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-border px-6 py-4 flex items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold">{t("central.title")}</h1>
            <Button
              variant="ghost"
              size="icon"
              onClick={onRefresh}
              disabled={isLoading}
              aria-label={t("central.refresh")}
            >
              <RefreshCw className={`size-4 ${isLoading ? "animate-spin" : ""}`} />
            </Button>
          </div>
          <p className="text-xs text-muted-foreground mt-1">
            {t("central.scope")}
          </p>
          <p className="text-sm text-muted-foreground mt-0.5">
            {centralSkillsDir}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            onClick={checkButton.onClick}
            disabled={checkButton.disabled}
          >
            <RefreshCw className={`size-3.5 ${isCheckingUpdates ? "animate-spin" : ""}`} />
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
          <Button
            variant="outline"
            onClick={() => setIsPlatformManageOpen(true)}
          >
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
          {onSwitchToNew && (
            <Button
              variant="ghost"
              size="sm"
              data-testid="central-switch-new-layout"
              onClick={onSwitchToNew}
            >
              <LayoutGrid className="size-3.5" />
              {t("central.v2.switchToNew")}
            </Button>
          )}
        </div>
      </div>

      <div className="px-6 py-3 border-b border-border">
        <div className="flex flex-col gap-3">
          <div className="relative w-full">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground pointer-events-none" />
            <Input
              placeholder={t("central.searchPlaceholder")}
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              className="pl-8 bg-muted/40"
              aria-label={t("central.searchPlaceholder")}
            />
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <ArrowUpDown className="size-3.5" />
              <span>{t("central.sortLabel")}</span>
            </div>
            <div
              role="group"
              aria-label={t("central.sortFieldLabel")}
              className="flex rounded-xl bg-muted/40 p-1"
            >
              {sortFieldOptions.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  aria-pressed={sortField === option.value}
                  onClick={() => setSortField(option.value)}
                  className={cn(
                    "h-7 rounded-lg px-3 text-xs font-medium transition-colors cursor-pointer",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
                    sortField === option.value
                      ? "bg-background/95 text-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-background/60 hover:text-foreground"
                  )}
                >
                  {option.label}
                </button>
              ))}
            </div>
            <div
              role="group"
              aria-label={t("central.sortDirectionLabel")}
              className="flex rounded-xl bg-muted/40 p-1"
            >
              {sortDirectionOptions.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  aria-pressed={sortDirection === option.value}
                  onClick={() => setSortDirection(option.value)}
                  className={cn(
                    "h-7 rounded-lg px-3 text-xs font-medium transition-colors cursor-pointer",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
                    sortDirection === option.value
                      ? "bg-background/95 text-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-background/60 hover:text-foreground"
                  )}
                >
                  {option.label}
                </button>
              ))}
            </div>
            <CentralSkillTagSearch
              tagFilter={tagFilter}
              setTagFilter={setTagFilter}
              setCategorizeTab={tagSearch.setCategorizeTab}
              tags={tags}
              tagCounts={tagSearch.tagCounts}
              uncategorizedCount={tagSearch.uncategorizedCount}
              updateAvailableSkillCount={updateAvailableSkillCount}
              aiReviewCount={tagSearch.aiReviewCount}
              totalSkillCount={tagSearch.totalSkillCount}
              t={t}
            />
            <Button
              variant={filterSidebar.repositoryFilter === "unassigned" ? "default" : "outline"}
              size="sm"
              onClick={() =>
                setRepositoryFilter(
                  filterSidebar.repositoryFilter === "unassigned" ? "all" : "unassigned"
                )
              }
            >
              {t("central.unassignedOnly")}
            </Button>
            <CentralInstalledSkillsQuickFilter
              {...installedSkillsFilter}
              t={t}
            />
          </div>
        </div>
      </div>

      <div className="flex min-h-0 flex-1">
        <CentralSkillFilterSidebar {...filterSidebar} t={t} />
        <CentralSkillListContent {...listContent} t={t} />
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
