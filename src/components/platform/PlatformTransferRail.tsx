import { useTranslation } from "react-i18next";
import { ArrowRightLeft, Eraser, PackagePlus, Sparkles, Trash2 } from "lucide-react";

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
      className="border-b border-slate-800 bg-[radial-gradient(circle_at_12%_0%,rgba(14,165,233,0.18),transparent_28%),linear-gradient(135deg,#020617_0%,#0f172a_52%,#111827_100%)] px-6 py-3 text-slate-100 shadow-inner"
    >
      <div className="flex flex-wrap items-center gap-3">
        <div className="flex min-w-[220px] flex-1 items-center gap-3">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-cyan-400/25 bg-cyan-400/10 text-cyan-200 shadow-[0_0_28px_rgba(34,211,238,0.16)]">
            <ArrowRightLeft className="size-4" aria-hidden />
          </div>
          <div className="min-w-0">
            <p className="text-[10px] font-semibold uppercase tracking-[0.24em] text-cyan-200/80">
              {t("platform.batchRailKicker")}
            </p>
            <p
              data-testid="platform-transfer-summary"
              className="text-sm font-semibold text-slate-50"
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
            variant="ghost"
            size="sm"
            className="h-8 border border-slate-700/80 bg-slate-900/80 px-2.5 text-xs text-slate-100 hover:bg-cyan-400/15 hover:text-cyan-100"
            disabled={!hasSelectableSkills}
            onClick={onSelectCurrentResults}
            data-testid="platform-select-current-results"
          >
            {t("platform.transferSelectCurrent")}
          </Button>
          {canSelectMostUniversal && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 border border-cyan-400/35 bg-cyan-400/10 px-2.5 text-xs text-cyan-100 hover:bg-cyan-400/20 hover:text-white"
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
            className="h-8 px-2.5 text-xs text-slate-300 hover:bg-slate-800 hover:text-slate-50"
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
      className="sticky bottom-0 z-20 border-t border-cyan-500/30 bg-slate-950/95 px-6 py-3 text-slate-100 shadow-[0_-16px_40px_rgba(2,6,23,0.45)] backdrop-blur"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <span className="inline-flex size-2 rounded-full bg-cyan-300 shadow-[0_0_18px_rgba(103,232,249,0.85)]" />
          <span className="text-sm font-semibold">
            {t("platform.batchActionSummary", { count: selectedCount })}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            className="h-9 px-3 text-xs text-slate-300 hover:bg-slate-800 hover:text-slate-50"
            onClick={onClearSelection}
          >
            {t("platform.transferClear")}
          </Button>
          {showInstall && (
            <Button
              type="button"
              className="h-9 bg-cyan-300 px-3 text-xs font-semibold text-slate-950 hover:bg-cyan-200"
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
