import { AlertTriangle, Check, Plus, Tags, Wand2 } from "lucide-react";
import type { ReactNode } from "react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import { CentralSkillAiTagPanel } from "@/components/central/CentralSkillAiTagPanel";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type {
  AiTagProgressItem,
  AiTagRateProfile,
} from "@/lib/centralAiTagDashboard";
import type { CentralCategorizeTab } from "@/pages/centralSkillsViewModel";
import type { AiTagJob, SkillAiTagReview, SkillTag } from "@/types";

export interface CentralSkillCategorizePanelProps {
  aiTagJob: AiTagJob;
  aiTagProgressItems: AiTagProgressItem[];
  aiTagRateProfile: AiTagRateProfile;
  aiTagReviews: SkillAiTagReview[];
  aiTaggingAvailable: boolean;
  canCreateManualTag: boolean;
  categorizeTab: CentralCategorizeTab;
  filteredManualTags: SkillTag[];
  isMetadataUpdating: boolean;
  isSuggestingTags: boolean;
  manualSelectedTagIds: string[];
  manualTagQuery: string;
  selectedSkillCount: number;
  sortedSkillCount: number;
  t: TFunction;
  onAcceptReview: (review: SkillAiTagReview) => void;
  onApplyManualTags: () => void;
  onApplyManualTagsToReview: (review: SkillAiTagReview) => void;
  onBulkSuggestTags: () => void;
  onCancelAiTag: () => void;
  onCreateManualTag: () => void;
  onSetCategorizeTab: (tab: CentralCategorizeTab) => void;
  onSetManualTagQuery: (query: string) => void;
  onSkipReview: (review: SkillAiTagReview) => void;
  onToggleManualTag: (tagId: string) => void;
}

/**
 * Categorize 内容主体。M5 起改为抽屉装载，原本的常驻 aside / 顶部 header / resizer 全部退场。
 * 由 CategorizeDrawer 包裹并提供：标题、关闭按钮、抽屉自身的宽度与遮罩。
 */
export function CentralSkillCategorizePanel({
  aiTagJob,
  aiTagProgressItems,
  aiTagRateProfile,
  aiTagReviews,
  aiTaggingAvailable,
  canCreateManualTag,
  categorizeTab,
  filteredManualTags,
  isMetadataUpdating,
  isSuggestingTags,
  manualSelectedTagIds,
  manualTagQuery,
  selectedSkillCount,
  sortedSkillCount,
  t,
  onAcceptReview,
  onApplyManualTags,
  onApplyManualTagsToReview,
  onBulkSuggestTags,
  onCancelAiTag,
  onCreateManualTag,
  onSetCategorizeTab,
  onSetManualTagQuery,
  onSkipReview,
  onToggleManualTag,
}: CentralSkillCategorizePanelProps) {
  function getManualPrimaryLabel() {
    if (selectedSkillCount === 0)
      return t("central.categorizeSelectSkillsFirst");
    if (manualSelectedTagIds.length === 0)
      return t("central.categorizeSelectTagsFirst");
    return t("central.applyToSelectedSkills", { count: selectedSkillCount });
  }

  function getManualDisabledReason() {
    if (selectedSkillCount === 0)
      return t("central.categorizeSelectSkillsReason");
    if (manualSelectedTagIds.length === 0)
      return t("central.categorizeSelectTagsReason");
    if (isMetadataUpdating) return t("central.categorizeApplyingReason");
    return null;
  }

  function getAiPrimaryLabel() {
    if (selectedSkillCount === 0)
      return t("central.categorizeSelectSkillsFirst");
    if (!aiTaggingAvailable) return t("central.aiTaggingConfigureFirst");
    if (isSuggestingTags || aiTagJob.status === "running")
      return t("central.aiTaggingRunning");
    return t("central.aiSuggestSelected", { count: selectedSkillCount });
  }

  function getAiDisabledReason() {
    if (!aiTaggingAvailable) return t("central.aiTaggingConfigureReason");
    if (selectedSkillCount === 0)
      return t("central.categorizeSelectSkillsReason");
    if (isSuggestingTags || aiTagJob.status === "running")
      return t("central.aiTaggingRunningReason");
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
    <div className="flex min-h-0 flex-1 flex-col">
      {/* 选中范围摘要 */}
      <div className="border-b border-border/80 px-5 py-3">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs">
          <span
            className={cn(
              "font-medium",
              selectedSkillCount > 0 ? "text-primary" : "text-foreground",
            )}
          >
            {t("central.selectedSkillSummary", { count: selectedSkillCount })}
          </span>
          <span className="h-3 w-px bg-border" />
          <span
            className={cn(
              "font-medium",
              manualSelectedTagIds.length > 0
                ? "text-primary"
                : "text-foreground",
            )}
          >
            {t("central.pendingTagSummary", {
              count: manualSelectedTagIds.length,
            })}
          </span>
          <span className="ml-auto text-muted-foreground">
            {t("central.aiScopeDesc", {
              selected: selectedSkillCount,
              total: sortedSkillCount,
            })}
          </span>
        </div>
      </div>

      {/* Tabs + 当前 tab 内容 */}
      <div className="scrollbar-subtle min-h-0 flex-1 overflow-y-auto p-5">
        <div
          role="tablist"
          aria-label={t("central.categorizeIntent")}
          className="mb-4 grid grid-cols-3 gap-1 rounded-xl bg-muted/20 p-1.5 shadow-inner ring-1 ring-border/80"
        >
          {(["manual", "ai", "review"] as const).map((tab) => (
            <button
              key={tab}
              type="button"
              role="tab"
              aria-selected={categorizeTab === tab}
              onClick={() => onSetCategorizeTab(tab)}
              className={cn(
                "flex h-9 items-center justify-center gap-1.5 rounded-lg border px-2 text-xs font-semibold transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
                categorizeTab === tab
                  ? "border-primary/35 bg-background text-primary shadow-sm ring-1 ring-primary/15"
                  : "border-transparent text-foreground hover:border-border/80 hover:bg-background/80 hover:text-foreground",
              )}
            >
              {tabIcons[tab]}
              <span>{t(`central.categorizeTab.${tab}`)}</span>
              {tab === "review" && aiTagReviews.length > 0 && (
                <span className="ml-0.5 inline-flex min-w-[18px] items-center justify-center rounded-full bg-warning/15 px-1.5 text-ui-micro font-semibold text-warning-foreground ring-1 ring-warning/30">
                  {aiTagReviews.length}
                </span>
              )}
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
            <div className="flex max-h-72 flex-wrap gap-2 overflow-auto rounded-xl bg-muted/10 p-3 text-foreground ring-1 ring-border/90">
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
                        : "border-border bg-background text-foreground hover:border-primary/40 hover:text-foreground",
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
          <CentralSkillAiTagPanel
            aiTagJob={aiTagJob}
            aiTagProgressItems={aiTagProgressItems}
            aiTagRateProfile={aiTagRateProfile}
            aiTagReviews={aiTagReviews}
            aiTaggingAvailable={aiTaggingAvailable}
            selectedSkillCount={selectedSkillCount}
            sortedSkillCount={sortedSkillCount}
            t={t}
            onCancelAiTag={onCancelAiTag}
            onOpenReviewTab={() => onSetCategorizeTab("review")}
          />
        )}

        {categorizeTab === "review" && (
          <div className="space-y-3">
            {aiTagReviews.length > 0 && manualSelectedTagIds.length === 0 && (
              <div className="rounded-xl bg-muted/20 p-3 text-xs text-foreground ring-1 ring-border/90">
                {t("central.reviewReplaceNeedsTags")}
              </div>
            )}
            {aiTagReviews.length === 0 ? (
              <div className="rounded-xl bg-muted/20 p-4 text-center text-xs text-foreground ring-1 ring-border/90">
                {t("central.reviewEmpty")}
              </div>
            ) : (
              aiTagReviews.map((review) => (
                <div
                  key={`${review.skill_id}:${review.tag.id}`}
                  className="rounded-xl bg-background p-3 shadow-sm ring-1 ring-border/90"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="truncate text-sm font-medium">
                        {review.skill_name}
                      </div>
                      <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-foreground">
                        {review.is_proposal && (
                          <span className="rounded-md bg-accent px-2 py-0.5 font-medium text-accent-foreground">
                            {t("central.reviewProposalBadge")}
                          </span>
                        )}
                        <span className="rounded-full bg-muted px-2 py-0.5">
                          {review.tag.name}
                        </span>
                        <span>
                          {review.confidence == null
                            ? "—"
                            : `${Math.round(review.confidence * 100)}%`}
                        </span>
                      </div>
                    </div>
                    <AlertTriangle className="size-4 shrink-0 text-warning-foreground" />
                  </div>
                  <p className="mt-2 text-xs leading-relaxed text-foreground">
                    {review.reason}
                  </p>
                  {review.is_proposal && review.tag.description && (
                    <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                      {review.tag.description}
                    </p>
                  )}
                  <div className="mt-3 flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => onAcceptReview(review)}
                    >
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
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => onSkipReview(review)}
                    >
                      {t("central.reviewSkip")}
                    </Button>
                  </div>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      {/* 主操作行 */}
      <div className="border-t border-border/80 bg-muted/10 px-5 py-3">
        {categorizeTab === "manual" && (
          <>
            <Button
              className={cn(
                "w-full",
                isManualActionDisabled
                  ? "border border-dashed border-border bg-muted/30 text-foreground shadow-none"
                  : "shadow-sm",
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
                className="mt-2 text-xs leading-relaxed text-foreground"
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
                  ? "border border-dashed border-border bg-muted/30 text-foreground shadow-none"
                  : "shadow-sm",
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
                className="mt-2 text-xs leading-relaxed text-foreground"
                data-testid="categorize-action-reason"
              >
                {aiDisabledReason}
              </p>
            )}
          </>
        )}
        {categorizeTab === "review" && (
          <p className="text-xs leading-relaxed text-foreground">
            {t("central.reviewApplyHint")}
          </p>
        )}
      </div>
    </div>
  );
}
