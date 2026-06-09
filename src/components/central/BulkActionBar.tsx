import { Bot, Download, Tags, Trash2, Unlink, X } from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * BulkActionBar：底部浮动批量操作条。
 * - 仅在 selectedCount ≥ 1 时渲染。
 * - 承载：批装 / 批删 / 打标签（→ Categorize 抽屉 manual）/ AI 建议（→ Categorize 抽屉 ai）/ 取消选择。
 * - 抽屉 review tab 由顶部 AI 进度条 / 卡片 review 徽章触发，不在这里。
 */
export interface BulkActionBarProps {
  selectedCount: number;
  isInstalling: boolean;
  isUninstalling: boolean;
  isDeleting: boolean;
  isAiBusy: boolean;
  aiTaggingAvailable: boolean;
  t: TFunction;
  onBatchInstall: () => void;
  onBatchUninstall: () => void;
  onBatchDelete: () => void;
  onOpenCategorize: () => void;
  onOpenAiSuggest: () => void;
  onClearSelection: () => void;
}

export function BulkActionBar({
  selectedCount,
  isInstalling,
  isUninstalling,
  isDeleting,
  isAiBusy,
  aiTaggingAvailable,
  t,
  onBatchInstall,
  onBatchUninstall,
  onBatchDelete,
  onOpenCategorize,
  onOpenAiSuggest,
  onClearSelection,
}: BulkActionBarProps) {
  if (selectedCount <= 0) return null;

  const aiDisabled = isAiBusy || !aiTaggingAvailable;
  const aiTitle = !aiTaggingAvailable
    ? t("central.aiTaggingConfigureReason")
    : isAiBusy
      ? t("central.aiTaggingRunningReason")
      : undefined;

  return (
    <div
      data-testid="central-bulk-action-bar"
      className={cn(
        "pointer-events-none fixed inset-x-0 bottom-4 z-30 flex justify-center px-4",
        "animate-in fade-in-0 slide-in-from-bottom-2 duration-150"
      )}
    >
      <div
        role="toolbar"
        aria-label={t("central.bulkBarAriaLabel")}
        className="pointer-events-auto flex max-w-full flex-wrap items-center gap-2 rounded-2xl border border-border/80 bg-popover/95 px-3 py-2 text-sm shadow-2xl ring-1 ring-foreground/10 backdrop-blur"
      >
        <span
          data-testid="central-bulk-action-count"
          className="flex items-center gap-2 rounded-full bg-primary/10 px-3 py-1 text-xs font-semibold text-primary ring-1 ring-primary/25"
        >
          {t("central.bulkBarSelectedCount", { count: selectedCount })}
        </span>
        <span className="hidden h-5 w-px bg-border sm:block" />
        <Button
          type="button"
          size="sm"
          variant="default"
          onClick={onBatchInstall}
          disabled={isInstalling}
          data-testid="bulk-bar-batch-install"
        >
          <Download className="size-3.5" />
          {t("central.bulkBarBatchInstall", { count: selectedCount })}
        </Button>
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={onBatchUninstall}
          disabled={isUninstalling}
          data-testid="bulk-bar-batch-uninstall"
        >
          <Unlink className="size-3.5" />
          {t("central.bulkBarBatchUninstall")}
        </Button>
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={onOpenCategorize}
          data-testid="bulk-bar-open-categorize"
        >
          <Tags className="size-3.5" />
          {t("central.bulkBarOpenCategorize")}
        </Button>
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={onOpenAiSuggest}
          disabled={aiDisabled}
          title={aiTitle}
          data-testid="bulk-bar-open-ai-suggest"
        >
          <Bot className="size-3.5" />
          {t("central.bulkBarOpenAiSuggest")}
        </Button>
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={onBatchDelete}
          disabled={isDeleting}
          className="border-destructive/30 text-destructive hover:bg-destructive/10 hover:text-destructive"
          data-testid="bulk-bar-batch-delete"
        >
          <Trash2 className="size-3.5" />
          {t("central.bulkBarBatchDelete", { count: selectedCount })}
        </Button>
        <span className="hidden h-5 w-px bg-border sm:block" />
        <Button
          type="button"
          size="sm"
          variant="ghost"
          className="text-muted-foreground hover:text-foreground"
          onClick={onClearSelection}
          data-testid="bulk-bar-clear-selection"
          aria-label={t("central.bulkBarClearSelectionLabel")}
        >
          <X className="size-3.5" />
          {t("central.bulkBarClearSelection")}
        </Button>
      </div>
    </div>
  );
}
