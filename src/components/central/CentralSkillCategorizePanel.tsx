import {
  AlertTriangle,
  Bot,
  Check,
  Download,
  ListChecks,
  Plus,
  Tags,
  Trash2,
  Wand2,
  X,
} from "lucide-react";
import type { ReactNode } from "react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { CentralCategorizeTab } from "@/pages/centralSkillsViewModel";
import type { AiTagJob, SkillAiTagReview, SkillTag } from "@/types";

export function CentralSkillCategorizePanel({
  aiTagJob,
  aiTagReviews,
  aiTaggingAvailable,
  canCreateManualTag,
  categorizeSidebarWidth,
  categorizeTab,
  filteredManualTags,
  isDeleting,
  isInstalling,
  isMetadataUpdating,
  isSuggestingTags,
  manualSelectedTagIds,
  manualTagQuery,
  selectedSkillCount,
  sortedSkillCount,
  startCategorizeSidebarResize,
  t,
  handleCategorizeSidebarResizeKeyDown,
  onAcceptReview,
  onApplyManualTags,
  onApplyManualTagsToReview,
  onBatchDelete,
  onBatchInstallOpen,
  onBulkSuggestTags,
  onClearSelection,
  onCreateManualTag,
  onSelectCurrentFilter,
  onSelectUncategorized,
  onSetCategorizeTab,
  onSetManualTagQuery,
  onSkipReview,
  onToggleManualTag,
}: {
  aiTagJob: AiTagJob;
  aiTagReviews: SkillAiTagReview[];
  aiTaggingAvailable: boolean;
  canCreateManualTag: boolean;
  categorizeSidebarWidth: number;
  categorizeTab: CentralCategorizeTab;
  filteredManualTags: SkillTag[];
  isDeleting: boolean;
  isInstalling: boolean;
  isMetadataUpdating: boolean;
  isSuggestingTags: boolean;
  manualSelectedTagIds: string[];
  manualTagQuery: string;
  selectedSkillCount: number;
  sortedSkillCount: number;
  startCategorizeSidebarResize: (event: React.PointerEvent<HTMLElement>) => void;
  t: TFunction;
  handleCategorizeSidebarResizeKeyDown: (event: React.KeyboardEvent<HTMLElement>) => void;
  onAcceptReview: (review: SkillAiTagReview) => void;
  onApplyManualTags: () => void;
  onApplyManualTagsToReview: (review: SkillAiTagReview) => void;
  onBatchDelete: () => void;
  onBatchInstallOpen: () => void;
  onBulkSuggestTags: () => void;
  onClearSelection: () => void;
  onCreateManualTag: () => void;
  onSelectCurrentFilter: () => void;
  onSelectUncategorized: () => void;
  onSetCategorizeTab: (tab: CentralCategorizeTab) => void;
  onSetManualTagQuery: (query: string) => void;
  onSkipReview: (review: SkillAiTagReview) => void;
  onToggleManualTag: (tagId: string) => void;
}) {
  function getManualPrimaryLabel() {
    if (selectedSkillCount === 0) return t("central.categorizeSelectSkillsFirst");
    if (manualSelectedTagIds.length === 0) return t("central.categorizeSelectTagsFirst");
    return t("central.applyToSelectedSkills", { count: selectedSkillCount });
  }

  function getManualDisabledReason() {
    if (selectedSkillCount === 0) return t("central.categorizeSelectSkillsReason");
    if (manualSelectedTagIds.length === 0) return t("central.categorizeSelectTagsReason");
    if (isMetadataUpdating) return t("central.categorizeApplyingReason");
    return null;
  }

  function getAiPrimaryLabel() {
    if (selectedSkillCount === 0) return t("central.categorizeSelectSkillsFirst");
    if (!aiTaggingAvailable) return t("central.aiTaggingConfigureFirst");
    if (isSuggestingTags || aiTagJob.status === "running") return t("central.aiTaggingRunning");
    return t("central.aiSuggestSelected", { count: selectedSkillCount });
  }

  function getAiDisabledReason() {
    if (!aiTaggingAvailable) return t("central.aiTaggingConfigureReason");
    if (selectedSkillCount === 0) return t("central.categorizeSelectSkillsReason");
    if (isSuggestingTags || aiTagJob.status === "running") return t("central.aiTaggingRunningReason");
    return null;
  }

  const manualDisabledReason = getManualDisabledReason();
  const aiDisabledReason = getAiDisabledReason();
  const isManualActionDisabled = Boolean(manualDisabledReason);
  const isAiActionDisabled = Boolean(aiDisabledReason);
  const tabIcons: Record<CentralCategorizeTab, ReactNode> = {
    manual: <Tags className="size-3.5" />,
    ai: <Wand2 className="size-3.5" />,
    review: <AlertTriangle className="size-3.5" />,
  };

  return (
    <aside
      data-testid="central-categorize-sidebar"
      className="relative hidden min-h-0 shrink-0 border-l border-border bg-background/95 xl:flex xl:flex-col"
      style={{ width: categorizeSidebarWidth }}
    >
      <div
        role="separator"
        aria-label={t("central.resizeCategorizeSidebar")}
        aria-orientation="vertical"
        tabIndex={0}
        onPointerDown={startCategorizeSidebarResize}
        onKeyDown={handleCategorizeSidebarResizeKeyDown}
        className="absolute left-0 top-0 z-20 h-full w-2 -translate-x-1/2 cursor-col-resize rounded-full bg-transparent transition-colors hover:bg-primary/30 focus:outline-none focus-visible:bg-primary/40"
      />
      <div className="border-b border-border/80 bg-background p-5">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-base font-semibold tracking-tight text-foreground">
              {t("central.categorizePanelTitle")}
            </h2>
            <p className="mt-1 text-[13px] leading-5 text-foreground/75">
              {t("central.categorizePanelDesc")}
            </p>
          </div>
          <span className="grid size-8 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary ring-1 ring-primary/20">
            <ListChecks className="size-4" />
          </span>
        </div>
        <div className="mt-4 flex flex-wrap items-center gap-x-2 gap-y-1 rounded-2xl border border-border/90 bg-muted/10 px-3 py-2 text-xs shadow-sm">
          <span
            className={cn(
              "font-medium",
              selectedSkillCount > 0 ? "text-primary" : "text-foreground/70"
            )}
          >
            {t("central.selectedSkillSummary", { count: selectedSkillCount })}
          </span>
          <span className="h-3 w-px bg-border" />
          <span
            className={cn(
              "font-medium",
              manualSelectedTagIds.length > 0 ? "text-primary" : "text-foreground/70"
            )}
          >
            {t("central.pendingTagSummary", { count: manualSelectedTagIds.length })}
          </span>
        </div>
      </div>

      <div className="scrollbar-subtle min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
        <section className="rounded-2xl border border-border/90 bg-background p-4 shadow-sm">
          <div className="mb-3 flex items-center gap-2 text-[13px] font-semibold text-foreground">
            <span className="grid size-5 place-items-center rounded-full bg-primary/15 text-[11px] font-bold text-primary ring-1 ring-primary/25">1</span>
            {t("central.categorizeRange")}
          </div>
          <div className="space-y-2">
            <RangeActionCard
              title={t("central.selectCurrentFilter")}
              description={t("central.categorizeSelectCurrentFilterDesc")}
              icon={<ListChecks className="size-4" />}
              emphasis="primary"
              onClick={onSelectCurrentFilter}
            />
            <RangeActionCard
              title={t("central.selectUncategorized")}
              description={t("central.categorizeSelectUncategorizedDesc")}
              icon={<Tags className="size-4" />}
              emphasis="secondary"
              onClick={onSelectUncategorized}
            />
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-1 text-xs text-foreground/65 hover:text-foreground"
              onClick={onClearSelection}
            >
              <X className="size-3.5" />
              {t("central.clearSelection")}
            </Button>
          </div>
          {selectedSkillCount > 0 && (
            <div className="mt-3 rounded-2xl border border-border/80 bg-muted/10 p-3 shadow-sm">
              <div className="mb-2 flex items-center justify-between gap-2">
                <span className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                  {t("central.categorizeSelectedActions")}
                </span>
                <span className="rounded-full border border-border/80 bg-background px-2 py-0.5 text-[10px] font-medium text-foreground/70">
                  {t("central.selectedSkillSummary", { count: selectedSkillCount })}
                </span>
              </div>
              <div className="grid grid-cols-2 gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="h-auto min-h-8 whitespace-normal px-2 leading-tight"
                  onClick={onBatchInstallOpen}
                  disabled={isInstalling}
                  data-testid="batch-install-central-skills"
                >
                  <Download className="size-3.5" />
                  {t("central.batchInstallSelected", { count: selectedSkillCount })}
                </Button>
                <Button
                  variant="destructive"
                  size="sm"
                  className="h-auto min-h-8 whitespace-normal border-destructive/20 bg-destructive/5 px-2 leading-tight"
                  onClick={onBatchDelete}
                  disabled={isDeleting}
                  data-testid="batch-delete-central-skills"
                >
                  <Trash2 className="size-3.5" />
                  {t("central.batchDeleteSelected", { count: selectedSkillCount })}
                </Button>
              </div>
            </div>
          )}
        </section>

        <section className="rounded-2xl border border-border/90 bg-background p-4 shadow-sm">
          <div className="mb-3 flex items-center gap-2 text-[13px] font-semibold text-foreground">
            <span className="grid size-5 place-items-center rounded-full bg-primary/15 text-[11px] font-bold text-primary ring-1 ring-primary/25">2</span>
            {t("central.categorizeIntent")}
          </div>
          <div
            role="tablist"
            aria-label={t("central.categorizeIntent")}
            className="mb-4 grid grid-cols-3 gap-1 rounded-2xl border border-border/80 bg-muted/20 p-1.5 shadow-inner"
          >
            {(["manual", "ai", "review"] as const).map((tab) => (
              <button
                key={tab}
                type="button"
                role="tab"
                aria-selected={categorizeTab === tab}
                onClick={() => onSetCategorizeTab(tab)}
                className={cn(
                  "flex h-9 items-center justify-center gap-1.5 rounded-xl border px-2 text-xs font-semibold transition-colors",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
                  categorizeTab === tab
                    ? "border-primary/35 bg-background text-primary shadow-sm ring-1 ring-primary/15"
                    : "border-transparent text-foreground/60 hover:border-border/80 hover:bg-background/80 hover:text-foreground"
                )}
              >
                {tabIcons[tab]}
                <span>{t(`central.categorizeTab.${tab}`)}</span>
              </button>
            ))}
          </div>
          {categorizeTab === "manual" && (
            <div className="space-y-3">
              <Input
                value={manualTagQuery}
                onChange={(event) => onSetManualTagQuery(event.target.value)}
                placeholder={t("central.searchTagsPlaceholder")}
                aria-label={t("central.searchTagsPlaceholder")}
                className="h-9 text-xs"
              />
              {canCreateManualTag && (
                <Button variant="ghost" size="sm" onClick={onCreateManualTag}>
                  <Plus className="size-3.5" />
                  {t("central.createTagInline", { name: manualTagQuery.trim() })}
                </Button>
              )}
              <div className="flex max-h-52 flex-wrap gap-2 overflow-auto rounded-2xl border border-border/90 bg-muted/10 p-3 text-foreground">
                {filteredManualTags.map((tag) => {
                  const selected = manualSelectedTagIds.includes(tag.id);
                  return (
                    <button
                      key={tag.id}
                      type="button"
                      aria-pressed={selected}
                      onClick={() => onToggleManualTag(tag.id)}
                      className={cn(
                        "inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium transition-colors",
                        selected
                          ? "border-primary/70 bg-primary/15 text-primary shadow-sm ring-1 ring-primary/20"
                          : "border-border bg-background text-foreground/75 hover:border-primary/40 hover:text-foreground"
                      )}
                    >
                      {selected && <Check className="size-3" />}
                      {tag.name}
                    </button>
                  );
                })}
              </div>
            </div>
          )}
          {categorizeTab === "ai" && (
            <div className="space-y-3">
              <div className="rounded-2xl border border-border/90 bg-background p-3 text-xs shadow-sm">
                <div className="mb-1 flex items-center gap-1.5 font-medium text-foreground">
                  <Bot className="size-3.5 text-primary" />
                  {t("central.aiScopeTitle")}
                </div>
                <p className="leading-relaxed text-foreground/75">
                  {t("central.aiScopeDesc", {
                    selected: selectedSkillCount,
                    total: sortedSkillCount,
                  })}
                </p>
              </div>
              {!aiTaggingAvailable && (
                <div className="rounded-2xl border border-border/90 bg-muted/20 p-3 text-xs text-foreground/75">
                  {t("central.aiTaggingNeedsConfig")}
                </div>
              )}
              <div className="rounded-2xl border border-border/90 bg-muted/10 p-3 text-xs text-foreground/75">
                {t("central.aiPreview", { count: selectedSkillCount })}
              </div>
              <div className="rounded-2xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-700 dark:text-amber-300">
                {t("central.aiTagRateHint")}
              </div>
            </div>
          )}
          {categorizeTab === "review" && (
            <div className="space-y-3">
              {aiTagReviews.length > 0 && manualSelectedTagIds.length === 0 && (
                <div className="rounded-2xl border border-border/90 bg-muted/20 p-3 text-xs text-foreground/75">
                  {t("central.reviewReplaceNeedsTags")}
                </div>
              )}
              {aiTagReviews.length === 0 ? (
                <div className="rounded-2xl border border-border/90 bg-muted/20 p-4 text-center text-xs text-foreground/70">
                  {t("central.reviewEmpty")}
                </div>
              ) : (
                aiTagReviews.map((review) => (
                  <div
                    key={`${review.skill_id}:${review.tag.id}`}
                    className="rounded-2xl border border-border/90 bg-background p-3 shadow-sm"
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="truncate text-sm font-medium">{review.skill_name}</div>
                        <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-foreground/70">
                          <span className="rounded-full bg-muted px-2 py-0.5">
                            {review.tag.name}
                          </span>
                          <span>{Math.round(review.confidence * 100)}%</span>
                        </div>
                      </div>
                      <AlertTriangle className="size-4 shrink-0 text-amber-500" />
                    </div>
                    <p className="mt-2 text-xs leading-relaxed text-foreground/75">
                      {review.reason}
                    </p>
                    <div className="mt-3 flex flex-wrap gap-2">
                      <Button size="sm" variant="outline" onClick={() => onAcceptReview(review)}>
                        {t("central.reviewAccept")}
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        disabled={manualSelectedTagIds.length === 0}
                        onClick={() => onApplyManualTagsToReview(review)}
                      >
                        {t("central.reviewChange")}
                      </Button>
                      <Button size="sm" variant="ghost" onClick={() => onSkipReview(review)}>
                        {t("central.reviewSkip")}
                      </Button>
                    </div>
                  </div>
                ))
              )}
            </div>
          )}
        </section>
      </div>

      <div className="border-t border-border/80 bg-background p-4 shadow-lg">
        <div className="mb-2 flex items-center gap-2 text-xs font-semibold text-foreground">
          <span className="grid size-5 place-items-center rounded-full bg-primary/15 text-[11px] font-bold text-primary ring-1 ring-primary/25">3</span>
          {t("central.categorizeApply")}
        </div>
        {categorizeTab === "manual" && (
          <>
            <Button
              className={cn(
                "w-full",
                isManualActionDisabled
                  ? "border border-dashed border-border bg-muted/30 text-foreground/60 shadow-none"
                  : "shadow-sm"
              )}
              disabled={isManualActionDisabled}
              onClick={onApplyManualTags}
              data-testid="categorize-primary-action"
            >
              <Check className="size-3.5" />
              {getManualPrimaryLabel()}
            </Button>
            {manualDisabledReason && (
              <p
                className="mt-2 text-xs leading-relaxed text-foreground/75"
                data-testid="categorize-action-reason"
              >
                {manualDisabledReason}
              </p>
            )}
          </>
        )}
        {categorizeTab === "ai" && (
          <>
            <Button
              className={cn(
                "w-full",
                isAiActionDisabled
                  ? "border border-dashed border-border bg-muted/30 text-foreground/60 shadow-none"
                  : "shadow-sm"
              )}
              disabled={isAiActionDisabled}
              onClick={onBulkSuggestTags}
              data-testid="categorize-primary-action"
            >
              <Wand2 className="size-3.5" />
              {getAiPrimaryLabel()}
            </Button>
            {aiDisabledReason && (
              <p
                className="mt-2 text-xs leading-relaxed text-foreground/75"
                data-testid="categorize-action-reason"
              >
                {aiDisabledReason}
              </p>
            )}
          </>
        )}
        {categorizeTab === "review" && (
          <p className="text-xs leading-relaxed text-foreground/70">
            {t("central.reviewApplyHint")}
          </p>
        )}
      </div>
    </aside>
  );
}

function RangeActionCard({
  title,
  description,
  icon,
  emphasis,
  onClick,
}: {
  title: string;
  description: string;
  icon: ReactNode;
  emphasis: "primary" | "secondary";
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={title}
      onClick={onClick}
      className={cn(
        "group flex w-full items-start gap-3 rounded-2xl border p-3 text-left shadow-sm transition-colors",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
        emphasis === "primary"
          ? "border-primary/25 bg-primary/10 text-foreground hover:bg-primary/15"
          : "border-border/90 bg-muted/15 text-foreground hover:border-primary/25 hover:bg-background"
      )}
    >
      <span
        className={cn(
          "mt-0.5 grid size-8 shrink-0 place-items-center rounded-xl ring-1 transition-colors",
          emphasis === "primary"
            ? "bg-background/90 text-primary ring-primary/20"
            : "bg-background text-foreground/70 ring-border/80 group-hover:text-primary"
        )}
      >
        {icon}
      </span>
      <span className="min-w-0 flex-1">
        <span className="block text-sm font-semibold leading-tight">{title}</span>
        <span className="mt-1 block text-[11px] leading-4 text-muted-foreground">
          {description}
        </span>
      </span>
    </button>
  );
}
