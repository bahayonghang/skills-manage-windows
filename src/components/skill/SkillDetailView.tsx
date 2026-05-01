import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type Ref, type ReactNode } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import {
  Tag,
  Plus,
  FileText,
  Code,
  Bot,
  RefreshCw,
  Loader2,
  ChevronDown,
  ChevronRight,
  Monitor,
  FolderOpen,
  Lock,
  Download,
  Copy,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { SkillFrontmatterCard } from "@/components/skill/SkillFrontmatterCard";
import { SkillMarkdownRenderer } from "@/components/skill/SkillMarkdownRenderer";
import { parseFrontmatter } from "@/lib/frontmatter";
import { useSkillDetailStore } from "@/stores/skillDetailStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useTargetStore } from "@/stores/targetStore";
import { CollectionPickerDialog } from "@/components/collection/CollectionPickerDialog";
import { AgentWithStatus, ClaudeSourceKind, SkillDetailRequest, SkillInstallation } from "@/types";
import { cn } from "@/lib/utils";
import { isTauriRuntime } from "@/lib/tauri";
import { arePathsEquivalent } from "@/lib/path";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
} from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberIds,
  getPlatformTargetMemberNames,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";

// ─── Section Label ─────────────────────────────────────────────────────────────

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/80 mb-2">
      {children}
    </div>
  );
}

// ─── MetadataRow (compact) ───────────────────────────────────────────────────

function MetadataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="space-y-0.5">
      <div className="text-[10px] text-muted-foreground/70 uppercase tracking-wider">{label}</div>
      <div className="font-mono text-xs text-foreground break-all leading-relaxed">
        {value}
      </div>
    </div>
  );
}

function SourceOriginBadge({ originKind }: { originKind: ClaudeSourceKind }) {
  const { t, i18n } = useTranslation();
  const isPlugin = originKind === "plugin";

  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium ring-1",
        isPlugin
          ? "bg-amber-500/10 text-amber-700 ring-amber-500/20 dark:text-amber-300"
          : "bg-sky-500/10 text-sky-700 ring-sky-500/20 dark:text-sky-300"
      )}
    >
      {isPlugin
        ? t("platform.originPlugin", {
            defaultValue: i18n.language.startsWith("zh") ? "插件来源" : "Plugin source",
          })
        : t("platform.originUser", {
            defaultValue: i18n.language.startsWith("zh") ? "用户来源" : "User source",
          })}
    </span>
  );
}

function ReadOnlySourceBadge() {
  const { t, i18n } = useTranslation();

  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground ring-1 ring-border/70">
      <Lock className="size-3 shrink-0" />
      {t("detail.readOnlySource", {
        defaultValue: i18n.language.startsWith("zh") ? "只读来源" : "Read-only source",
      })}
    </span>
  );
}

// ─── Platform Toggle Icon (compact install/uninstall) ─────────────────────────

interface PlatformToggleIconProps {
  agent: AgentWithStatus;
  skillName: string;
  isInstalled: boolean;
  isLoading: boolean;
  isLocked: boolean;
  onToggle: () => void;
}

function PlatformToggleIcon({ agent, skillName, isInstalled, isLoading, isLocked, onToggle }: PlatformToggleIconProps) {
  const { t } = useTranslation();
  const displayName = isUniversalPlatformTarget(agent)
    ? t("platformTargets.universalShortLabel")
    : agent.display_name;
  const memberNames = getPlatformTargetMemberNames(agent).join(", ");
  const title = isLocked
    ? `${displayName} - ${t("platformTargets.alwaysIncluded")} - ${memberNames}`
    : `${displayName}${isInstalled ? ` - ${t("central.linked")}` : ""}`;

  return (
    <button
      className={cn(
        "p-1.5 rounded-md transition-colors cursor-pointer",
        isLocked
          ? "text-primary cursor-default"
          : isInstalled
            ? "text-primary hover:bg-primary/15"
            : "text-muted-foreground/40 hover:bg-muted/60 hover:text-muted-foreground",
        isLoading && "animate-pulse pointer-events-none"
      )}
      title={title}
      aria-label={
        isLocked
          ? title
          : t("central.toggleInstallLabel", { platform: displayName, skill: skillName })
      }
      disabled={isLoading || isLocked}
      onClick={onToggle}
    >
      <PlatformIcon agentId={agent.id} className="size-4 shrink-0" size={16} />
    </button>
  );
}

// ─── Tab Toggle ───────────────────────────────────────────────────────────────

type PreviewTab = "markdown" | "raw" | "explanation";

interface TabToggleProps {
  activeTab: PreviewTab;
  onChange: (tab: PreviewTab) => void;
}

function TabToggle({ activeTab, onChange }: TabToggleProps) {
  const { t } = useTranslation();
  return (
    <div className="flex border border-border rounded-lg p-0.5 gap-0.5 bg-muted/40">
      <button
        role="tab"
        aria-selected={activeTab === "markdown"}
        onClick={() => onChange("markdown")}
        className={cn(
          "flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-colors",
          activeTab === "markdown"
            ? "bg-background text-foreground shadow-sm"
            : "text-muted-foreground hover:text-foreground"
        )}
      >
        <FileText className="size-3.5" />
        {t("detail.markdown")}
      </button>
      <button
        role="tab"
        aria-selected={activeTab === "raw"}
        onClick={() => onChange("raw")}
        className={cn(
          "flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-colors",
          activeTab === "raw"
            ? "bg-background text-foreground shadow-sm"
            : "text-muted-foreground hover:text-foreground"
        )}
      >
        <Code className="size-3.5" />
        {t("detail.rawSource")}
      </button>
      <button
        role="tab"
        aria-selected={activeTab === "explanation"}
        onClick={() => onChange("explanation")}
        className={cn(
          "flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-colors",
          activeTab === "explanation"
            ? "bg-background text-foreground shadow-sm"
            : "text-muted-foreground hover:text-foreground"
        )}
      >
        <Bot className="size-3.5" />
        {t("detail.aiExplanation")}
      </button>
    </div>
  );
}

const inspectorCardClassName =
  "min-w-0 space-y-3 overflow-x-hidden rounded-lg border border-border/70 bg-muted/20 p-3";
const inspectorFieldLabelClassName =
  "text-[10px] font-medium uppercase tracking-wider text-muted-foreground/70";
const inspectorSelectClassName =
  "w-full min-w-0 rounded-md border border-border bg-background px-2 py-1.5 text-xs";
const inspectorActionButtonClassName = "w-full justify-center";

// ─── SkillDetailView ──────────────────────────────────────────────────────────

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
export interface DiscoverMetadata {
  name: string;
  description?: string;
  platformName: string;
  projectName: string;
  filePath: string;
  dirPath: string;
  isAlreadyCentral: boolean;
}

export interface SkillDetailViewProps {
  /** The skill id to load from DB. Required for central skills. */
  skillId?: string;
  /** Optional platform context for source-aware detail loading. */
  agentId?: string;
  /** Optional stable row identity for duplicate platform rows. */
  rowId?: string;
  /** Direct file path to load content from. Used for discover non-central skills. */
  filePath?: string;
  /** Metadata for discover non-central skills (shown in sidebar). */
  discoverMetadata?: DiscoverMetadata;
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
  discoverMetadata,
  variant,
  leading = null,
  onRequestClose: _onRequestClose,
  scrollContainerRef,
  titleId,
}: SkillDetailViewProps) {
  const { t, i18n } = useTranslation();
  const isFileMode = !skillId && !!filePath;

  // Store data (used in skillId mode)
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

  // Platform agents (loaded at app init)
  const agents = usePlatformStore((s) => s.agents);
  const categoryVisibility = usePlatformStore((s) => s.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
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

  // Local state for filePath mode
  const fileContent = useSkillDetailStore((s) => s.fileContent);
  const fileIsLoading = useSkillDetailStore((s) => s.fileIsLoading);
  const fileExplanation = useSkillDetailStore((s) => s.fileExplanation);
  const fileIsExplaining = useSkillDetailStore((s) => s.fileIsExplaining);
  const loadFileContent = useSkillDetailStore((s) => s.loadFileContent);
  const explainFileContent = useSkillDetailStore((s) => s.explainFileContent);
  const openInFileManager = useSkillDetailStore((s) => s.openInFileManager);
  const resetFileMode = useSkillDetailStore((s) => s.resetFileMode);
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

  // Unified accessors
  const content = isFileMode ? fileContent : storeContent;
  const isLoading = isFileMode ? fileIsLoading : storeIsLoading;
  const explanation = isFileMode ? fileExplanation : storeExplanation;
  const isExplanationLoading = isFileMode ? fileIsExplaining : storeIsExplanationLoading;
  const isRemoteTarget = activeTarget.kind === "ssh";

  // Local UI state
  const [activeTab, setActiveTab] = useState<PreviewTab>("markdown");
  const [isCollectionPickerOpen, setIsCollectionPickerOpen] = useState(false);
  const [showErrorDetails, setShowErrorDetails] = useState(false);
  const [selectedRepositoryId, setSelectedRepositoryId] = useState("");
  const [selectedTagId, setSelectedTagId] = useState("");
  const addToCollectionButtonRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (detail?.is_read_only && isCollectionPickerOpen) {
      setIsCollectionPickerOpen(false);
    }
  }, [detail?.is_read_only, isCollectionPickerOpen]);

  useEffect(() => {
    setSelectedRepositoryId(detail?.repository?.id ?? "");
  }, [detail?.repository?.id]);

  // ── File mode: load content from path ─────────────────────────────────
  const fetchFileContent = useCallback(async () => {
    if (!filePath) return;
    await loadFileContent(filePath);
  }, [filePath, loadFileContent]);

  useEffect(() => {
    if (isFileMode) {
      resetFileMode();
      setActiveTab("markdown");
      void fetchFileContent();
    }
  }, [isFileMode, fetchFileContent, resetFileMode]);

  // ── Store mode: load detail by skillId ────────────────────────────────
  useEffect(() => {
    if (detailRequest) {
      loadDetail(detailRequest);
    }
    return () => {
      reset();
    };
  }, [detailRequest, loadDetail, reset]);

  useLayoutEffect(() => {
    if (explanationRequestKey && storeContent) {
      loadCachedExplanation(explanationRequestKey, i18n.language);
    }
  }, [explanationRequestKey, storeContent, i18n.language, loadCachedExplanation]);

  // ── Derived values ───────────────────────────────────────────────────────

  const targetAgents = getPlatformTargetGroups(agents, categoryVisibility);
  const lobsterAgents = targetAgents.filter((a) => a.category === "lobster");
  const codingAgents = targetAgents.filter((a) => a.category !== "lobster");
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

  const installationMap = new Map<string, SkillInstallation>(
    (detail?.installations ?? []).map((inst) => [inst.agent_id, inst])
  );
  const skillCollections = detail?.collections ?? [];
  const updateStatus = detail?.id ? updateStatuses[detail.id] : undefined;
  const isUpdatingThisSkill = detail?.id ? updatingSkillIds.includes(detail.id) : false;

  // ── Handlers ─────────────────────────────────────────────────────────────

  async function handleToggle(agentId: string) {
    if (!skillId || detail?.is_read_only || sharedRootAgentIds.has(agentId)) return;
    const isInstalled = installationMap.has(agentId);
    try {
      if (isInstalled) {
        await uninstallSkill(skillId, agentId);
      } else {
        await installSkill(skillId, agentId);
      }
      await Promise.all([
        refreshCounts(),
        refreshInstallations(skillId),
      ]);
    } catch (err) {
      toast.error(
        isInstalled
          ? t("detail.uninstallError", { error: String(err) })
          : t("detail.installError", { error: String(err) })
      );
    }
  }

  function handleCollectionAdded() {
    if (detailRequest) {
      loadDetail(detailRequest);
    }
  }

  function handleCollectionPickerOpenChange(open: boolean) {
    setIsCollectionPickerOpen(open);
    if (!open) {
      queueMicrotask(() => {
        addToCollectionButtonRef.current?.focus();
      });
    }
  }

  async function handleAssignRepository() {
    if (!skillId || !selectedRepositoryId || !detailRequest) return;
    try {
      await assignSkillsToRepository([skillId], selectedRepositoryId);
      await loadDetail(detailRequest);
      toast.success(t("central.repositoryAssigned", { count: 1 }));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleAssignTag() {
    if (!skillId || !selectedTagId || !detailRequest) return;
    try {
      await assignSkillTags([skillId], [selectedTagId]);
      await loadDetail(detailRequest);
      toast.success(t("central.tagsAssigned", { count: 1 }));
      setSelectedTagId("");
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleCheckSkillUpdate() {
    if (!skillId) return;
    try {
      await checkSkillUpdates([skillId]);
      toast.success(t("central.updateCheckOneFinished"));
    } catch (err) {
      toast.error(t("central.updateCheckError", { error: String(err) }));
    }
  }

  async function handleUpdateSkill() {
    if (!skillId || updateStatus?.status !== "update_available") return;
    try {
      const result = await updateSkills([skillId]);
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
    } catch (err) {
      toast.error(t("central.updateError", { error: String(err) }));
    }
  }

  function handleGenerateExplanation() {
    if (isFileMode && content) {
      void explainFileContent(content);
      return;
    }
    if (explanationRequestKey && content) {
      generateExplanation(explanationRequestKey, content, i18n.language);
    }
  }

  function handleRefreshExplanation() {
    if (isFileMode && content) {
      handleGenerateExplanation();
      return;
    }
    if (explanationRequestKey && content) {
      refreshExplanation(explanationRequestKey, content, i18n.language);
    }
  }

  const handleOpenDiscoverPath = useCallback(async () => {
    if (!discoverMetadata) return;
    try {
      if (isRemoteTarget) {
        await navigator.clipboard.writeText(discoverMetadata.dirPath);
        toast.success(t("targets.pathCopied"));
        return;
      }
      await openInFileManager(discoverMetadata.dirPath);
    } catch {
      // silently ignore
    }
  }, [discoverMetadata, isRemoteTarget, openInFileManager, t]);

  const { frontmatterRaw, frontmatterData, body: markdownContent } = content
    ? parseFrontmatter(content)
    : { frontmatterRaw: "", frontmatterData: {}, body: "" };
  const isBrowserFallback = !isTauriRuntime() && !isLoading && !detail && !error && !isFileMode;
  const effectiveName = isFileMode
    ? (discoverMetadata?.name ?? "")
    : (detail?.name ?? detailRequest?.skillId ?? "");
  const effectiveDescription = isFileMode
    ? discoverMetadata?.description
    : detail?.description;
  const hasData = isFileMode ? content !== null : !!detail;
  const previewStackClassName = variant === "page"
    ? "mx-auto w-full max-w-5xl space-y-5"
    : "mx-auto w-full max-w-4xl space-y-5";

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div className={cn("flex flex-col h-full", variant === "drawer" && "min-h-0")}>
      {/* ── ViewHeader: leading slot + title/description + TabToggle ─────── */}
      <div className="border-b border-border px-6 py-3 flex items-center gap-3 shrink-0">
        {leading}
        <div className="min-w-0 flex-1">
          <h1 id={titleId} className="text-lg font-semibold truncate">
            {isLoading ? (skillId ?? discoverMetadata?.name ?? "") : effectiveName}
          </h1>
          {effectiveDescription && (
            <p className="text-xs text-muted-foreground truncate mt-0.5">
              {effectiveDescription}
            </p>
          )}
        </div>
        <TabToggle activeTab={activeTab} onChange={setActiveTab} />
      </div>

      {/* ── ContentArea ──────────────────────────────────────────────────── */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {/* Loading state */}
        {isLoading && (
          <div className="flex items-center justify-center h-full gap-2 text-muted-foreground">
            <Loader2 className="size-5 animate-spin" />
            <span className="text-sm">{t("detail.loading")}</span>
          </div>
        )}

        {/* Error state */}
        {!isLoading && error && (
          <div className="flex items-center justify-center h-full">
            <div className="text-center space-y-2">
              <p className="text-sm text-destructive">{error}</p>
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
          <div className="flex items-center justify-center h-full">
            <div className="text-center space-y-3 max-w-md px-6">
              <Bot className="size-8 mx-auto text-muted-foreground/60" />
              <div className="space-y-1">
                <p className="text-sm font-medium">{t("detail.browserFallbackTitle")}</p>
                <p className="text-sm text-muted-foreground">
                  {t("detail.browserFallbackDesc")}
                </p>
              </div>
            </div>
          </div>
        )}

        {/* ── TwoColumnLayout: LeftPreview + RightSidebar ────────────────── */}
        {!isLoading && !error && hasData && (
          <div
            className="flex h-full min-w-0 flex-col lg:flex-row"
            data-testid="skill-detail-two-column-layout"
          >
            {/* ── Left: SKILL.md Preview ─────────────────────────────── */}
            <div
              ref={scrollContainerRef}
              className="scrollbar-subtle flex-1 min-w-0 overflow-auto"
            >
              {activeTab === "markdown" ? (
                <div
                  className="p-6 space-y-4"
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
                      <p className="text-sm text-muted-foreground italic">
                        {t("detail.noContent")}
                      </p>
                    )}
                  </div>
                </div>
              ) : activeTab === "raw" ? (
                <pre
                  className="p-6 text-[12px] leading-5 font-mono whitespace-pre-wrap break-words text-foreground/80"
                  role="tabpanel"
                  aria-label={t("detail.rawSource")}
                >
                  {content ?? t("detail.noContent")}
                </pre>
              ) : (
                <div
                  className="p-6 space-y-4"
                  role="tabpanel"
                  aria-label={t("detail.aiExplanation")}
                >
                  <div className={previewStackClassName}>
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <h2 className="text-sm font-semibold flex items-center gap-2">
                          <Bot className="size-4 text-primary" />
                          {t("detail.aiExplanation")}
                        </h2>
                        <p className="text-xs text-muted-foreground mt-1">
                          {t("detail.aiExplanationDesc")}
                        </p>
                      </div>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={explanation ? handleRefreshExplanation : handleGenerateExplanation}
                        disabled={!content || isExplanationLoading || isExplanationStreaming}
                        className="gap-1.5"
                      >
                        {isExplanationLoading || isExplanationStreaming ? (
                          <Loader2 className="size-3.5 animate-spin" />
                        ) : explanation ? (
                          <RefreshCw className="size-3.5" />
                        ) : (
                          <Bot className="size-3.5" />
                        )}
                        {explanation ? t("detail.regenerateExplanation") : t("detail.generateExplanation")}
                      </Button>
                    </div>

                    {explanationError && (
                      <div className="rounded-md border border-destructive/30 bg-destructive/5 p-3 space-y-2">
                        <p className="text-sm text-destructive">
                          {explanationErrorInfo?.message || explanationError}
                        </p>
                        {(explanationErrorInfo?.details || explanationError !== explanationErrorInfo?.message) && (
                          <div>
                            <button
                              className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                              onClick={() => setShowErrorDetails((v) => !v)}
                            >
                              {showErrorDetails ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
                              {t("detail.showDetails")}
                            </button>
                            {showErrorDetails && (
                              <pre className="scrollbar-subtle mt-1.5 text-[11px] leading-4 font-mono text-muted-foreground whitespace-pre-wrap break-all bg-muted/30 rounded-md p-2 max-h-40 overflow-auto">
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
                      <div className="rounded-lg border border-dashed border-border p-8 text-center space-y-3">
                        <Bot className="size-8 mx-auto text-muted-foreground/60" />
                        <div>
                          <p className="text-sm font-medium">{t("detail.noExplanationTitle")}</p>
                          <p className="text-xs text-muted-foreground mt-1">
                            {t("detail.noExplanationDesc")}
                          </p>
                        </div>
                        <Button
                          size="sm"
                          onClick={handleGenerateExplanation}
                          disabled={!content || isExplanationLoading || isExplanationStreaming}
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

            {/* ── Right: Sidebar ─────────────────────────────────────── */}
            <aside
              data-testid="skill-detail-right-sidebar"
              className="scrollbar-subtle w-full min-w-0 shrink-0 overflow-x-hidden overflow-y-auto border-t border-border p-4 space-y-5 lg:w-72 lg:border-t-0 lg:border-l xl:w-80"
            >
              {isFileMode && discoverMetadata ? (
                <>
                  {/* Discover metadata */}
                  <section aria-label={t("detail.metadataRegion")}>
                    <SectionLabel>{t("detail.metadata")}</SectionLabel>
                    <div className="space-y-2.5">
                      <div className="space-y-0.5">
                        <div className="text-[10px] text-muted-foreground/70 uppercase tracking-wider">
                          {t("discover.platform")}
                        </div>
                        <div className="font-mono text-xs text-foreground break-all leading-relaxed inline-flex items-center gap-1">
                          <Monitor className="size-3.5" />
                          <span>{discoverMetadata.platformName}</span>
                        </div>
                      </div>
                      <div className="space-y-0.5">
                        <div className="text-[10px] text-muted-foreground/70 uppercase tracking-wider">
                          {t("discover.project")}
                        </div>
                        <div className="font-mono text-xs text-foreground break-all leading-relaxed inline-flex items-center gap-1">
                          <FolderOpen className="size-3.5" />
                          <span>{discoverMetadata.projectName}</span>
                        </div>
                      </div>
                      <div className="space-y-0.5">
                        <div className="text-[10px] text-muted-foreground/70 uppercase tracking-wider">
                          {t("discover.filePath")}
                        </div>
                        <button
                          type="button"
                          onClick={handleOpenDiscoverPath}
                          className="font-mono text-xs text-foreground break-all leading-relaxed hover:text-primary hover:underline cursor-pointer text-left"
                        >
                          {isRemoteTarget && <Copy className="mr-1 inline size-3 shrink-0" />}
                          {discoverMetadata.filePath}
                        </button>
                      </div>
                    </div>
                  </section>
                </>
              ) : detail ? (
                <>
                  {(detail.source_kind || detail.is_read_only) && (
                    <section
                      aria-label={t("detail.sourceStatusRegion", {
                        defaultValue: i18n.language.startsWith("zh") ? "来源状态" : "Source status",
                      })}
                    >
                      <SectionLabel>
                        {t("detail.sourceStatus", {
                          defaultValue: i18n.language.startsWith("zh") ? "来源状态" : "Source status",
                        })}
                      </SectionLabel>
                      <div className="rounded-lg border border-border/70 bg-muted/30 p-3 space-y-2">
                        <div className="flex flex-wrap items-center gap-1.5">
                          {detail.source_kind && (
                            <SourceOriginBadge originKind={detail.source_kind} />
                          )}
                          {detail.is_read_only && <ReadOnlySourceBadge />}
                        </div>
                        {detail.is_read_only ? (
                          <p className="text-xs leading-relaxed text-muted-foreground">
                            {t("detail.readOnlyDesc", {
                              defaultValue: i18n.language.startsWith("zh")
                                ? "插件安装的副本仅供查看，不能在这里安装、卸载或调整技能集。"
                                : "Plugin-installed copies are display-only here, so install, uninstall, and collection changes are unavailable.",
                            })}
                          </p>
                        ) : detail.source_kind === "user" ? (
                          <p className="text-xs leading-relaxed text-muted-foreground">
                            {t("detail.userManagedDesc", {
                              defaultValue: i18n.language.startsWith("zh")
                                ? "此 Claude 用户副本会保留正常的安装状态与技能集管理能力。"
                                : "This Claude user copy keeps the normal install-state and collection-management controls.",
                            })}
                          </p>
                        ) : null}
                      </div>
                    </section>
                  )}

                  {/* Metadata */}
                  <section aria-label={t("detail.metadataRegion")}>
                    <SectionLabel>{t("detail.metadata")}</SectionLabel>
                    <div className="space-y-2.5">
                      <MetadataRow label={t("detail.filePath")} value={detail.file_path} />
                      {detail.dir_path && (
                        <MetadataRow
                          label={t("detail.directoryPath", {
                            defaultValue: i18n.language.startsWith("zh") ? "目录路径" : "Directory path",
                          })}
                          value={detail.dir_path}
                        />
                      )}
                      {detail.canonical_path && (
                        <MetadataRow label={t("detail.canonical")} value={detail.canonical_path} />
                      )}
                      {detail.source_root && (
                        <MetadataRow
                          label={t("detail.sourceRoot", {
                            defaultValue: i18n.language.startsWith("zh") ? "来源根目录" : "Source root",
                          })}
                          value={detail.source_root}
                        />
                      )}
                      {!detail.source_kind && detail.source && (
                        <MetadataRow label={t("detail.source")} value={detail.source} />
                      )}
                      {detail.repository && (
                        <MetadataRow
                          label={t("detail.repository", {
                            defaultValue: i18n.language.startsWith("zh") ? "仓库来源" : "Repository",
                          })}
                          value={detail.repository.name}
                        />
                      )}
                      {detail.source_path && (
                        <MetadataRow
                          label={t("detail.sourcePath", {
                            defaultValue: i18n.language.startsWith("zh") ? "来源路径" : "Source path",
                          })}
                          value={detail.source_path}
                        />
                      )}
                      <MetadataRow
                        label={t("detail.scannedAt")}
                        value={new Date(detail.scanned_at).toLocaleString()}
                      />
                    </div>
                  </section>

                  {detail.is_central && !detail.is_read_only && (
                    <section aria-label={t("detail.updateStatusRegion")}>
                      <SectionLabel>{t("detail.updateStatus")}</SectionLabel>
                      <div
                        data-testid="detail-update-status-card"
                        className={inspectorCardClassName}
                      >
                        <div className="min-w-0 space-y-2">
                          <span className="inline-flex rounded-full bg-background px-2 py-0.5 text-[10px] font-medium text-muted-foreground ring-1 ring-border/70">
                            {updateStatus
                              ? t(`central.updateStatus.${updateStatus.status}`)
                              : t("central.updateStatus.not_checked")}
                          </span>
                        </div>
                        <div
                          data-testid="detail-update-actions"
                          className="grid grid-cols-2 gap-2"
                        >
                          <Button
                            type="button"
                            size="sm"
                            variant="outline"
                            disabled={isCheckingUpdates || isUpdatingThisSkill}
                            onClick={handleCheckSkillUpdate}
                            className={inspectorActionButtonClassName}
                          >
                            {isCheckingUpdates ? (
                              <Loader2 className="size-3.5 animate-spin" />
                            ) : (
                              <RefreshCw className="size-3.5" />
                            )}
                            {t("central.checkUpdatesShort")}
                          </Button>
                          <Button
                            type="button"
                            size="sm"
                            variant="secondary"
                            disabled={updateStatus?.status !== "update_available" || isUpdatingThisSkill}
                            onClick={handleUpdateSkill}
                            className={inspectorActionButtonClassName}
                          >
                            {isUpdatingThisSkill ? (
                              <Loader2 className="size-3.5 animate-spin" />
                            ) : (
                              <Download className="size-3.5" />
                            )}
                            {t("central.updateShort")}
                          </Button>
                        </div>
                        {updateStatus?.source_url && (
                          <MetadataRow label={t("detail.updateSource")} value={updateStatus.source_url} />
                        )}
                        {updateStatus?.ref && (
                          <MetadataRow label={t("detail.updateRef")} value={updateStatus.ref} />
                        )}
                        {updateStatus?.source_path && (
                          <MetadataRow label={t("detail.sourcePath")} value={updateStatus.source_path} />
                        )}
                        {updateStatus?.last_checked_at && (
                          <MetadataRow
                            label={t("detail.lastCheckedAt")}
                            value={new Date(updateStatus.last_checked_at).toLocaleString()}
                          />
                        )}
                        {updateStatus?.error && (
                          <p className="text-xs leading-relaxed text-destructive">{updateStatus.error}</p>
                        )}
                      </div>
                    </section>
                  )}

                  {detail.tags && detail.tags.length > 0 && (
                    <section aria-label={t("detail.tags", {
                      defaultValue: i18n.language.startsWith("zh") ? "分类标签" : "Tags",
                    })}>
                      <SectionLabel>
                        {t("detail.tags", {
                          defaultValue: i18n.language.startsWith("zh") ? "分类标签" : "Tags",
                        })}
                      </SectionLabel>
                      <div className="flex flex-wrap gap-1.5">
                        {detail.tags.map((tag) => (
                          <span
                            key={tag.id}
                            className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground ring-1 ring-border/70"
                            title={tag.description ?? tag.name}
                          >
                            <Tag className="size-2.5" />
                            {tag.name}
                          </span>
                        ))}
                      </div>
                    </section>
                  )}

                  {detail.is_central && !detail.is_read_only && (
                    <section aria-label={t("detail.metadataManagementRegion")}>
                      <SectionLabel>{t("detail.metadataManagement")}</SectionLabel>
                      <div className={cn(inspectorCardClassName, "space-y-4")}>
                        <div
                          data-testid="detail-repository-management"
                          className="space-y-2"
                        >
                          <label
                            htmlFor="detail-repository-select"
                            className={inspectorFieldLabelClassName}
                          >
                            {t("central.repositorySelectLabel")}
                          </label>
                          <select
                            id="detail-repository-select"
                            className={inspectorSelectClassName}
                            value={selectedRepositoryId}
                            aria-label={t("central.repositorySelectLabel")}
                            onChange={(event) => setSelectedRepositoryId(event.target.value)}
                          >
                            <option value="">{t("central.chooseRepository")}</option>
                            {repositories.map((repository) => (
                              <option key={repository.id} value={repository.id}>
                                {repository.name}
                              </option>
                            ))}
                          </select>
                          <Button
                            type="button"
                            size="sm"
                            variant="secondary"
                            disabled={!selectedRepositoryId || isMetadataUpdating}
                            onClick={handleAssignRepository}
                            className={inspectorActionButtonClassName}
                          >
                            {t("central.assignRepository")}
                          </Button>
                        </div>
                        <div
                          data-testid="detail-tag-management"
                          className="space-y-2"
                        >
                          <label
                            htmlFor="detail-tag-select"
                            className={inspectorFieldLabelClassName}
                          >
                            {t("central.tagSelectLabel")}
                          </label>
                          <select
                            id="detail-tag-select"
                            className={inspectorSelectClassName}
                            value={selectedTagId}
                            aria-label={t("central.tagSelectLabel")}
                            onChange={(event) => setSelectedTagId(event.target.value)}
                          >
                            <option value="">{t("central.chooseTag")}</option>
                            {tags.map((tag) => (
                              <option key={tag.id} value={tag.id}>
                                {tag.name}
                              </option>
                            ))}
                          </select>
                          <Button
                            type="button"
                            size="sm"
                            variant="secondary"
                            disabled={!selectedTagId || isMetadataUpdating}
                            onClick={handleAssignTag}
                            className={inspectorActionButtonClassName}
                          >
                            {t("central.assignTag")}
                          </Button>
                        </div>
                      </div>
                    </section>
                  )}

                  {/* Install Status — compact icon grid */}
                  <section aria-label={t("detail.installStatusRegion")}>
                    <SectionLabel>{t("detail.installStatus")}</SectionLabel>
                    <div className="space-y-1.5">
                      {detail.is_read_only ? (
                        <p className="text-xs leading-relaxed text-muted-foreground">
                          {t("detail.readOnlyInstallBlocked", {
                            defaultValue: i18n.language.startsWith("zh")
                              ? "插件来源的只读副本不可安装或卸载。"
                              : "Install and uninstall are unavailable for read-only plugin copies.",
                          })}
                        </p>
                      ) : targetAgents.length === 0 ? (
                        <p className="text-xs text-muted-foreground">
                          {t("detail.noPlatforms")}
                        </p>
                      ) : (
                        <>
                          {lobsterAgents.length > 0 && (
                            <div className="flex items-center gap-1">
                              <span className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider w-12 shrink-0">
                                {t("sidebar.categoryLobster")}
                              </span>
                              <div className="flex items-center gap-0.5 flex-wrap">
                                {lobsterAgents.map((agent) => {
                                  const memberIds = getPlatformTargetMemberIds(agent);
                                  const isInstalled = memberIds.some((agentId) => installationMap.has(agentId)) ||
                                    memberIds.some((agentId) => sharedRootAgentIds.has(agentId));
                                  const isLocked = memberIds.some((agentId) => sharedRootAgentIds.has(agentId));
                                  const isLoading = memberIds.some((agentId) => installingAgentId === agentId);

                                  return (
                                    <PlatformToggleIcon
                                      key={agent.id}
                                      agent={agent}
                                      skillName={detail.name}
                                      isInstalled={isInstalled}
                                      isLoading={isLoading}
                                      isLocked={isLocked}
                                      onToggle={() => {
                                        const [agentId] = getPlatformTargetInstallAgentIds(agent);
                                        if (agentId) {
                                          void handleToggle(agentId);
                                        }
                                      }}
                                    />
                                  );
                                })}
                              </div>
                            </div>
                          )}
                          {codingAgents.length > 0 && (
                            <div className="flex items-center gap-1">
                              <span className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider w-12 shrink-0">
                                {t("sidebar.categoryCoding")}
                              </span>
                              <div className="flex items-center gap-0.5 flex-wrap">
                                {codingAgents.map((agent) => {
                                  const memberIds = getPlatformTargetMemberIds(agent);
                                  const isInstalled = memberIds.some((agentId) => installationMap.has(agentId)) ||
                                    memberIds.some((agentId) => sharedRootAgentIds.has(agentId));
                                  const isLocked = memberIds.some((agentId) => sharedRootAgentIds.has(agentId));
                                  const isLoading = memberIds.some((agentId) => installingAgentId === agentId);

                                  return (
                                    <PlatformToggleIcon
                                      key={agent.id}
                                      agent={agent}
                                      skillName={detail.name}
                                      isInstalled={isInstalled}
                                      isLoading={isLoading}
                                      isLocked={isLocked}
                                      onToggle={() => {
                                        const [agentId] = getPlatformTargetInstallAgentIds(agent);
                                        if (agentId) {
                                          void handleToggle(agentId);
                                        }
                                      }}
                                    />
                                  );
                                })}
                              </div>
                            </div>
                          )}
                        </>
                      )}
                    </div>
                  </section>

                  {/* Collections */}
                  <section aria-label={t("detail.collections")}>
                    <SectionLabel>{t("detail.collections")}</SectionLabel>
                    {detail.is_read_only ? (
                      <p className="text-xs leading-relaxed text-muted-foreground">
                        {t("detail.readOnlyCollectionsBlocked", {
                          defaultValue: i18n.language.startsWith("zh")
                            ? "插件来源的只读副本不可调整技能集。"
                            : "Collection management is unavailable for read-only plugin copies.",
                        })}
                      </p>
                    ) : (
                      <div className="flex flex-wrap gap-1.5 items-center">
                        {skillCollections.map((collection) => (
                          <span
                            key={collection.id}
                            className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-primary/10 text-primary ring-1 ring-primary/20"
                            title={collection.description ?? collection.name}
                          >
                            <Tag className="size-2.5" />
                            {collection.name}
                          </span>
                        ))}
                        <Button
                          ref={addToCollectionButtonRef}
                          variant="ghost"
                          size="sm"
                          className="h-8 gap-1 px-2 text-xs text-muted-foreground hover:text-foreground md:h-6"
                          aria-label={t("detail.addToCollection")}
                          onClick={() => setIsCollectionPickerOpen(true)}
                        >
                          <Plus className="size-3" />
                          {t("detail.addToCollection")}
                        </Button>
                      </div>
                    )}
                  </section>
                </>
              ) : null}
            </aside>
          </div>
        )}
      </div>

      {/* Collection Picker Dialog */}
      {skillId && !detail?.is_read_only && (
        <CollectionPickerDialog
          open={isCollectionPickerOpen}
          onOpenChange={handleCollectionPickerOpenChange}
          skillId={skillId}
          currentCollectionIds={skillCollections.map((collection) => collection.id)}
          onAdded={handleCollectionAdded}
        />
      )}
    </div>
  );
}
