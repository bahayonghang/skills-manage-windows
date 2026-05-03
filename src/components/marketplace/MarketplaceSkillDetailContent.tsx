import { Bot, Code, FileText, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { SkillFrontmatterCard } from "@/components/skill/SkillFrontmatterCard";
import { SkillMarkdownRenderer } from "@/components/skill/SkillMarkdownRenderer";
import { cn } from "@/lib/utils";
import type { FrontmatterValue } from "@/lib/frontmatter";
import type { MarketplaceDetailViewMode } from "./marketplaceSkillDetailTypes";

interface MarketplaceSkillDetailContentProps {
  browserMode: boolean;
  content: string;
  contentError: string | null;
  displayContent: {
    frontmatterRaw: string;
    frontmatterData: Record<string, FrontmatterValue>;
    body: string;
  };
  explanation: string | null;
  explanationError: string | null;
  isExplaining: boolean;
  isLoadingContent: boolean;
  onExplain: () => void;
  onRetryContent: () => void;
  onViewModeChange: (mode: MarketplaceDetailViewMode) => void;
  variant?: "drawer" | "dialog";
  viewMode: MarketplaceDetailViewMode;
}

export function MarketplaceSkillDetailContent({
  browserMode,
  content,
  contentError,
  displayContent,
  explanation,
  explanationError,
  isExplaining,
  isLoadingContent,
  onExplain,
  onRetryContent,
  onViewModeChange,
  variant = "drawer",
  viewMode,
}: MarketplaceSkillDetailContentProps) {
  const { t } = useTranslation();
  const previewStackClassName =
    variant === "drawer"
      ? "mx-auto w-full max-w-4xl space-y-4"
      : "mx-auto w-full max-w-5xl space-y-4";

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={() => onViewModeChange("markdown")}
          className={cn(
            "inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs transition-colors cursor-pointer",
            viewMode === "markdown"
              ? "bg-primary/15 text-foreground font-medium"
              : "text-muted-foreground hover:bg-muted/40"
          )}
        >
          <FileText className="size-3" />
          {t("detail.markdown")}
        </button>
        <button
          type="button"
          onClick={() => onViewModeChange("raw")}
          className={cn(
            "inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs transition-colors cursor-pointer",
            viewMode === "raw"
              ? "bg-primary/15 text-foreground font-medium"
              : "text-muted-foreground hover:bg-muted/40"
          )}
        >
          <Code className="size-3" />
          {t("detail.rawSource")}
        </button>

        <div className="flex-1" />

        <Button
          variant="outline"
          size="sm"
          onClick={onExplain}
          disabled={isExplaining || !content || isLoadingContent || !!contentError || browserMode}
          className="h-8 text-xs"
        >
          {isExplaining ? (
            <Loader2 className="size-3 animate-spin" />
          ) : (
            <Bot className="size-3" />
          )}
          <span>
            {isExplaining
              ? t("detail.explanationStreaming")
              : t("detail.aiExplanation")}
          </span>
        </Button>
      </div>

      {browserMode && (
        <div className="rounded-md border border-border bg-muted/20 p-3 text-sm text-muted-foreground">
          {t("marketplace.previewBrowserFallback")}
        </div>
      )}

      {isLoadingContent ? (
        <div className="flex items-center justify-center gap-2 py-12 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
          {t("common.loading")}
        </div>
      ) : contentError ? (
        <div className="rounded-xl border border-destructive/30 bg-destructive/5 p-4">
          <div className="space-y-3">
            <p className="text-sm text-destructive">{contentError}</p>
            <Button variant="outline" size="sm" onClick={onRetryContent}>
              {t("common.retry")}
            </Button>
          </div>
        </div>
      ) : viewMode === "markdown" ? (
        <div className={previewStackClassName}>
          <SkillFrontmatterCard
            data={displayContent.frontmatterData}
            raw={displayContent.frontmatterRaw}
          />
          {content ? (
            <SkillMarkdownRenderer
              content={displayContent.body}
              variant="detail"
            />
          ) : (
            <p className="text-sm italic text-muted-foreground">
              {t("detail.noContent")}
            </p>
          )}
        </div>
      ) : (
        <pre className="overflow-auto whitespace-pre-wrap rounded-xl border border-border/70 bg-muted/30 p-4 text-xs">
          {content || t("detail.noContent")}
        </pre>
      )}

      {(explanation || isExplaining || explanationError) && (
        <div className="rounded-xl border border-primary/30 bg-primary/5 p-4">
          <div className="mb-2 flex items-center gap-1.5 text-xs font-medium text-primary">
            <Bot className="size-3.5" />
            {t("detail.aiExplanation")}
          </div>
          {isExplaining ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="size-3.5 animate-spin" />
              {t("detail.explanationStreaming")}
            </div>
          ) : explanationError ? (
            <div className="whitespace-pre-wrap text-sm leading-relaxed text-destructive">
              {explanationError}
            </div>
          ) : (
            <div className="whitespace-pre-wrap text-sm leading-relaxed text-foreground">
              {explanation}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
