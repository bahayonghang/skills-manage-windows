import { useMemo } from "react";
import {
  Loader2,
  Download,
  Store,
  CheckCircle2,
  Sparkles,
  ShieldCheck,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogBody,
  DialogFooter,
  DialogClose,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { isTauriRuntime } from "@/lib/ipc";
import { MarketplaceSkillDetailContent } from "./MarketplaceSkillDetailContent";
import { MarketplaceSkillDetailSummary } from "./MarketplaceSkillDetailSummary";
import type { MarketplaceSkillDetail } from "./marketplaceSkillDetailTypes";
import { useMarketplaceSkillDetailViewModel } from "./marketplaceSkillDetailViewModel";

interface SkillPreviewDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  skillName: string;
  downloadUrl: string;
  description?: string;
  publisher?: string;
  sourceLabel?: string;
  sourceUrl?: string | null;
  installed?: boolean;
  onInstall: () => void;
  isInstalling: boolean;
  onAfterCloseFocus?: () => void;
}

export function SkillPreviewDialog({
  open,
  onOpenChange,
  skillName,
  downloadUrl,
  description,
  publisher,
  sourceLabel,
  sourceUrl,
  installed = false,
  onInstall,
  isInstalling,
  onAfterCloseFocus,
}: SkillPreviewDialogProps) {
  const { t } = useTranslation();
  const browserMode = !isTauriRuntime();
  const detailSkill = useMemo<MarketplaceSkillDetail>(
    () => ({
      id: downloadUrl || skillName,
      name: skillName,
      downloadUrl,
      description,
      publisher,
      sourceLabel,
      sourceUrl,
      installed,
    }),
    [description, downloadUrl, installed, publisher, skillName, sourceLabel, sourceUrl]
  );
  const {
    content,
    contentError,
    displayContent,
    explanation,
    explanationError,
    handleExplain,
    isExplaining,
    isLoadingContent,
    retryContent,
    setViewMode,
    viewMode,
  } = useMarketplaceSkillDetailViewModel({
    open,
    skill: detailSkill,
    onAfterCloseFocus,
  });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="!right-0 !left-auto !top-0 !h-screen !w-[min(42rem,100vw)] !max-w-none !translate-x-0 !translate-y-0 rounded-none border-l border-border/80 bg-popover/95 p-0 shadow-2xl supports-backdrop-filter:backdrop-blur">
        <DialogHeader className="gap-0 border-b border-border/70 px-6 py-5">
          <div className="flex items-start justify-between gap-4">
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <DialogTitle>{skillName}</DialogTitle>
                {installed ? (
                  <span className="inline-flex items-center gap-1 rounded-full bg-primary/10 px-2 py-0.5 text-[11px] font-medium text-primary">
                    <CheckCircle2 className="size-3.5" />
                    {t("marketplace.installed")}
                  </span>
                ) : null}
              </div>
              {description ? (
                <p className="mt-2 max-w-2xl text-sm leading-relaxed text-muted-foreground">
                  {description}
                </p>
              ) : null}
              <div className="mt-3 flex flex-wrap gap-2 text-xs text-muted-foreground">
                {sourceLabel ? (
                  <span className="inline-flex items-center gap-1 rounded-full bg-muted/60 px-2 py-1">
                    <Store className="size-3.5" />
                    {sourceLabel}
                  </span>
                ) : null}
                {publisher ? (
                  <span className="inline-flex items-center gap-1 rounded-full bg-muted/60 px-2 py-1">
                    <Sparkles className="size-3.5" />
                    {publisher}
                  </span>
                ) : null}
                <span className="inline-flex items-center gap-1 rounded-full bg-muted/60 px-2 py-1">
                  <ShieldCheck className="size-3.5" />
                  {browserMode
                    ? t("marketplace.previewModeBrowser")
                    : t("marketplace.previewModeDesktop")}
                </span>
              </div>
            </div>
            <DialogClose />
          </div>
        </DialogHeader>

        <DialogBody className="!max-h-none h-[calc(100vh-10rem)] space-y-5 px-6 py-5">
          <MarketplaceSkillDetailSummary skill={detailSkill} />
          <MarketplaceSkillDetailContent
            browserMode={browserMode}
            content={content}
            contentError={contentError}
            displayContent={displayContent}
            explanation={explanation}
            explanationError={explanationError}
            isExplaining={isExplaining}
            isLoadingContent={isLoadingContent}
            onExplain={handleExplain}
            onRetryContent={retryContent}
            onViewModeChange={setViewMode}
            variant="dialog"
            viewMode={viewMode}
          />
        </DialogBody>

        <DialogFooter className="px-6">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.close")}
          </Button>
          <Button onClick={onInstall} disabled={isInstalling || installed}>
            {isInstalling ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : installed ? (
              <CheckCircle2 className="size-3.5" />
            ) : (
              <Download className="size-3.5" />
            )}
            <span>{installed ? t("marketplace.installed") : t("marketplace.install")}</span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
