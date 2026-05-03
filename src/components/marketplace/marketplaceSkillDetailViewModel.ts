import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import i18n from "@/i18n";
import { setupExplanationStreamListeners } from "@/lib/explanationStream";
import { parseFrontmatter } from "@/lib/frontmatter";
import { isTauriRuntime } from "@/lib/tauri";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import type {
  MarketplaceDetailViewMode,
  MarketplaceSkillDetail,
} from "./marketplaceSkillDetailTypes";

interface UseMarketplaceSkillDetailViewModelOptions {
  open: boolean;
  skill: MarketplaceSkillDetail | null;
  onAfterCloseFocus?: () => void;
}

export function useMarketplaceSkillDetailViewModel({
  open,
  skill,
  onAfterCloseFocus,
}: UseMarketplaceSkillDetailViewModelOptions) {
  const triggerSkillExplanation = useMarketplaceStore(
    (state) => state.triggerSkillExplanation
  );
  const [content, setContent] = useState("");
  const [contentError, setContentError] = useState<string | null>(null);
  const [isLoadingContent, setIsLoadingContent] = useState(false);
  const [viewMode, setViewMode] =
    useState<MarketplaceDetailViewMode>("markdown");
  const [explanation, setExplanation] = useState<string | null>(null);
  const [isExplaining, setIsExplaining] = useState(false);
  const [explanationError, setExplanationError] = useState<string | null>(null);
  const explanationRequestRef = useRef(0);
  const explanationUnlistenRef = useRef<(() => void) | null>(null);
  const browserMode = !isTauriRuntime();

  const cleanupExplanation = useCallback(() => {
    explanationUnlistenRef.current?.();
    explanationUnlistenRef.current = null;
  }, []);

  const fetchContent = useCallback(async () => {
    if (!open || !skill?.downloadUrl) {
      return;
    }

    setIsLoadingContent(true);
    setContentError(null);
    try {
      const response = await fetch(skill.downloadUrl);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      setContent(await response.text());
    } catch {
      setContent("");
      setContentError(i18n.t("marketplace.previewLoadSkillError"));
    } finally {
      setIsLoadingContent(false);
    }
  }, [open, skill?.downloadUrl]);

  useEffect(() => {
    if (!open || !skill?.downloadUrl) {
      return;
    }
    setContent("");
    setContentError(null);
    setExplanation(null);
    setExplanationError(null);
    setViewMode("markdown");
    void fetchContent();
  }, [open, skill?.downloadUrl, fetchContent]);

  useEffect(() => {
    if (!open) {
      cleanupExplanation();
      onAfterCloseFocus?.();
    }
  }, [cleanupExplanation, onAfterCloseFocus, open]);

  useEffect(() => cleanupExplanation, [cleanupExplanation]);

  const parsedContent = useMemo(() => {
    if (!content) {
      return { frontmatterRaw: "", frontmatterData: {}, body: "" };
    }
    return parseFrontmatter(content);
  }, [content]);

  const handleExplain = useCallback(async () => {
    if (!content || browserMode || !skill) {
      return;
    }

    explanationRequestRef.current += 1;
    const requestId = explanationRequestRef.current;
    const skillExplanationId = `marketplace-preview:${skill.id || skill.downloadUrl}`;

    cleanupExplanation();
    setIsExplaining(true);
    setExplanation(null);
    setExplanationError(null);

    try {
      explanationUnlistenRef.current = await setupExplanationStreamListeners(
        skillExplanationId,
        {
          onChunk: (chunkText) => {
            if (requestId !== explanationRequestRef.current) {
              return;
            }
            setIsExplaining(false);
            setExplanation((previous) => `${previous ?? ""}${chunkText}`);
          },
          onComplete: (payload) => {
            if (requestId !== explanationRequestRef.current) {
              return;
            }
            cleanupExplanation();
            const nextExplanation = payload.explanation ?? "";
            if (nextExplanation.trim()) {
              setExplanation(payload.explanation ?? null);
              setExplanationError(null);
            } else {
              setExplanation(null);
              setExplanationError(i18n.t("marketplace.previewExplanationEmpty"));
            }
            setIsExplaining(false);
          },
          onError: (payload) => {
            if (requestId !== explanationRequestRef.current) {
              return;
            }
            cleanupExplanation();
            setExplanation(null);
            setExplanationError(
              payload.error ?? i18n.t("marketplace.previewExplanationUnknownError")
            );
            setIsExplaining(false);
          },
        }
      );

      await triggerSkillExplanation(skillExplanationId, content, i18n.language);
    } catch (error) {
      cleanupExplanation();
      setExplanation(null);
      setExplanationError(String(error));
      setIsExplaining(false);
    }
  }, [browserMode, cleanupExplanation, content, skill, triggerSkillExplanation]);

  return {
    browserMode,
    content,
    contentError,
    displayContent: parsedContent,
    explanation,
    explanationError,
    handleExplain,
    isExplaining,
    isLoadingContent,
    retryContent: fetchContent,
    setViewMode,
    viewMode,
  };
}
