import type { Ref } from "react";
import { useTranslation } from "react-i18next";
import {
  Bot,
  ChevronDown,
  ChevronRight,
  Loader2,
  RefreshCw,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { SkillFrontmatterCard } from "@/components/skill/SkillFrontmatterCard";
import { SkillMarkdownRenderer } from "@/components/skill/SkillMarkdownRenderer";
import { formatBackendError } from "@/lib/backendError";
import { statusChipClass } from "@/lib/statusTone";
import type { FrontmatterValue } from "@/lib/frontmatter";
import type { ExplanationErrorInfo } from "@/lib/explanationStream";
import type { PreviewTab } from "./skillDetailViewTypes";

interface SkillDetailPreviewProps {
  activeTab: PreviewTab;
  scrollContainerRef?: Ref<HTMLDivElement>;
  previewStackClassName: string;
  frontmatterRaw: string;
  frontmatterData: Record<string, FrontmatterValue>;
  markdownContent: string;
  content: string | null;
  explanation: string | null;
  isExplanationLoading: boolean;
  isExplanationStreaming: boolean;
  explanationError: string | null;
  explanationErrorInfo: ExplanationErrorInfo | null;
  showErrorDetails: boolean;
  onToggleErrorDetails: () => void;
  onGenerateExplanation: () => void;
  onRefreshExplanation: () => void;
}

export function SkillDetailPreview({
  activeTab,
  scrollContainerRef,
  previewStackClassName,
  frontmatterRaw,
  frontmatterData,
  markdownContent,
  content,
  explanation,
  isExplanationLoading,
  isExplanationStreaming,
  explanationError,
  explanationErrorInfo,
  showErrorDetails,
  onToggleErrorDetails,
  onGenerateExplanation,
  onRefreshExplanation,
}: SkillDetailPreviewProps) {
  const { t } = useTranslation();
  const explanationErrorMessage = explanationErrorInfo?.code
    ? formatBackendError(
        `${explanationErrorInfo.code}:${explanationErrorInfo.message}`,
        t,
      )
    : explanationErrorInfo?.message || explanationError;

  return (
    <div
      ref={scrollContainerRef}
      className="scrollbar-subtle min-w-0 flex-1 overflow-auto"
    >
      {activeTab === "markdown" ? (
        <div
          className="space-y-4 p-6"
          role="tabpanel"
          aria-label={t("detail.markdown")}
        >
          <div className={previewStackClassName}>
            <SkillFrontmatterCard data={frontmatterData} raw={frontmatterRaw} />
            {content ? (
              <SkillMarkdownRenderer
                content={markdownContent}
                variant="detail"
              />
            ) : (
              <p className="text-sm italic text-muted-foreground">
                {t("detail.noContent")}
              </p>
            )}
          </div>
        </div>
      ) : activeTab === "raw" ? (
        <pre
          className="whitespace-pre-wrap break-words p-6 font-mono text-ui-meta leading-5 text-foreground"
          role="tabpanel"
          aria-label={t("detail.rawSource")}
        >
          {content ?? t("detail.noContent")}
        </pre>
      ) : (
        <div
          className="space-y-4 p-6"
          role="tabpanel"
          aria-label={t("detail.aiExplanation")}
        >
          <div className={previewStackClassName}>
            <div className="flex items-center justify-between gap-3">
              <div>
                <h2 className="flex items-center gap-2 text-sm font-semibold">
                  <Bot className="size-4 text-primary" />
                  {t("detail.aiExplanation")}
                </h2>
                <p className="mt-1 text-xs text-muted-foreground">
                  {t("detail.aiExplanationDesc")}
                </p>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={
                  explanation ? onRefreshExplanation : onGenerateExplanation
                }
                disabled={
                  !content || isExplanationLoading || isExplanationStreaming
                }
                className="gap-1.5"
              >
                {isExplanationLoading || isExplanationStreaming ? (
                  <Loader2 className="size-3.5 animate-spin" />
                ) : explanation ? (
                  <RefreshCw className="size-3.5" />
                ) : (
                  <Bot className="size-3.5" />
                )}
                {explanation
                  ? t("detail.regenerateExplanation")
                  : t("detail.generateExplanation")}
              </Button>
            </div>

            {explanationError && (
              <div
                className={`space-y-2 rounded-md border p-3 ${statusChipClass.error}`}
              >
                <p className="text-sm">{explanationErrorMessage}</p>
                {(explanationErrorInfo?.details ||
                  explanationError !== explanationErrorMessage) && (
                  <div>
                    <button
                      className="flex cursor-pointer items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
                      onClick={onToggleErrorDetails}
                    >
                      {showErrorDetails ? (
                        <ChevronDown className="size-3" />
                      ) : (
                        <ChevronRight className="size-3" />
                      )}
                      {t("detail.showDetails")}
                    </button>
                    {showErrorDetails && (
                      <pre className="scrollbar-subtle mt-1.5 max-h-40 overflow-auto whitespace-pre-wrap break-all rounded-md bg-muted/30 p-2 font-mono text-ui-meta leading-4 text-muted-foreground">
                        {explanationErrorInfo?.details || explanationError}
                      </pre>
                    )}
                  </div>
                )}
                {explanationErrorInfo?.fallbackTried && (
                  <p className="text-xs text-muted-foreground">
                    {t("detail.fallbackTried")}
                  </p>
                )}
              </div>
            )}

            {isExplanationLoading && !explanation ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                {t("detail.explanationLoading")}
              </div>
            ) : explanation ? (
              <div className="space-y-3">
                <SkillMarkdownRenderer
                  content={explanation}
                  variant="compact"
                />
                {isExplanationStreaming && (
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    <Loader2 className="size-3.5 animate-spin" />
                    {t("detail.explanationStreaming")}
                  </div>
                )}
              </div>
            ) : (
              <div className="space-y-3 rounded-lg border border-dashed border-border p-8 text-center">
                <Bot className="mx-auto size-8 text-muted-foreground" />
                <div>
                  <p className="text-sm font-medium">
                    {t("detail.noExplanationTitle")}
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {t("detail.noExplanationDesc")}
                  </p>
                </div>
                <Button
                  size="sm"
                  onClick={onGenerateExplanation}
                  disabled={
                    !content || isExplanationLoading || isExplanationStreaming
                  }
                  className="gap-1.5"
                >
                  <Bot className="size-3.5" />
                  {t("detail.generateExplanation")}
                </Button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
