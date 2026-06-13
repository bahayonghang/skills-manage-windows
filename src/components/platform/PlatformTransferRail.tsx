import { useTranslation } from "react-i18next";
import {
  ArrowRightLeft,
  Eraser,
  PackagePlus,
  Sparkles,
  Trash2,
} from "lucide-react";

import { Button } from "@/components/ui/button";

export function PlatformTransferRail({
  selectedCount,
  selectableCount,
  onSelectCurrentResults,
  onClearSelection,
  onSelectMostUniversal,
}: {
  selectedCount: number;
  selectableCount: number;
  onSelectCurrentResults: () => void;
  onClearSelection: () => void;
  onSelectMostUniversal?: () => void;
}) {
  const { t } = useTranslation();
  const hasSelectableSkills = selectableCount > 0;
  const canSelectMostUniversal = typeof onSelectMostUniversal === "function";

  return (
    <div
      data-testid="platform-transfer-rail"
      className="border-b border-border bg-muted/40 px-6 py-3"
    >
      <div className="flex flex-wrap items-center gap-3">
        <div className="flex min-w-[220px] flex-1 items-center gap-3">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border bg-primary/10 text-primary">
            <ArrowRightLeft className="size-4" aria-hidden />
          </div>
          <div className="min-w-0">
            <p className="text-xs font-medium text-muted-foreground">
              {t("platform.batchRailKicker")}
            </p>
            <p
              data-testid="platform-transfer-summary"
              className="text-sm font-semibold text-foreground"
            >
              {t("platform.batchSelectionSummary", {
                selectedCount,
                selectableCount,
              })}
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={!hasSelectableSkills}
            onClick={onSelectCurrentResults}
            data-testid="platform-select-current-results"
          >
            {t("platform.transferSelectCurrent")}
          </Button>
          {canSelectMostUniversal && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={!hasSelectableSkills}
              onClick={onSelectMostUniversal}
              data-testid="platform-select-most-universal"
            >
              <Sparkles className="size-3.5" aria-hidden />
              {t("platform.transferSelectMostUniversal")}
            </Button>
          )}
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={selectedCount === 0}
            onClick={onClearSelection}
            data-testid="platform-clear-selection"
          >
            <Eraser className="size-3.5" aria-hidden />
            {t("platform.transferClear")}
          </Button>
        </div>
      </div>
    </div>
  );
}

export function PlatformTransferActionBar({
  selectedCount,
  isInstalling,
  isDeleting,
  onClearSelection,
  onDelete,
  onInstall,
}: {
  selectedCount: number;
  isInstalling: boolean;
  isDeleting: boolean;
  onClearSelection: () => void;
  onDelete: () => void;
  onInstall?: () => void;
}) {
  const { t } = useTranslation();
  const showInstall = typeof onInstall === "function";

  if (selectedCount === 0) return null;

  return (
    <div
      data-testid="platform-batch-action-bar"
      className="sticky bottom-0 z-20 border-t border-border bg-card/95 px-6 py-3 backdrop-blur"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <span className="inline-flex size-2 rounded-full bg-primary" />
          <span className="text-sm font-semibold text-foreground">
            {t("platform.batchActionSummary", { count: selectedCount })}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            className="h-9 px-3 text-xs"
            onClick={onClearSelection}
          >
            {t("platform.transferClear")}
          </Button>
          {showInstall && (
            <Button
              type="button"
              className="h-9 px-3 text-xs font-semibold"
              disabled={isInstalling || isDeleting}
              onClick={onInstall}
              data-testid="platform-batch-install"
            >
              <PackagePlus className="size-3.5" aria-hidden />
              {t("platform.transferInstall")}
            </Button>
          )}
          <Button
            type="button"
            variant="destructive"
            className="h-9 px-3 text-xs font-semibold"
            disabled={isDeleting || isInstalling}
            onClick={onDelete}
            data-testid="platform-batch-delete"
          >
            <Trash2 className="size-3.5" aria-hidden />
            {t("platform.batchDelete")}
          </Button>
        </div>
      </div>
    </div>
  );
}
