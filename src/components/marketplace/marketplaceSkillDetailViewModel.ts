import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import i18n from "@/i18n";
import { setupExplanationStreamListeners } from "@/lib/explanationStream";
import { buildDirectoryTreeFromSkillsShEntries } from "@/lib/fileTree";
import { parseFrontmatter } from "@/lib/frontmatter";
import { isTauriRuntime } from "@/lib/tauri";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import type { DirectoryTreeEntry } from "@/types";
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
  const browseSkillsShDirectory = useMarketplaceStore(
    (state) => state.browseSkillsShDirectory
  );
  const readSkillsShFile = useMarketplaceStore((state) => state.readSkillsShFile);
  const resolveSkillsShUrl = useMarketplaceStore((state) => state.resolveSkillsShUrl);
  const [content, setContent] = useState("");
  const [contentError, setContentError] = useState<string | null>(null);
  const [isLoadingContent, setIsLoadingContent] = useState(false);
  const [fileTree, setFileTree] = useState<DirectoryTreeEntry[]>([]);
  const [isFileTreeLoading, setIsFileTreeLoading] = useState(false);
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const [resolvedDownloadUrl, setResolvedDownloadUrl] = useState<string | null>(null);
  const [viewMode, setViewMode] =
    useState<MarketplaceDetailViewMode>("markdown");
  const [explanation, setExplanation] = useState<string | null>(null);
  const [isExplaining, setIsExplaining] = useState(false);
  const [explanationError, setExplanationError] = useState<string | null>(null);
  const explanationRequestRef = useRef(0);
  const explanationUnlistenRef = useRef<(() => void) | null>(null);
  const browserMode = !isTauriRuntime();
  const isSkillsShDetail = skill?.remoteKind === "skills_sh" && !!skill.source && !!skill.skillId;

  const cleanupExplanation = useCallback(() => {
    explanationUnlistenRef.current?.();
    explanationUnlistenRef.current = null;
  }, []);

  const fetchContent = useCallback(async (pathOverride?: string | null) => {
    if (!open || !skill?.downloadUrl) {
      return;
    }

    setIsLoadingContent(true);
    setContentError(null);
    try {
      if (isSkillsShDetail && skill.source && pathOverride) {
        setContent(await readSkillsShFile(skill.source, pathOverride));
        setSelectedFilePath(pathOverride);
        return;
      }

    const response = await fetch(resolvedDownloadUrl ?? skill.downloadUrl);
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
  }, [
    isSkillsShDetail,
    open,
    readSkillsShFile,
    resolvedDownloadUrl,
    skill?.downloadUrl,
    skill?.source,
  ]);

  useEffect(() => {
    if (!open || !skill?.downloadUrl) {
      return;
    }
    setContent("");
    setContentError(null);
    setExplanation(null);
    setExplanationError(null);
    setFileTree([]);
    setSelectedFilePath(null);
    setResolvedDownloadUrl(null);
    setViewMode("markdown");
    if (!isSkillsShDetail) {
      void fetchContent();
    }
  }, [fetchContent, isSkillsShDetail, open, skill?.downloadUrl]);

  useEffect(() => {
    if (!open || !isSkillsShDetail || !skill?.source || !skill.skillId) {
      return;
    }

    let cancelled = false;
    const source = skill.source;
    const skillId = skill.skillId;
    setIsFileTreeLoading(true);
    setFileTree([]);

    async function loadSkillsShFiles() {
      try {
        const [resolvedUrl, entries] = await Promise.all([
          resolveSkillsShUrl(source, skillId),
          browseSkillsShDirectory(source, skillId),
        ]);
        if (cancelled) return;
        const tree = buildDirectoryTreeFromSkillsShEntries(entries);
        setFileTree(tree);
        setIsFileTreeLoading(false);
        setResolvedDownloadUrl(resolvedUrl);
        const skillMd = entries.find((entry) => /(^|\/)SKILL\.md$/i.test(entry.path));
        if (skillMd) {
          setSelectedFilePath(skillMd.path);
          void fetchContent(skillMd.path);
        }
      } catch (error) {
        if (!cancelled) {
          setIsFileTreeLoading(false);
          setContentError(String(error));
        }
      }
    }

    void loadSkillsShFiles();
    return () => {
      cancelled = true;
    };
  }, [
    browseSkillsShDirectory,
    fetchContent,
    isSkillsShDetail,
    open,
    resolveSkillsShUrl,
    skill?.skillId,
    skill?.source,
  ]);

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
    fileTree,
    isExplaining,
    isFileTreeLoading,
    isLoadingContent,
    openFileTreePath: (path: string) => {
      const findNode = (nodes: typeof fileTree): boolean =>
        nodes.some((node) => {
          if (node.path === path) {
            return node.file_type !== "dir";
          }
          return findNode(node.children);
        });
      if (findNode(fileTree)) {
        void fetchContent(path);
      }
    },
    retryContent: () => fetchContent(selectedFilePath),
    resolvedDownloadUrl,
    selectedFilePath,
    setViewMode,
    viewMode,
  };
}
