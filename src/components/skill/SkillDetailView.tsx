import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type Ref,
} from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { Bot, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { CentralUpdateConfirmDialog } from "@/components/central/CentralUpdateConfirmDialog";
import { CollectionPickerDialog } from "@/components/collection/CollectionPickerDialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { SkillDetailPreview } from "@/components/skill/SkillDetailPreview";
import { SkillDetailSidebar } from "@/components/skill/SkillDetailSidebar";
import type { InstalledPlatformUninstallRequest } from "@/components/skill/skillDetailInstalledPlatformsModel";
import { TabToggle } from "@/components/skill/SkillDetailViewShared";
import { formatBackendError } from "@/lib/backendError";
import { openExternalUrl } from "@/lib/externalUrl";
import { parseFrontmatter } from "@/lib/frontmatter";
import { arePathsEquivalent } from "@/lib/path";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";
import { isRemoteLikeTarget } from "@/lib/targetKind";
import {
  getPlatformTargetGroups,
  getPlatformTargetMemberIds,
} from "@/lib/platformTargetGroups";
import { isTauriRuntime } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillDetailStore } from "@/stores/skillDetailStore";
import { useTargetStore } from "@/stores/targetStore";
import type {
  CentralSkillUpdateState,
  SkillDetail,
  SkillDetailRequest,
  SkillInstallation,
} from "@/types";
import type { SourceMetadata, PreviewTab } from "@/components/skill/skillDetailViewTypes";

export type { SourceMetadata } from "@/components/skill/skillDetailViewTypes";

const NOOP_ASYNC = async () => undefined;
const NOOP = () => undefined;
const EMPTY_DIRECTORY_TREE: never[] = [];

function rowAwareUninstallId(
  agentId: string,
  detail: SkillDetail | null
): string | undefined {
  return agentId === "claude-code" && detail?.source_kind === "user" && !detail.is_read_only
    ? detail.row_id
    : undefined;
}

/**
 * Shared presentation component for skill detail. Rendered by both the
 * full-page route wrapper (`SkillDetailPage`) and the list-entry drawer
 * (`SkillDetailDrawer`). This component owns:
 *   - ViewHeader (title/description/TabToggle + optional leading slot)
 *   - TwoColumnLayout (LeftPreview tab panel + RightSidebar metadata/install/collections)
 *   - CollectionPicker portal
 *
 * It does NOT render a back button, breadcrumb, or close button. Those belong
 * to the outer shell. It also does NOT call `useNavigate` / `useParams`; all
 * route/shell concerns are handled outside.
 */
export interface SkillDetailViewProps {
  /** The skill id to load from DB. Required for central skills. */
  skillId?: string;
  /** Optional platform context for source-aware detail loading. */
  agentId?: string;
  /** Optional stable row identity for duplicate platform rows. */
  rowId?: string;
  /** Direct file path to load content from. Used for non-central source skills. */
  filePath?: string;
  /** Metadata for non-central source skills (shown in sidebar). */
  sourceMetadata?: SourceMetadata;
  /** Affects local styling only, never behavior. */
  variant: "page" | "drawer";
  /** ViewHeader leftmost slot; currently null from both shells. */
  leading?: ReactNode;
  /** Drawer-only: used so the view can request its shell to close (e.g. on Esc). */
  onRequestClose?: () => void;
  /** Optional: exposes the left-preview scroll container to the outer shell. */
  scrollContainerRef?: Ref<HTMLDivElement>;
  /** Optional id applied to the ViewHeader h1 for shell-level aria-labelledby. */
  titleId?: string;
}

export function SkillDetailView({
  skillId,
  agentId,
  rowId,
  filePath,
  sourceMetadata,
  variant,
  leading = null,
  onRequestClose: _onRequestClose,
  scrollContainerRef,
  titleId,
}: SkillDetailViewProps) {
  const { t, i18n } = useTranslation();
  const isFileMode = !skillId && !!filePath;

  const detail = useSkillDetailStore((s) => s.detail);
  const storeContent = useSkillDetailStore((s) => s.content);
  const storeIsLoading = useSkillDetailStore((s) => s.isLoading);
  const installingAgentId = useSkillDetailStore((s) => s.installingAgentId);
  const error = useSkillDetailStore((s) => s.error);
  const loadDetail = useSkillDetailStore((s) => s.loadDetail);
  const installSkill = useSkillDetailStore((s) => s.installSkill);
  const uninstallSkill = useSkillDetailStore((s) => s.uninstallSkill);
  const refreshInstallations = useSkillDetailStore((s) => s.refreshInstallations);
  const storeExplanation = useSkillDetailStore((s) => s.explanation);
  const storeIsExplanationLoading = useSkillDetailStore((s) => s.isExplanationLoading);
  const isExplanationStreaming = useSkillDetailStore((s) => s.isExplanationStreaming);
  const explanationError = useSkillDetailStore((s) => s.explanationError);
  const explanationErrorInfo = useSkillDetailStore((s) => s.explanationErrorInfo);
  const loadCachedExplanation = useSkillDetailStore((s) => s.loadCachedExplanation);
  const generateExplanation = useSkillDetailStore((s) => s.generateExplanation);
  const refreshExplanation = useSkillDetailStore((s) => s.refreshExplanation);
  const reset = useSkillDetailStore((s) => s.reset);
  const fileContent = useSkillDetailStore((s) => s.fileContent);
  const fileIsLoading = useSkillDetailStore((s) => s.fileIsLoading);
  const fileExplanation = useSkillDetailStore((s) => s.fileExplanation);
  const fileIsExplaining = useSkillDetailStore((s) => s.fileIsExplaining);
  const loadFileContent = useSkillDetailStore((s) => s.loadFileContent);
  const explainFileContent = useSkillDetailStore((s) => s.explainFileContent);
  const openInFileManager = useSkillDetailStore((s) => s.openInFileManager) ?? NOOP_ASYNC;
  const directoryTree = useSkillDetailStore((s) => s.directoryTree) ?? EMPTY_DIRECTORY_TREE;
  const isDirectoryLoading = useSkillDetailStore((s) => s.isDirectoryLoading) ?? false;
  const loadDirectoryTree = useSkillDetailStore((s) => s.loadDirectoryTree) ?? NOOP_ASYNC;
  const resetFileMode = useSkillDetailStore((s) => s.resetFileMode) ?? NOOP;
  const projectsUsingSkill = useSkillDetailStore((s) => s.projectsUsingSkill) ?? [];
  const isLoadingProjectsUsingSkill =
    useSkillDetailStore((s) => s.isLoadingProjectsUsingSkill) ?? false;
  const loadProjectsUsingSkill =
    useSkillDetailStore((s) => s.loadProjectsUsingSkill) ?? NOOP_ASYNC;

  const agents = usePlatformStore((s) => s.agents);
  const categoryVisibility =
    usePlatformStore((s) => s.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const refreshCounts = usePlatformStore((s) => s.refreshCounts);
  const activeTarget = useTargetStore((s) => s.activeTarget);
  const repositories = useCentralSkillsStore((s) => s.repositories);
  const tags = useCentralSkillsStore((s) => s.tags);
  const isMetadataUpdating = useCentralSkillsStore((s) => s.isMetadataUpdating);
  const updateStatuses = useCentralSkillsStore((s) => s.updateStatuses);
  const isCheckingUpdates = useCentralSkillsStore((s) => s.isCheckingUpdates);
  const updatingSkillIds = useCentralSkillsStore((s) => s.updatingSkillIds);
  const checkSkillUpdates = useCentralSkillsStore((s) => s.checkSkillUpdates);
  const updateSkills = useCentralSkillsStore((s) => s.updateSkills);
  const assignSkillsToRepository = useCentralSkillsStore((s) => s.assignSkillsToRepository);
  const assignSkillTags = useCentralSkillsStore((s) => s.assignSkillTags);

  const detailRequest = useMemo<SkillDetailRequest | null>(
    () => (skillId ? { skillId, agentId, rowId } : null),
    [skillId, agentId, rowId]
  );
  const explanationRequestKey = useMemo(() => {
    if (!skillId) {
      return null;
    }
    return detail?.row_id ?? rowId ?? skillId;
  }, [detail?.row_id, rowId, skillId]);

  const content = isFileMode ? fileContent : storeContent;
  const isLoading = isFileMode ? fileIsLoading : storeIsLoading;
  const explanation = isFileMode ? fileExplanation : storeExplanation;
  const isExplanationLoading = isFileMode ? fileIsExplaining : storeIsExplanationLoading;
  const isRemoteTarget = isRemoteLikeTarget(activeTarget);

  const [activeTab, setActiveTab] = useState<PreviewTab>("markdown");
  const [isCollectionPickerOpen, setIsCollectionPickerOpen] = useState(false);
  const [showErrorDetails, setShowErrorDetails] = useState(false);
  const [selectedRepositoryId, setSelectedRepositoryId] = useState("");
  const [selectedTagId, setSelectedTagId] = useState("");
  const [isUpdateConfirmOpen, setIsUpdateConfirmOpen] = useState(false);
  const [pendingUpdateStates, setPendingUpdateStates] = useState<CentralSkillUpdateState[]>([]);
  const [pendingPlatformUninstall, setPendingPlatformUninstall] =
    useState<InstalledPlatformUninstallRequest | null>(null);
  const addToCollectionButtonRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (detail?.is_read_only && isCollectionPickerOpen) {
      setIsCollectionPickerOpen(false);
    }
  }, [detail?.is_read_only, isCollectionPickerOpen]);

  useEffect(() => {
    setSelectedRepositoryId(detail?.repository?.id ?? "");
  }, [detail?.repository?.id]);

  const fetchFileContent = useCallback(async () => {
    if (!filePath) {
      return;
    }
    await loadFileContent(filePath);
  }, [filePath, loadFileContent]);

  useEffect(() => {
    if (isFileMode) {
      resetFileMode();
      setActiveTab("markdown");
      void fetchFileContent();
    }
  }, [isFileMode, fetchFileContent, resetFileMode]);

  useEffect(() => {
    if (detailRequest) {
      loadDetail(detailRequest);
    }
    return () => {
      reset();
    };
  }, [detailRequest, loadDetail, reset]);

  const directoryPath = useMemo(() => {
    if (isFileMode) {
      return sourceMetadata?.dirPath ?? null;
    }
    return detail?.dir_path ?? null;
  }, [detail?.dir_path, sourceMetadata?.dirPath, isFileMode]);

  useEffect(() => {
    if (!directoryPath) {
      void loadDirectoryTree("");
      return;
    }
    void loadDirectoryTree(directoryPath);
  }, [directoryPath, loadDirectoryTree]);

  useEffect(() => {
    if (skillId && detail?.is_central && !detail.is_read_only) {
      void loadProjectsUsingSkill(skillId);
    }
  }, [skillId, detail?.is_central, detail?.is_read_only, loadProjectsUsingSkill]);

  useLayoutEffect(() => {
    if (explanationRequestKey && storeContent) {
      loadCachedExplanation(explanationRequestKey, i18n.language);
    }
  }, [explanationRequestKey, storeContent, i18n.language, loadCachedExplanation]);

  const targetAgents = getPlatformTargetGroups(agents, categoryVisibility);
  const lobsterAgents = targetAgents.filter((agent) => agent.category === "lobster");
  const codingAgents = targetAgents.filter((agent) => agent.category !== "lobster");
  const sharedRootAgentIds = useMemo(() => {
    if (!detail?.is_central) {
      return new Set<string>();
    }

    const centralDir = agents.find((agent) => agent.id === "central")?.global_skills_dir;
    return new Set(
      targetAgents
        .filter((agent) => agent.id !== "central")
        .filter((agent) => arePathsEquivalent(agent.global_skills_dir, centralDir))
        .flatMap((agent) => getPlatformTargetMemberIds(agent))
    );
  }, [agents, detail?.is_central, targetAgents]);

  const installationMap = useMemo(
    () => new Map<string, SkillInstallation>((detail?.installations ?? []).map((inst) => [inst.agent_id, inst])),
    [detail?.installations]
  );
  const skillCollections = detail?.collections ?? [];
  const updateStatus = detail?.id ? updateStatuses[detail.id] : undefined;
  const isUpdatingThisSkill = detail?.id ? updatingSkillIds.includes(detail.id) : false;

  const handleToggle = useCallback(
    async (targetAgentId: string) => {
      if (!skillId || detail?.is_read_only || sharedRootAgentIds.has(targetAgentId)) {
        return;
      }

      const isInstalled = installationMap.has(targetAgentId);
      const uninstallRowId = rowAwareUninstallId(targetAgentId, detail);
      try {
        if (isInstalled) {
          if (uninstallRowId) {
            await uninstallSkill(skillId, targetAgentId, uninstallRowId);
          } else {
            await uninstallSkill(skillId, targetAgentId);
          }
        } else {
          await installSkill(skillId, targetAgentId);
        }
        await Promise.all([refreshCounts(), refreshInstallations(skillId)]);
      } catch (err) {
        toast.error(
          isInstalled
            ? t("detail.uninstallError", { error: String(err) })
            : t("detail.installError", { error: String(err) })
        );
      }
    },
    [
      detail,
      installationMap,
      installSkill,
      refreshCounts,
      refreshInstallations,
      sharedRootAgentIds,
      skillId,
      t,
      uninstallSkill,
    ]
  );

  const handleCollectionAdded = useCallback(() => {
    if (detailRequest) {
      loadDetail(detailRequest);
    }
  }, [detailRequest, loadDetail]);

  const handleCollectionPickerOpenChange = useCallback((open: boolean) => {
    setIsCollectionPickerOpen(open);
    if (!open) {
      queueMicrotask(() => {
        addToCollectionButtonRef.current?.focus();
      });
    }
  }, []);

  const handleAssignRepository = useCallback(async () => {
    if (!skillId || !selectedRepositoryId || !detailRequest) {
      return;
    }
    try {
      await assignSkillsToRepository([skillId], selectedRepositoryId);
      await loadDetail(detailRequest);
      toast.success(t("central.repositoryAssigned", { count: 1 }));
    } catch (err) {
      toast.error(t("central.metadataError", { error: formatBackendError(err, t) }));
    }
  }, [assignSkillsToRepository, detailRequest, loadDetail, selectedRepositoryId, skillId, t]);

  const handleAssignTag = useCallback(async () => {
    if (!skillId || !selectedTagId || !detailRequest) {
      return;
    }
    try {
      await assignSkillTags([skillId], [selectedTagId]);
      await loadDetail(detailRequest);
      toast.success(t("central.tagsAssigned", { count: 1 }));
      setSelectedTagId("");
    } catch (err) {
      toast.error(t("central.metadataError", { error: formatBackendError(err, t) }));
    }
  }, [assignSkillTags, detailRequest, loadDetail, selectedTagId, skillId, t]);

  const handleCheckSkillUpdate = useCallback(async () => {
    if (!skillId) {
      return;
    }
    try {
      const states = await checkSkillUpdates([skillId]);
      const updatableStates = states.filter(
        (state) => state.status === "update_available"
      );
      if (updatableStates.length > 0) {
        setPendingUpdateStates(updatableStates);
        setIsUpdateConfirmOpen(true);
      }
      toast.success(t("central.updateCheckOneFinished"));
    } catch (err) {
      toast.error(t("central.updateCheckError", { error: String(err) }));
    }
  }, [checkSkillUpdates, skillId, t]);

  const handleUpdateSkill = useCallback(async () => {
    if (!skillId || updateStatus?.status !== "update_available") {
      return;
    }
    setPendingUpdateStates([updateStatus]);
    setIsUpdateConfirmOpen(true);
  }, [skillId, updateStatus]);

  const handleConfirmUpdateSkill = useCallback(async (skillIds: string[]) => {
    if (skillIds.length === 0) {
      return;
    }
    try {
      const result = await updateSkills(skillIds);
      if (result.succeeded.length > 0 && detailRequest) {
        await loadDetail(detailRequest);
      }
      toast.success(
        t("central.updateFinished", {
          succeeded: result.succeeded.length,
          failed: result.failed.length,
          skipped: result.skipped.length,
        })
      );
      setIsUpdateConfirmOpen(false);
      setPendingUpdateStates([]);
    } catch (err) {
      toast.error(t("central.updateError", { error: String(err) }));
      throw err;
    }
  }, [detailRequest, loadDetail, t, updateSkills]);

  const handleGenerateExplanation = useCallback(() => {
    if (isFileMode && content) {
      void explainFileContent(content);
      return;
    }
    if (explanationRequestKey && content) {
      generateExplanation(explanationRequestKey, content, i18n.language);
    }
  }, [content, explainFileContent, explanationRequestKey, generateExplanation, i18n.language, isFileMode]);

  const handleRefreshExplanation = useCallback(() => {
    if (isFileMode && content) {
      handleGenerateExplanation();
      return;
    }
    if (explanationRequestKey && content) {
      refreshExplanation(explanationRequestKey, content, i18n.language);
    }
  }, [content, explanationRequestKey, handleGenerateExplanation, i18n.language, isFileMode, refreshExplanation]);

  const handleOpenSourcePath = useCallback(async () => {
    if (!sourceMetadata) {
      return;
    }
    try {
      if (isRemoteTarget) {
        await navigator.clipboard.writeText(sourceMetadata.dirPath);
        toast.success(t("targets.pathCopied"));
        return;
      }
      await openInFileManager(sourceMetadata.dirPath);
    } catch {
      // silently ignore
    }
  }, [sourceMetadata, isRemoteTarget, openInFileManager, t]);

  const handleOpenDetailDirectory = useCallback(async () => {
    if (!detail?.dir_path) {
      return;
    }
    try {
      if (isRemoteTarget) {
        await navigator.clipboard.writeText(detail.dir_path);
        toast.success(t("targets.pathCopied"));
        return;
      }
      await openInFileManager(detail.dir_path);
    } catch {
      // keep opener failures non-blocking; the file tree remains usable
    }
  }, [detail?.dir_path, isRemoteTarget, openInFileManager, t]);

  const handleOpenGitHubRepository = useCallback(async (url: string) => {
    try {
      await openExternalUrl(url);
    } catch (err) {
      toast.error(
        t("detail.openGithubRepoError", {
          error: formatBackendError(err, t),
        })
      );
    }
  }, [t]);

  const handleConfirmPlatformUninstall = useCallback(async () => {
    if (!skillId || !detail || !pendingPlatformUninstall) {
      return;
    }

    try {
      const uninstallRowId = rowAwareUninstallId(
        pendingPlatformUninstall.agentId,
        detail
      );
      if (uninstallRowId) {
        await uninstallSkill(skillId, pendingPlatformUninstall.agentId, uninstallRowId);
      } else {
        await uninstallSkill(skillId, pendingPlatformUninstall.agentId);
      }
      await Promise.all([refreshCounts(), refreshInstallations(skillId)]);
      setPendingPlatformUninstall(null);
    } catch (err) {
      toast.error(
        t("detail.uninstallError", {
          error: formatBackendError(err, t),
        })
      );
    }
  }, [
    detail,
    pendingPlatformUninstall,
    refreshCounts,
    refreshInstallations,
    skillId,
    t,
    uninstallSkill,
  ]);

  const handleOpenFileTreePath = useCallback(async (path: string) => {
    if (!path) {
      return;
    }
    try {
      if (isRemoteTarget) {
        await navigator.clipboard.writeText(path);
        toast.success(t("targets.pathCopied"));
        return;
      }
      await openInFileManager(path);
    } catch {
      // ignore opener errors inside the inspector
    }
  }, [isRemoteTarget, openInFileManager, t]);

  const { frontmatterRaw, frontmatterData, body: markdownContent } = content
    ? parseFrontmatter(content)
    : { frontmatterRaw: "", frontmatterData: {}, body: "" };
  const isBrowserFallback = !isTauriRuntime() && !isLoading && !detail && !error && !isFileMode;
  const effectiveName = isFileMode
    ? (sourceMetadata?.name ?? "")
    : (detail?.name ?? detailRequest?.skillId ?? "");
  const effectiveDescription = isFileMode ? sourceMetadata?.description : detail?.description;
  const hasData = isFileMode ? content !== null : !!detail;
  const previewStackClassName =
    variant === "page" ? "mx-auto w-full max-w-5xl space-y-5" : "mx-auto w-full max-w-4xl space-y-5";

  return (
    <div className={cn("flex h-full flex-col", variant === "drawer" && "min-h-0")}>
      <div className="flex shrink-0 items-center gap-3 border-b border-border px-6 py-3">
        {leading}
        <div className="min-w-0 flex-1">
          <h1 id={titleId} className="truncate text-lg font-semibold">
            {isLoading ? (skillId ?? sourceMetadata?.name ?? "") : effectiveName}
          </h1>
          {effectiveDescription && (
            <p className="mt-0.5 truncate text-xs text-muted-foreground">{effectiveDescription}</p>
          )}
        </div>
        <TabToggle activeTab={activeTab} onChange={setActiveTab} />
      </div>

      <div className="min-h-0 flex-1 overflow-hidden">
        {isLoading && (
          <div className="flex h-full items-center justify-center gap-2 text-muted-foreground">
            <Loader2 className="size-5 animate-spin" />
            <span className="text-sm">{t("detail.loading")}</span>
          </div>
        )}

        {!isLoading && error && (
          <div className="flex h-full items-center justify-center">
            <div className="space-y-2 text-center">
              <p className="text-sm text-destructive-text">{error}</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => detailRequest && loadDetail(detailRequest)}
              >
                {t("detail.retry")}
              </Button>
            </div>
          </div>
        )}

        {!isLoading && !error && isBrowserFallback && (
          <div className="flex h-full items-center justify-center">
            <div className="max-w-md space-y-3 px-6 text-center">
              <Bot className="mx-auto size-8 text-muted-foreground" />
              <div className="space-y-1">
                <p className="text-sm font-medium">{t("detail.browserFallbackTitle")}</p>
                <p className="text-sm text-muted-foreground">{t("detail.browserFallbackDesc")}</p>
              </div>
            </div>
          </div>
        )}

        {!isLoading && !error && hasData && (
          <div className="flex h-full min-w-0 flex-col lg:flex-row" data-testid="skill-detail-two-column-layout">
            <SkillDetailPreview
              activeTab={activeTab}
              scrollContainerRef={scrollContainerRef}
              previewStackClassName={previewStackClassName}
              frontmatterRaw={frontmatterRaw}
              frontmatterData={frontmatterData}
              markdownContent={markdownContent}
              content={content}
              explanation={explanation}
              isExplanationLoading={isExplanationLoading}
              isExplanationStreaming={isExplanationStreaming}
              explanationError={explanationError}
              explanationErrorInfo={explanationErrorInfo}
              showErrorDetails={showErrorDetails}
              onToggleErrorDetails={() => setShowErrorDetails((value) => !value)}
              onGenerateExplanation={handleGenerateExplanation}
              onRefreshExplanation={handleRefreshExplanation}
            />
            <SkillDetailSidebar
              isFileMode={isFileMode}
              sourceMetadata={sourceMetadata}
              detail={detail}
              isRemoteTarget={isRemoteTarget}
              onOpenSourcePath={handleOpenSourcePath}
              onOpenDetailDirectory={handleOpenDetailDirectory}
              directoryTree={directoryTree}
              isDirectoryLoading={isDirectoryLoading}
              onOpenFileTreePath={handleOpenFileTreePath}
              updateStatus={updateStatus}
              isCheckingUpdates={isCheckingUpdates}
              isUpdatingThisSkill={isUpdatingThisSkill}
              onCheckSkillUpdate={handleCheckSkillUpdate}
              onUpdateSkill={handleUpdateSkill}
              repositories={repositories}
              tags={tags}
              isMetadataUpdating={isMetadataUpdating}
              selectedRepositoryId={selectedRepositoryId}
              onSelectedRepositoryChange={setSelectedRepositoryId}
              onAssignRepository={handleAssignRepository}
              selectedTagId={selectedTagId}
              onSelectedTagChange={setSelectedTagId}
              onAssignTag={handleAssignTag}
              targetAgents={targetAgents}
              lobsterAgents={lobsterAgents}
              codingAgents={codingAgents}
              installationMap={installationMap}
              sharedRootAgentIds={sharedRootAgentIds}
              installingAgentId={installingAgentId}
              onToggleInstall={(targetAgentId) => {
                void handleToggle(targetAgentId);
              }}
              onOpenGitHubRepository={(url) => {
                void handleOpenGitHubRepository(url);
              }}
              onRequestUninstallInstalledPlatform={setPendingPlatformUninstall}
              skillCollections={skillCollections}
              addToCollectionButtonRef={addToCollectionButtonRef}
              onOpenCollectionPicker={() => setIsCollectionPickerOpen(true)}
              projectsUsingSkill={projectsUsingSkill}
              isLoadingProjectsUsingSkill={isLoadingProjectsUsingSkill}
            />
          </div>
        )}
      </div>

      {skillId && !detail?.is_read_only && (
        <CollectionPickerDialog
          open={isCollectionPickerOpen}
          onOpenChange={handleCollectionPickerOpenChange}
          skillId={skillId}
          currentCollectionIds={skillCollections.map((collection) => collection.id)}
          onAdded={handleCollectionAdded}
        />
      )}

      <CentralUpdateConfirmDialog
        open={isUpdateConfirmOpen}
        onOpenChange={(open) => {
          setIsUpdateConfirmOpen(open);
          if (!open) {
            setPendingUpdateStates([]);
          }
        }}
        states={pendingUpdateStates}
        skills={detail ? [detail] : []}
        isApplying={isUpdatingThisSkill}
        onConfirm={handleConfirmUpdateSkill}
      />

      <Dialog
        open={pendingPlatformUninstall !== null}
        onOpenChange={(open) => {
          if (!open) {
            setPendingPlatformUninstall(null);
          }
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{t("detail.uninstallPlatformTitle")}</DialogTitle>
            <DialogDescription>
              {t("detail.uninstallPlatformDesc", {
                skill: detail?.name ?? skillId ?? "",
                platform: pendingPlatformUninstall?.displayName ?? "",
              })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setPendingPlatformUninstall(null)}
            >
              {t("common.cancel")}
            </Button>
            <Button
              type="button"
              variant="destructive"
              data-testid="confirm-detail-platform-uninstall"
              disabled={
                !!pendingPlatformUninstall
                && installingAgentId === pendingPlatformUninstall.agentId
              }
              onClick={() => {
                void handleConfirmPlatformUninstall();
              }}
            >
              {t("detail.uninstallPlatformConfirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
