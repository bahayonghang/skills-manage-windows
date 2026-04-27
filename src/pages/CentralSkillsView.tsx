import { useDeferredValue, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Search, RefreshCw, Blocks, FolderOpen, Settings, ArrowUpDown, Tag, X, Wand2, AlertTriangle, Check, Eye, ListChecks, Plus, Download, Trash2, FolderGit2, FileJson } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";

import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillStore } from "@/stores/skillStore";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { SkillDetailDrawer } from "@/components/skill/SkillDetailDrawer";
import { InstallDialog } from "@/components/central/InstallDialog";
import { BatchInstallCentralSkillsDialog } from "@/components/central/BatchInstallCentralSkillsDialog";
import { DeleteCentralSkillDialog } from "@/components/central/DeleteCentralSkillDialog";
import { BatchDeleteCentralSkillsDialog } from "@/components/central/BatchDeleteCentralSkillsDialog";
import { CentralStatePortabilityDialog } from "@/components/central/CentralStatePortabilityDialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { AgentWithStatus, AiTagJob, BatchDeleteCentralSkillPreviewResult, BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult, CentralBatchInstallResult, CentralSkillUpdateJob, CentralSkillUpdateState, SkillAiTagReview, ScannedSkill, SkillDetail, SkillRepositoryWithStats, SkillTag, SkillWithLinks, SkillportStateImportPreview, SkillportStateImportResolution, SkillportStateImportResult } from "@/types";
import { markAppPerformance } from "@/lib/performance";
import { cn } from "@/lib/utils";
import { GitHubRepoImportWizard } from "@/components/marketplace/GitHubRepoImportWizard";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useTargetStore } from "@/stores/targetStore";
import { VirtualizedList } from "@/components/ui/virtualized-list";
import { VirtualizedGrid } from "@/components/ui/virtualized-grid";
import { formatPathForDisplay } from "@/lib/path";
import { buildSearchText, normalizeSearchQuery } from "@/lib/search";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
} from "@/lib/platformVisibility";
import { getPlatformTargetGroups } from "@/lib/platformTargetGroups";
import { isTauriRuntime } from "@/lib/tauri";
import { useResizableWidth } from "@/hooks/useResizableWidth";

const BROWSER_FIXTURE_SKILLS: SkillWithLinks[] = [
  {
    id: "fixture-central-skill",
    name: "fixture-central-skill",
    description: "Browser validation fixture for Central and drawer entry flows.",
    file_path: "~/.skillsmanage/skills/fixture-central-skill/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/fixture-central-skill",
    is_central: true,
    source: "browser-fixture",
    scanned_at: "2026-04-17T00:00:00.000Z",
    created_at: "2026-04-17T00:00:00.000Z",
    updated_at: "2026-04-17T00:00:00.000Z",
    linked_agents: ["claude-code"],
    shared_root_agents: [],
    tags: [],
    is_source_unknown: true,
  },
];

const EMPTY_SKILLS: SkillWithLinks[] = [];
const EMPTY_AGENTS: AgentWithStatus[] = [];
const EMPTY_REPOSITORIES: SkillRepositoryWithStats[] = [];
const EMPTY_TAGS: SkillTag[] = [];
const EMPTY_AI_TAG_REVIEWS: SkillAiTagReview[] = [];
const EMPTY_SKILLS_BY_AGENT: Record<string, ScannedSkill[]> = {};
const EMPTY_UPDATE_STATUSES: Record<string, CentralSkillUpdateState> = {};
const CENTRAL_FILTER_DEFAULT_WIDTH = 286;
const CENTRAL_FILTER_MIN_WIDTH = 220;
const CENTRAL_FILTER_MAX_WIDTH = 460;
const CENTRAL_CATEGORIZE_DEFAULT_WIDTH = 392;
const CENTRAL_CATEGORIZE_MIN_WIDTH = 336;
const CENTRAL_CATEGORIZE_MAX_WIDTH = 640;
const IDLE_AI_TAG_JOB: AiTagJob = {
  jobId: null,
  status: "idle",
  total: 0,
  completed: 0,
  succeeded: 0,
  failed: 0,
  lowConfidenceCount: 0,
  items: {},
};
const IDLE_UPDATE_JOB: CentralSkillUpdateJob = {
  phase: null,
  status: "idle",
  total: 0,
  completed: 0,
  succeeded: 0,
  failed: 0,
  skipped: 0,
  items: {},
};
const EMPTY_GITHUB_IMPORT_STATE = {
  isPreviewLoading: false,
  isImporting: false,
  preview: null,
  importResult: null,
  previewedRepoUrl: null,
  error: null,
};

async function noopAsync(): Promise<void> {}

async function noopInstallSkill() {
  return {
    succeeded: [],
    failed: [],
  };
}

async function noopBatchInstallSkills(): Promise<CentralBatchInstallResult> {
  return {
    succeeded: [],
    failed: [],
  };
}

async function noopPreviewGitHubRepoImport() {
  return null;
}

async function noopImportGitHubRepoSkills() {
  throw new Error("GitHub import is unavailable");
}

async function noopExportSkillportState(): Promise<string> {
  return JSON.stringify(
    {
      kind: "skillport/state-export",
      version: 1,
      exportedAt: new Date().toISOString(),
      exportedFrom: { app: "SkillPort" },
      githubSources: [],
      centralSkills: [],
      unrestorableSkills: [],
    },
    null,
    2
  );
}

async function noopPreviewSkillportStateImport(_json: string): Promise<SkillportStateImportPreview> {
  throw new Error("State import is unavailable");
}

async function noopImportSkillportState(
  _json: string,
  _resolutions: SkillportStateImportResolution[]
): Promise<SkillportStateImportResult> {
  throw new Error("State import is unavailable");
}

async function noopGetSkillsByAgent(_agentId: string): Promise<void> {}

async function noopLoadDeletePreview(): Promise<SkillDetail> {
  throw new Error("Central skill deletion is unavailable");
}

async function noopLoadBatchDeletePreview(): Promise<BatchDeleteCentralSkillPreviewResult> {
  throw new Error("Central skill deletion is unavailable");
}

async function noopDeleteCentralSkill(): Promise<void> {}

async function noopDeleteCentralSkills(): Promise<BatchDeleteCentralSkillResult> {
  return {
    succeeded: [],
    failed: [],
  };
}

async function noopUnlisten() {
  return () => {};
}

async function noopCheckUpdates(): Promise<CentralSkillUpdateState[]> {
  return [];
}

async function noopUpdateSkills() {
  return {
    succeeded: [],
    failed: [],
    skipped: [],
    states: [],
  };
}

function noopResetGitHubImport() {}

// ─── Empty State ──────────────────────────────────────────────────────────────

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-4 py-20">
      <div className="p-4 rounded-full bg-muted/60">
        <Blocks className="size-12 text-muted-foreground opacity-60" />
      </div>
      <p className="text-sm text-muted-foreground font-medium">{message}</p>
    </div>
  );
}

// ─── First Visit Empty State ──────────────────────────────────────────────────

function FirstVisitEmptyState() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-center justify-center h-full gap-6 py-16 text-center px-8">
      <div className="p-5 rounded-full bg-primary/10 ring-1 ring-primary/20">
        <Blocks className="size-14 text-primary opacity-70" />
      </div>
      <div className="space-y-2">
        <h2 className="text-xl font-semibold text-foreground">{t("empty.welcomeTitle")}</h2>
        <p className="text-sm text-muted-foreground max-w-sm leading-relaxed">
          {t("empty.welcomeDesc")}
        </p>
      </div>
      <div className="flex flex-col gap-3 items-center">
        <div className="flex items-center gap-2 text-xs text-muted-foreground bg-muted/50 rounded-xl px-4 py-3 max-w-xs text-left border border-border">
          <FolderOpen className="size-4 shrink-0 text-primary/60" />
          <span>
            {t("empty.createHint")} <code className="font-mono">~/.skillsmanage/skills/my-skill/SKILL.md</code>
          </span>
        </div>
        <Button
          variant="default"
          size="sm"
          onClick={() => navigate("/settings")}
          className="gap-2"
        >
          <Settings className="size-4" />
          {t("empty.goToSettings")}
        </Button>
      </div>
    </div>
  );
}

function parseSortableTimestamp(value?: string | null): number {
  if (!value) return 0;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? timestamp : 0;
}

function getSkillSortTimestamp(
  skill: SkillWithLinks,
  field: "createdAt" | "updatedAt"
): number {
  return parseSortableTimestamp(
    field === "createdAt"
      ? skill.created_at ?? skill.scanned_at
      : skill.updated_at ?? skill.scanned_at
  );
}

// ─── CentralSkillsView ────────────────────────────────────────────────────────

function isGithubRepository(repository: SkillRepositoryWithStats): boolean {
  return (
    repository.source_type === "github" ||
    Boolean(repository.owner && repository.repo)
  );
}

function getRepositorySkillCount(repository: SkillRepositoryWithStats): number {
  return repository.is_unknown
    ? repository.unknown_skill_count
    : repository.skill_count;
}

function FilterSection({
  icon,
  title,
  children,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="space-y-2.5">
      <div className="flex items-center gap-2 px-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
        {icon}
        <span>{title}</span>
      </div>
      <div className="space-y-1.5">{children}</div>
    </section>
  );
}

function RepositoryFilterButton({
  active,
  count,
  label,
  sourceKind,
  onClick,
  testId,
}: {
  active: boolean;
  count: number;
  label: string;
  sourceKind: "all" | "github" | "local";
  onClick: () => void;
  testId: string;
}) {
  const Icon = sourceKind === "github" ? FolderGit2 : FolderOpen;

  return (
    <button
      type="button"
      data-testid={testId}
      data-source-kind={sourceKind}
      onClick={onClick}
      className={cn(
        "group flex min-h-10 w-full items-center gap-2.5 rounded-lg border px-2.5 py-2 text-left text-xs transition-colors",
        active
          ? "border-primary/25 bg-background text-foreground shadow-sm ring-1 ring-primary/10"
          : "border-transparent text-muted-foreground hover:border-border/70 hover:bg-background/70 hover:text-foreground",
        sourceKind === "local" && !active
          ? "border-dashed border-border/80 bg-muted/20"
          : null
      )}
    >
      <span
        className={cn(
          "grid size-6 shrink-0 place-items-center rounded-md border",
          sourceKind === "github"
            ? "border-sky-500/20 bg-sky-500/10 text-sky-600 dark:text-sky-300"
            : sourceKind === "local"
              ? "border-dashed border-border bg-muted text-muted-foreground"
              : "border-border bg-background text-muted-foreground"
        )}
      >
        <Icon className="size-3.5" />
      </span>
      <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
      <span className="shrink-0 rounded-md border border-border/80 bg-background px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-muted-foreground">
        {count}
      </span>
    </button>
  );
}

function TagFilterButton({
  active,
  count,
  label,
  tone,
  onClick,
  testId,
}: {
  active: boolean;
  count: number;
  label: string;
  tone: "all" | "tag" | "uncategorized" | "updates" | "ai";
  onClick: () => void;
  testId: string;
}) {
  const dotClassName =
    tone === "updates"
      ? "bg-amber-500"
      : tone === "ai"
        ? "bg-violet-500"
        : tone === "uncategorized"
          ? "bg-slate-400"
          : tone === "tag"
            ? "bg-emerald-500"
            : "bg-muted-foreground/60";

  return (
    <button
      type="button"
      data-testid={testId}
      data-filter-kind="tag"
      onClick={onClick}
      className={cn(
        "flex min-h-9 w-full items-center gap-2 rounded-full border px-2.5 py-1.5 text-left text-xs transition-colors",
        active
          ? "border-primary/30 bg-primary/10 text-primary ring-1 ring-primary/10"
          : "border-border/70 bg-background/35 text-muted-foreground hover:bg-background hover:text-foreground"
      )}
    >
      <span className={cn("size-2 shrink-0 rounded-full", dotClassName)} />
      <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
      <span className="shrink-0 rounded-full bg-muted px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-muted-foreground">
        {count}
      </span>
    </button>
  );
}

export function CentralSkillsView() {
  const { t } = useTranslation();
  const rawSkills = useCentralSkillsStore((state) => state.skills);
  const rawAgents = useCentralSkillsStore((state) => state.agents);
  const rawRepositories = useCentralSkillsStore((state) => state.repositories);
  const rawTags = useCentralSkillsStore((state) => state.tags);
  const rawAiTagReviews = useCentralSkillsStore((state) => state.aiTagReviews);
  const rawAiTagJob = useCentralSkillsStore((state) => state.aiTagJob);
  const rawUpdateStatuses = useCentralSkillsStore((state) => state.updateStatuses);
  const rawUpdateJob = useCentralSkillsStore((state) => state.updateJob);
  const rawAiTaggingAvailable = useCentralSkillsStore((state) => state.aiTaggingAvailable);
  const rawIsLoading = useCentralSkillsStore((state) => state.isLoading);
  const rawLoadCentralSkills = useCentralSkillsStore(
    (state) => state.loadCentralSkills
  );
  const shouldUseBrowserFixtures =
    !isTauriRuntime() &&
    rawSkills === undefined &&
    rawAgents === undefined &&
    rawLoadCentralSkills === undefined;
  const skills = shouldUseBrowserFixtures ? BROWSER_FIXTURE_SKILLS : rawSkills ?? EMPTY_SKILLS;
  const agents = rawAgents ?? EMPTY_AGENTS;
  const repositories = rawRepositories ?? EMPTY_REPOSITORIES;
  const tags = rawTags ?? EMPTY_TAGS;
  const aiTagReviews = rawAiTagReviews ?? EMPTY_AI_TAG_REVIEWS;
  const aiTagJob = rawAiTagJob ?? IDLE_AI_TAG_JOB;
  const updateStatuses = rawUpdateStatuses ?? EMPTY_UPDATE_STATUSES;
  const updateJob = rawUpdateJob ?? IDLE_UPDATE_JOB;
  const aiTaggingAvailable = rawAiTaggingAvailable ?? false;
  const centralSkillsDir = formatPathForDisplay(
    agents.find((agent) => agent.id === "central")?.global_skills_dir ?? t("central.path")
  );
  const isLoading = shouldUseBrowserFixtures ? false : rawIsLoading ?? false;
  const loadCentralSkills = rawLoadCentralSkills ?? noopAsync;
  const installSkill = useCentralSkillsStore((state) => state.installSkill) ?? noopInstallSkill;
  const batchInstallSkills =
    useCentralSkillsStore((state) => state.batchInstallSkills) ?? noopBatchInstallSkills;
  const loadDeletePreview = useCentralSkillsStore((state) => state.loadDeletePreview) ?? noopLoadDeletePreview;
  const loadBatchDeletePreview = useCentralSkillsStore((state) => state.loadBatchDeletePreview) ?? noopLoadBatchDeletePreview;
  const deleteCentralSkill = useCentralSkillsStore((state) => state.deleteCentralSkill) ?? noopDeleteCentralSkill;
  const deleteCentralSkills = useCentralSkillsStore((state) => state.deleteCentralSkills) ?? noopDeleteCentralSkills;
  const togglePlatformLink = useCentralSkillsStore((state) => state.togglePlatformLink) ?? noopAsync;
  const createTag = useCentralSkillsStore((state) => state.createTag);
  const assignSkillTags = useCentralSkillsStore((state) => state.assignSkillTags);
  const bulkSuggestSkillTags = useCentralSkillsStore((state) => state.bulkSuggestSkillTags);
  const cancelAiTagJob = useCentralSkillsStore((state) => state.cancelAiTagJob);
  const acceptAiTagReview = useCentralSkillsStore((state) => state.acceptAiTagReview);
  const skipAiTagReview = useCentralSkillsStore((state) => state.skipAiTagReview);
  const checkSkillUpdates = useCentralSkillsStore((state) => state.checkSkillUpdates) ?? noopCheckUpdates;
  const updateSkills = useCentralSkillsStore((state) => state.updateSkills) ?? noopUpdateSkills;
  const subscribeAiTagProgress =
    useCentralSkillsStore((state) => state.subscribeAiTagProgress) ?? noopUnlisten;
  const subscribeUpdateProgress =
    useCentralSkillsStore((state) => state.subscribeUpdateProgress) ?? noopUnlisten;
  const isMetadataUpdating = useCentralSkillsStore((state) => state.isMetadataUpdating) ?? false;
  const isSuggestingTags = useCentralSkillsStore((state) => state.isSuggestingTags) ?? false;
  const isCheckingUpdates = useCentralSkillsStore((state) => state.isCheckingUpdates) ?? false;
  const updatingSkillIds = useCentralSkillsStore((state) => state.updatingSkillIds) ?? [];
  const isInstalling = useCentralSkillsStore((state) => state.isInstalling) ?? false;
  const isDeleting = useCentralSkillsStore((state) => state.isDeleting) ?? false;
  const togglingAgentId = useCentralSkillsStore((state) => state.togglingAgentId);
  const activeTarget = useTargetStore((state) => state.activeTarget);
  const isRemoteTarget = activeTarget.kind === "ssh";

  // Keep the platform sidebar counts in sync after install.
  const categoryVisibility = usePlatformStore((state) => state.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const refreshCounts = usePlatformStore((state) => state.refreshCounts) ?? noopAsync;
  const platformAgents = usePlatformStore((state) => state.agents) ?? EMPTY_AGENTS;
  const skillsByAgent = useSkillStore((state) => state.skillsByAgent) ?? EMPTY_SKILLS_BY_AGENT;
  const getSkillsByAgent = useSkillStore((state) => state.getSkillsByAgent) ?? noopGetSkillsByAgent;
  const githubImport = useMarketplaceStore((state) => state.githubImport) ?? EMPTY_GITHUB_IMPORT_STATE;
  const previewGitHubRepoImport =
    useMarketplaceStore((state) => state.previewGitHubRepoImport) ?? noopPreviewGitHubRepoImport;
  const importGitHubRepoSkills =
    useMarketplaceStore((state) => state.importGitHubRepoSkills) ?? noopImportGitHubRepoSkills;
  const resetGitHubImport =
    useMarketplaceStore((state) => state.resetGitHubImport) ?? noopResetGitHubImport;
  const exportSkillportState =
    useCentralSkillsStore((state) => state.exportSkillportState) ?? noopExportSkillportState;
  const previewSkillportStateImport =
    useCentralSkillsStore((state) => state.previewSkillportStateImport) ?? noopPreviewSkillportStateImport;
  const importSkillportState =
    useCentralSkillsStore((state) => state.importSkillportState) ?? noopImportSkillportState;

  type SortField = "name" | "createdAt" | "updatedAt";
  type SortDirection = "asc" | "desc";
  type CategorizeTab = "manual" | "ai" | "review";
  const [sortField, setSortField] = useState<SortField>("name");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");
  const [searchQuery, setSearchQuery] = useState("");
  const [repositoryFilter, setRepositoryFilter] = useState("all");
  const [tagFilter, setTagFilter] = useState("all");
  const [selectedSkillIds, setSelectedSkillIds] = useState<string[]>([]);
  const [categorizeTab, setCategorizeTab] = useState<CategorizeTab>("manual");
  const [manualTagQuery, setManualTagQuery] = useState("");
  const [manualSelectedTagIds, setManualSelectedTagIds] = useState<string[]>([]);
  const [installTargetSkill, setInstallTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [deleteTargetSkill, setDeleteTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [deletePreview, setDeletePreview] = useState<SkillDetail | null>(null);
  const [batchDeletePreview, setBatchDeletePreview] =
    useState<BatchDeleteCentralSkillPreviewResult | null>(null);
  const {
    width: filterSidebarWidth,
    startResize: startFilterSidebarResize,
    handleResizeKeyDown: handleFilterSidebarResizeKeyDown,
  } = useResizableWidth({
    defaultWidth: CENTRAL_FILTER_DEFAULT_WIDTH,
    minWidth: CENTRAL_FILTER_MIN_WIDTH,
    maxWidth: CENTRAL_FILTER_MAX_WIDTH,
  });
  const {
    width: categorizeSidebarWidth,
    startResize: startCategorizeSidebarResize,
    handleResizeKeyDown: handleCategorizeSidebarResizeKeyDown,
  } = useResizableWidth({
    defaultWidth: CENTRAL_CATEGORIZE_DEFAULT_WIDTH,
    minWidth: CENTRAL_CATEGORIZE_MIN_WIDTH,
    maxWidth: CENTRAL_CATEGORIZE_MAX_WIDTH,
    resizeFrom: "left",
  });
  const [isDeletePreviewLoading, setIsDeletePreviewLoading] = useState(false);
  const [isBatchDeletePreviewLoading, setIsBatchDeletePreviewLoading] = useState(false);
  const [deletePreviewError, setDeletePreviewError] = useState<string | null>(null);
  const [batchDeletePreviewError, setBatchDeletePreviewError] = useState<string | null>(null);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isBatchInstallDialogOpen, setIsBatchInstallDialogOpen] = useState(false);
  const [isBatchDeleteDialogOpen, setIsBatchDeleteDialogOpen] = useState(false);
  const [drawerSkillId, setDrawerSkillId] = useState<string | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isGitHubImportOpen, setIsGitHubImportOpen] = useState(false);
  const [isPortabilityOpen, setIsPortabilityOpen] = useState(false);
  const [githubRepoUrl, setGitHubRepoUrl] = useState("");
  const contentRef = useRef<HTMLDivElement | null>(null);
  const detailButtonRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const hasMarkedCentralListReady = useRef(false);
  const deferredSearchQuery = useDeferredValue(searchQuery);
  const effectiveSearchQuery =
    skills.length > 80 ? deferredSearchQuery : searchQuery;
  const normalizedSearchQuery = useMemo(
    () => normalizeSearchQuery(effectiveSearchQuery),
    [effectiveSearchQuery]
  );
  const searchableSkills = useMemo(
    () =>
      skills.map((skill) => ({
        skill,
        searchText: buildSearchText([
          skill.name,
          skill.description,
          skill.repository?.name,
          skill.source_path,
          ...(skill.tags ?? []).map((tag) => tag.name),
        ]),
      })),
    [skills]
  );
  const aiReviewSkillIds = useMemo(
    () => new Set(aiTagReviews.map((review) => review.skill_id)),
    [aiTagReviews]
  );
  const updateAvailableSkillIds = useMemo(
    () =>
      skills
        .filter((skill) => updateStatuses[skill.id]?.status === "update_available")
        .map((skill) => skill.id),
    [skills, updateStatuses]
  );
  const selectedUpdatableSkillIds = useMemo(
    () => selectedSkillIds.filter((skillId) => updateStatuses[skillId]?.status === "update_available"),
    [selectedSkillIds, updateStatuses]
  );
  const updateTargetSkillIds =
    selectedSkillIds.length > 0 ? selectedUpdatableSkillIds : updateAvailableSkillIds;
  const uncategorizedCount = useMemo(
    () =>
      skills.filter((skill) => {
        const skillTags = skill.tags ?? [];
        return skillTags.length === 0 || skillTags.some((tag) => tag.id === "uncategorized");
      }).length,
    [skills]
  );
  const tagCounts = useMemo(
    () =>
      tags.map((tag) => ({
        tag,
        count: skills.filter((skill) => (skill.tags ?? []).some((item) => item.id === tag.id)).length,
      })),
    [skills, tags]
  );
  const filteredManualTags = useMemo(() => {
    const query = normalizeSearchQuery(manualTagQuery);
    return query
      ? tags.filter((tag) => normalizeSearchQuery(tag.name).includes(query))
      : tags;
  }, [manualTagQuery, tags]);
  const canCreateManualTag = useMemo(() => {
    const name = manualTagQuery.trim();
    if (!name) return false;
    return !tags.some((tag) => normalizeSearchQuery(tag.name) === normalizeSearchQuery(name));
  }, [manualTagQuery, tags]);
  const skillIdsKey = useMemo(() => skills.map((skill) => skill.id).join("\0"), [skills]);
  const isSearchActive = normalizedSearchQuery.length > 0;

  // Load central skills on mount.
  useEffect(() => {
    loadCentralSkills();
  }, [loadCentralSkills]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    subscribeAiTagProgress().then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unlisten = unsubscribe;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [subscribeAiTagProgress]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    subscribeUpdateProgress().then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unlisten = unsubscribe;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [subscribeUpdateProgress]);

  // Filter skills by search query.
  const filteredSkills = useMemo(() => {
    return searchableSkills
      .filter(({ searchText }) => !normalizedSearchQuery || searchText.includes(normalizedSearchQuery))
      .map(({ skill }) => skill)
      .filter((skill) => {
        const repository = skill.repository;
        const skillTags = skill.tags ?? [];
        const matchesRepository =
          repositoryFilter === "all" ||
          (repositoryFilter === "unassigned"
            ? skill.is_source_unknown || repository?.is_unknown
            : repository?.id === repositoryFilter);
        const matchesTag =
          tagFilter === "all" ||
          (tagFilter === "updates"
            ? updateStatuses[skill.id]?.status === "update_available"
            : false) ||
          (tagFilter === "ai-review"
            ? aiReviewSkillIds.has(skill.id)
            : false) ||
          (tagFilter === "uncategorized"
            ? skillTags.length === 0 || skillTags.some((tag) => tag.id === "uncategorized")
            : skillTags.some((tag) => tag.id === tagFilter));
        return matchesRepository && matchesTag;
      });
  }, [aiReviewSkillIds, normalizedSearchQuery, repositoryFilter, searchableSkills, tagFilter, updateStatuses]);

  // Sort filtered skills.
  const sortedSkills = useMemo(() => {
    const list = [...filteredSkills];
    const direction = sortDirection === "asc" ? 1 : -1;
    return list.sort((a, b) => {
      const nameComparison = a.name.localeCompare(b.name, undefined, {
        numeric: true,
        sensitivity: "base",
      });

      if (sortField === "name") {
        return nameComparison * direction;
      }

      const leftTime = getSkillSortTimestamp(a, sortField);
      const rightTime = getSkillSortTimestamp(b, sortField);
      const timeComparison = leftTime - rightTime;

      return timeComparison === 0 ? nameComparison : timeComparison * direction;
    });
  }, [filteredSkills, sortDirection, sortField]);

  useEffect(() => {
    if (!isSearchActive || !contentRef.current) return;
    contentRef.current.scrollTop = 0;
  }, [isSearchActive, normalizedSearchQuery]);

  useEffect(() => {
    const visibleIds = new Set(skillIdsKey ? skillIdsKey.split("\0") : []);
    setSelectedSkillIds((current) => {
      const next = current.filter((skillId) => visibleIds.has(skillId));
      return next.length === current.length ? current : next;
    });
  }, [skillIdsKey]);

  useEffect(() => {
    if (!isLoading && skills.length > 0 && !hasMarkedCentralListReady.current) {
      hasMarkedCentralListReady.current = true;
      markAppPerformance("central_list_ready");
    }
  }, [isLoading, skills.length]);

  function handleInstallClick(skill: SkillWithLinks) {
    setInstallTargetSkill(skill);
    setIsDialogOpen(true);
  }

  function handleDeleteDialogOpenChange(open: boolean) {
    setIsDeleteDialogOpen(open);
    if (!open) {
      setDeleteTargetSkill(null);
      setDeletePreview(null);
      setDeletePreviewError(null);
      setIsDeletePreviewLoading(false);
    }
  }

  function handleBatchDeleteDialogOpenChange(open: boolean) {
    setIsBatchDeleteDialogOpen(open);
    if (!open) {
      setBatchDeletePreview(null);
      setBatchDeletePreviewError(null);
      setIsBatchDeletePreviewLoading(false);
    }
  }

  async function handleDeleteClick(skill: SkillWithLinks) {
    setDeleteTargetSkill(skill);
    setDeletePreview(null);
    setDeletePreviewError(null);
    setIsDeleteDialogOpen(true);
    setIsDeletePreviewLoading(true);
    try {
      const preview = await loadDeletePreview(skill.id);
      setDeletePreview(preview);
    } catch (err) {
      const message = String(err);
      setDeletePreviewError(message);
      toast.error(t("central.deletePreviewError", { error: message }));
    } finally {
      setIsDeletePreviewLoading(false);
    }
  }

  async function handleBatchDeleteClick() {
    if (selectedSkillIds.length === 0) return;
    setBatchDeletePreview(null);
    setBatchDeletePreviewError(null);
    setIsBatchDeleteDialogOpen(true);
    setIsBatchDeletePreviewLoading(true);
    try {
      const preview = await loadBatchDeletePreview(selectedSkillIds);
      setBatchDeletePreview(preview);
    } catch (err) {
      const message = String(err);
      setBatchDeletePreviewError(message);
      toast.error(t("central.batchDeletePreviewError", { error: message }));
    } finally {
      setIsBatchDeletePreviewLoading(false);
    }
  }

  const sortFieldOptions: Array<{ value: SortField; label: string }> = [
    { value: "name", label: t("central.sortByName") },
    { value: "createdAt", label: t("central.sortByCreatedAt") },
    { value: "updatedAt", label: t("central.sortByUpdatedAt") },
  ];

  const sortDirectionOptions: Array<{ value: SortDirection; label: string }> = [
    { value: "asc", label: t("central.sortAscending") },
    { value: "desc", label: t("central.sortDescending") },
  ];

  function setDetailButtonRef(skillId: string, node: HTMLButtonElement | null) {
    detailButtonRefs.current[skillId] = node;
  }

  function handleOpenDrawer(skillId: string) {
    setDrawerSkillId(skillId);
    setIsDrawerOpen(true);
  }

  async function handleTogglePlatform(skillId: string, agentId: string) {
    try {
      await togglePlatformLink(skillId, agentId);
      await refreshCounts();
    } catch (err) {
      toast.error(t("central.installError", { error: String(err) }));
    }
  }

  async function handleInstall(skillId: string, agentIds: string[], method: string) {
    try {
      const result = await installSkill(skillId, agentIds, method);
      // Refresh sidebar counts after install.
      await refreshCounts();
      if (result.failed.length > 0) {
        const failedNames = result.failed
          .map((failure) => `${failure.agent_id}: ${failure.error}`)
          .join("; ");
        toast.error(t("central.installPartialFail", { platforms: failedNames }));
      }
      return result;
    } catch (err) {
      toast.error(t("central.installError", { error: String(err) }));
      throw err;
    }
  }

  async function handleBatchInstallCentralSkills(
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null
  ) {
    const requestedSkillIds = [...selectedSkillIds];
    const requestedSkillCount = requestedSkillIds.length;
    const platformCount = agentIds.length;

    try {
      const result = await batchInstallSkills(requestedSkillIds, agentIds, method, projectPath);
      await refreshCounts();
      if (result.failed.length > 0) {
        toast.error(
          t("central.batchInstallPartialToast", {
            skillCount: requestedSkillCount,
            platformCount,
            failedCount: result.failed.length,
          })
        );
      } else {
        toast.success(
          t("central.batchInstallSuccess", {
            skillCount: requestedSkillCount,
            platformCount,
          })
        );
        setSelectedSkillIds([]);
      }
      return result;
    } catch (err) {
      toast.error(t("central.installError", { error: String(err) }));
      throw err;
    }
  }

  async function handleDeleteCentralSkill(skillId: string, removeAgentIds: string[]) {
    try {
      await deleteCentralSkill(skillId, removeAgentIds);
      await refreshCounts();
      toast.success(t("central.deleteSkillSuccess", { name: deleteTargetSkill?.name ?? skillId }));
      handleDeleteDialogOpenChange(false);
    } catch (err) {
      const message = String(err);
      setDeletePreviewError(message);
      toast.error(t("central.deleteSkillError", { error: message }));
      throw err;
    }
  }

  async function handleBatchDeleteCentralSkills(requests: BatchDeleteCentralSkillRequest[]) {
    try {
      const result = await deleteCentralSkills(requests);
      await refreshCounts();
      const succeededIds = new Set(result.succeeded.map((item) => item.skill_id));
      setSelectedSkillIds((current) => current.filter((skillId) => !succeededIds.has(skillId)));

      if (result.failed.length > 0) {
        toast.error(
          t("central.batchDeletePartialError", {
            succeeded: result.succeeded.length,
            failed: result.failed.length,
          })
        );
      } else {
        toast.success(t("central.batchDeleteSuccess", { count: result.succeeded.length }));
      }
      handleBatchDeleteDialogOpenChange(false);
      return result;
    } catch (err) {
      const message = String(err);
      setBatchDeletePreviewError(message);
      toast.error(t("central.deleteSkillError", { error: message }));
      throw err;
    }
  }

  function handleToggleSelection(skillId: string) {
    setSelectedSkillIds((current) =>
      current.includes(skillId)
        ? current.filter((id) => id !== skillId)
        : [...current, skillId]
    );
  }

  function handleSelectCurrentFilter() {
    setSelectedSkillIds(sortedSkills.map((skill) => skill.id));
  }

  function handleSelectUncategorized() {
    setSelectedSkillIds(
      sortedSkills
        .filter((skill) => {
          const skillTags = skill.tags ?? [];
          return skillTags.length === 0 || skillTags.some((tag) => tag.id === "uncategorized");
        })
        .map((skill) => skill.id)
    );
  }

  function handleToggleManualTag(tagId: string) {
    setManualSelectedTagIds((current) =>
      current.includes(tagId)
        ? current.filter((id) => id !== tagId)
        : [...current, tagId]
    );
  }

  async function handleCreateManualTag() {
    const name = manualTagQuery.trim();
    if (!name || !createTag) return;
    try {
      const tag = await createTag(name);
      setManualSelectedTagIds((current) =>
        current.includes(tag.id) ? current : [...current, tag.id]
      );
      setManualTagQuery("");
      toast.success(t("central.tagCreated"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleApplyManualTags() {
    if (!assignSkillTags || selectedSkillIds.length === 0 || manualSelectedTagIds.length === 0) return;
    try {
      await assignSkillTags(selectedSkillIds, manualSelectedTagIds);
      toast.success(t("central.tagsAssigned", { count: selectedSkillIds.length }));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleAcceptReview(review: SkillAiTagReview) {
    if (!acceptAiTagReview) return;
    try {
      await acceptAiTagReview(review.skill_id, [review.tag.id]);
      toast.success(t("central.reviewAccepted"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleApplyManualTagsToReview(review: SkillAiTagReview) {
    if (!assignSkillTags || !skipAiTagReview || manualSelectedTagIds.length === 0) return;
    try {
      await assignSkillTags([review.skill_id], manualSelectedTagIds);
      await skipAiTagReview(review.skill_id);
      toast.success(t("central.reviewChanged"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleSkipReview(review: SkillAiTagReview) {
    if (!skipAiTagReview) return;
    try {
      await skipAiTagReview(review.skill_id);
      toast.success(t("central.reviewSkipped"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleBulkSuggestTags() {
    if (!bulkSuggestSkillTags || selectedSkillIds.length === 0) return;
    try {
      const result = await bulkSuggestSkillTags(selectedSkillIds);
      const succeeded = result.filter((item) => item.succeeded !== false && !item.error).length;
      const failed = result.length - succeeded;
      const review = result.reduce((count, item) => count + (item.low_confidence_count ?? 0), 0);
      toast.success(t("central.aiTagsFinished", { succeeded, failed, review }));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleCancelAiTagJob() {
    if (!cancelAiTagJob) return;
    try {
      await cancelAiTagJob();
      toast.info(t("central.aiTagCancelRequested"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleRefresh() {
    try {
      // Re-scan the filesystem first so new/removed skills are picked up,
      // then reload central skills from the (now-updated) database.
      await refreshCounts();
      await loadCentralSkills();
    } catch (err) {
      toast.error(t("central.refreshError", { error: String(err) }));
    }
  }

  async function handleCheckUpdates() {
    try {
      const states = await checkSkillUpdates();
      const available = states.filter((state) => state.status === "update_available").length;
      const unsupported = states.filter((state) => state.status === "unsupported").length;
      const failed = states.filter((state) => state.status === "error").length;
      toast.success(t("central.updateCheckFinished", { available, unsupported, failed }));
    } catch (err) {
      toast.error(t("central.updateCheckError", { error: String(err) }));
    }
  }

  async function handleUpdateSkills(skillIds: string[]) {
    if (skillIds.length === 0) return;
    try {
      const result = await updateSkills(skillIds);
      await refreshCounts();
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

  async function handleGitHubPreview() {
    try {
      return await previewGitHubRepoImport(githubRepoUrl);
    } catch {
      return null;
    }
  }

  async function handleGitHubImport(
    selections: Parameters<typeof importGitHubRepoSkills>[1]
  ) {
    try {
      const result = await importGitHubRepoSkills(githubRepoUrl, selections);
      await Promise.all([refreshCounts(), loadCentralSkills()]);
      toast.success(t("marketplace.githubImportCentralSuccess"));
      return result;
    } catch (err) {
      toast.error(t("marketplace.githubImportError", { error: String(err) }));
      throw err;
    }
  }

  async function handleInstallImportedSkill(
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy"
  ) {
    const result = await handleInstall(skillId, agentIds, method);
    await Promise.all(agentIds.map((agentId) => getSkillsByAgent(agentId)));
    return result;
  }

  const installableImportedSkills = useMemo(() => {
    if (!githubImport.importResult) return [];
    const importedIds = new Set(
      githubImport.importResult.importedSkills.map((skill) => skill.importedSkillId)
    );
    return skills.filter((skill) => importedIds.has(skill.id));
  }, [githubImport.importResult, skills]);

  const availableInstallAgents = useMemo(
    () => getPlatformTargetGroups(platformAgents, categoryVisibility),
    [platformAgents, categoryVisibility]
  );

  async function handleAfterImportSuccess() {
    const agentIds = Object.keys(skillsByAgent);
    if (agentIds.length === 0) return;
    await Promise.all(agentIds.map((agentId) => getSkillsByAgent(agentId)));
  }

  function renderSearchResult(skill: SkillWithLinks) {
    return (
      <UnifiedSkillCard
        key={skill.id}
        name={skill.name}
        description={skill.description}
        checkbox={{
          checked: selectedSkillIds.includes(skill.id),
          onChange: () => handleToggleSelection(skill.id),
        }}
        tags={(skill.tags ?? []).map((tag) => ({ key: tag.id, label: tag.name }))}
        publisher={skill.repository?.name}
        updateStatus={
          updateStatuses[skill.id]
            ? {
                ...updateStatuses[skill.id],
                isUpdating: updatingSkillIds.includes(skill.id),
              }
            : undefined
        }
        onDetail={() => handleOpenDrawer(skill.id)}
        onInstallTo={() => handleInstallClick(skill)}
        onUpdateCentral={() => void handleUpdateSkills([skill.id])}
        onDeleteFromCentral={() => void handleDeleteClick(skill)}
        detailButtonRef={(node) => setDetailButtonRef(skill.id, node)}
        className="h-[132px]"
      />
    );
  }

  function renderGridCard(skill: SkillWithLinks) {
    return (
      <UnifiedSkillCard
        key={skill.id}
        name={skill.name}
        description={skill.description}
        checkbox={{
          checked: selectedSkillIds.includes(skill.id),
          onChange: () => handleToggleSelection(skill.id),
        }}
        tags={(skill.tags ?? []).map((tag) => ({ key: tag.id, label: tag.name }))}
        publisher={skill.repository?.name}
        updateStatus={
          updateStatuses[skill.id]
            ? {
                ...updateStatuses[skill.id],
                isUpdating: updatingSkillIds.includes(skill.id),
              }
            : undefined
        }
        onDetail={() => handleOpenDrawer(skill.id)}
        onInstallTo={() => handleInstallClick(skill)}
        onUpdateCentral={() => void handleUpdateSkills([skill.id])}
        onDeleteFromCentral={() => void handleDeleteClick(skill)}
        detailButtonRef={(node) => setDetailButtonRef(skill.id, node)}
        platformIcons={{
          agents: availableInstallAgents,
          linkedAgents: skill.linked_agents,
          lockedAgentIds: skill.shared_root_agents,
          skillId: skill.id,
          onToggle: handleTogglePlatform,
          togglingAgentId,
        }}
        className="h-[212px]"
      />
    );
  }

  function getManualPrimaryLabel() {
    if (selectedSkillIds.length === 0) return t("central.categorizeSelectSkillsFirst");
    if (manualSelectedTagIds.length === 0) return t("central.categorizeSelectTagsFirst");
    return t("central.applyToSelectedSkills", { count: selectedSkillIds.length });
  }

  function getManualDisabledReason() {
    if (selectedSkillIds.length === 0) return t("central.categorizeSelectSkillsReason");
    if (manualSelectedTagIds.length === 0) return t("central.categorizeSelectTagsReason");
    if (isMetadataUpdating) return t("central.categorizeApplyingReason");
    return null;
  }

  function getAiPrimaryLabel() {
    if (selectedSkillIds.length === 0) return t("central.categorizeSelectSkillsFirst");
    if (!aiTaggingAvailable) return t("central.aiTaggingConfigureFirst");
    if (isSuggestingTags || aiTagJob.status === "running") return t("central.aiTaggingRunning");
    return t("central.aiSuggestSelected", { count: selectedSkillIds.length });
  }

  function getAiDisabledReason() {
    if (!aiTaggingAvailable) return t("central.aiTaggingConfigureReason");
    if (selectedSkillIds.length === 0) return t("central.categorizeSelectSkillsReason");
    if (isSuggestingTags || aiTagJob.status === "running") return t("central.aiTaggingRunningReason");
    return null;
  }

  function renderCategorizeScope() {
    return (
      <section className="rounded-2xl border border-border/90 bg-background p-4 shadow-sm">
        <div className="mb-3 flex items-center gap-2 text-[13px] font-semibold text-foreground">
          <span className="grid size-5 place-items-center rounded-full bg-primary/15 text-[11px] font-bold text-primary ring-1 ring-primary/25">1</span>
          {t("central.categorizeRange")}
        </div>
        <div className="grid grid-cols-2 gap-2">
          <Button size="sm" className="h-auto min-h-8 whitespace-normal px-2 leading-tight" onClick={handleSelectCurrentFilter}>
            {t("central.selectCurrentFilter")}
          </Button>
          <Button variant="secondary" size="sm" className="h-auto min-h-8 whitespace-normal px-2 leading-tight" onClick={handleSelectUncategorized}>
            {t("central.selectUncategorized")}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="col-span-2 justify-start text-foreground/75 hover:text-foreground"
            onClick={() => setSelectedSkillIds([])}
          >
            <X className="size-3.5" />
            {t("central.clearSelection")}
          </Button>
        </div>
        {selectedSkillIds.length > 0 && (
          <div className="mt-3 grid grid-cols-2 gap-2 border-t border-border/70 pt-3">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setIsBatchInstallDialogOpen(true)}
              disabled={isInstalling}
              data-testid="batch-install-central-skills"
            >
              <Download className="size-3.5" />
              {t("central.batchInstallSelected", { count: selectedSkillIds.length })}
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleBatchDeleteClick}
              disabled={isDeleting}
              data-testid="batch-delete-central-skills"
            >
              <Trash2 className="size-3.5" />
              {t("central.batchDeleteSelected", { count: selectedSkillIds.length })}
            </Button>
          </div>
        )}
      </section>
    );
  }

  function renderManualCategorize() {
    return (
      <div className="space-y-3">
        <Input
          value={manualTagQuery}
          onChange={(event) => setManualTagQuery(event.target.value)}
          placeholder={t("central.searchTagsPlaceholder")}
          aria-label={t("central.searchTagsPlaceholder")}
          className="h-9 text-xs"
        />
        {canCreateManualTag && (
          <Button variant="ghost" size="sm" onClick={handleCreateManualTag}>
            <Plus className="size-3.5" />
            {t("central.createTagInline", { name: manualTagQuery.trim() })}
          </Button>
        )}
        <div className="flex max-h-52 flex-wrap gap-2 overflow-auto rounded-2xl border border-border/90 bg-muted/10 p-3 text-foreground">
          {filteredManualTags.map((tag) => {
            const selected = manualSelectedTagIds.includes(tag.id);
            return (
              <button
                key={tag.id}
                type="button"
                aria-pressed={selected}
                onClick={() => handleToggleManualTag(tag.id)}
                className={cn(
                  "inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium transition-colors",
                  selected
                    ? "border-primary/70 bg-primary/15 text-primary shadow-sm ring-1 ring-primary/20"
                    : "border-border bg-background text-foreground/75 hover:border-primary/40 hover:text-foreground"
                )}
              >
                {selected && <Check className="size-3" />}
                {tag.name}
              </button>
            );
          })}
        </div>
      </div>
    );
  }

  function renderAiCategorize() {
    return (
      <div className="space-y-3">
        <div className="rounded-2xl border border-border/90 bg-background p-3 text-xs shadow-sm">
          <div className="mb-1 font-medium text-foreground">{t("central.aiScopeTitle")}</div>
          <p className="leading-relaxed text-foreground/75">
            {t("central.aiScopeDesc", {
              selected: selectedSkillIds.length,
              total: sortedSkills.length,
            })}
          </p>
        </div>
        {!aiTaggingAvailable && (
          <div className="rounded-2xl border border-border/90 bg-muted/20 p-3 text-xs text-foreground/75">
            {t("central.aiTaggingNeedsConfig")}
          </div>
        )}
        <div className="rounded-2xl border border-border/90 bg-muted/10 p-3 text-xs text-foreground/75">
          {t("central.aiPreview", { count: selectedSkillIds.length })}
        </div>
        <div className="rounded-2xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-700 dark:text-amber-300">
          {t("central.aiTagRateHint")}
        </div>
      </div>
    );
  }

  function renderReviewQueue() {
    return (
      <div className="space-y-3">
        {aiTagReviews.length > 0 && manualSelectedTagIds.length === 0 && (
          <div className="rounded-2xl border border-border/90 bg-muted/20 p-3 text-xs text-foreground/75">
            {t("central.reviewReplaceNeedsTags")}
          </div>
        )}
        {aiTagReviews.length === 0 ? (
          <div className="rounded-2xl border border-border/90 bg-muted/20 p-4 text-center text-xs text-foreground/70">
            {t("central.reviewEmpty")}
          </div>
        ) : (
          aiTagReviews.map((review) => (
            <div
              key={`${review.skill_id}:${review.tag.id}`}
              className="rounded-2xl border border-border/90 bg-background p-3 shadow-sm"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium">{review.skill_name}</div>
                  <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-foreground/70">
                    <span className="rounded-full bg-muted px-2 py-0.5">
                      {review.tag.name}
                    </span>
                    <span>{Math.round(review.confidence * 100)}%</span>
                  </div>
                </div>
                <AlertTriangle className="size-4 shrink-0 text-amber-500" />
              </div>
              <p className="mt-2 text-xs leading-relaxed text-foreground/75">{review.reason}</p>
              <div className="mt-3 flex flex-wrap gap-2">
                <Button size="sm" variant="outline" onClick={() => handleAcceptReview(review)}>
                  {t("central.reviewAccept")}
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={manualSelectedTagIds.length === 0}
                  onClick={() => handleApplyManualTagsToReview(review)}
                >
                  {t("central.reviewChange")}
                </Button>
                <Button size="sm" variant="ghost" onClick={() => handleSkipReview(review)}>
                  {t("central.reviewSkip")}
                </Button>
              </div>
            </div>
          ))
        )}
      </div>
    );
  }

  function renderCategorizePanel() {
    const manualDisabledReason = getManualDisabledReason();
    const aiDisabledReason = getAiDisabledReason();
    const isManualActionDisabled = Boolean(manualDisabledReason);
    const isAiActionDisabled = Boolean(aiDisabledReason);

    return (
      <aside data-testid="central-categorize-sidebar" className="relative hidden min-h-0 shrink-0 border-l border-border bg-background/95 xl:flex xl:flex-col" style={{ width: categorizeSidebarWidth }}>
        <div
          role="separator"
          aria-label={t("central.resizeCategorizeSidebar")}
          aria-orientation="vertical"
          tabIndex={0}
          onPointerDown={startCategorizeSidebarResize}
          onKeyDown={handleCategorizeSidebarResizeKeyDown}
          className="absolute left-0 top-0 z-20 h-full w-2 -translate-x-1/2 cursor-col-resize rounded-full bg-transparent transition-colors hover:bg-primary/30 focus:outline-none focus-visible:bg-primary/40"
        />
        <div className="border-b border-border/80 bg-background p-5">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold tracking-tight text-foreground">{t("central.categorizePanelTitle")}</h2>
              <p className="mt-1 text-[13px] leading-5 text-foreground/75">
                {t("central.categorizePanelDesc")}
              </p>
            </div>
            <span className="grid size-8 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary ring-1 ring-primary/20"><ListChecks className="size-4" /></span>
          </div>
          <div className="mt-4 flex flex-wrap items-center gap-x-2 gap-y-1 rounded-2xl border border-border/90 bg-muted/10 px-3 py-2 text-xs shadow-sm">
            <span
              className={cn(
                "font-medium",
                selectedSkillIds.length > 0 ? "text-primary" : "text-foreground/70"
              )}
            >
              {t("central.selectedSkillSummary", { count: selectedSkillIds.length })}
            </span>
            <span className="h-3 w-px bg-border" />
            <span
              className={cn(
                "font-medium",
                manualSelectedTagIds.length > 0 ? "text-primary" : "text-foreground/70"
              )}
            >
              {t("central.pendingTagSummary", { count: manualSelectedTagIds.length })}
            </span>
          </div>
        </div>

        <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4 [scrollbar-width:thin] [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent">
          {renderCategorizeScope()}

          <section className="rounded-2xl border border-border/90 bg-background p-4 shadow-sm">
            <div className="mb-3 flex items-center gap-2 text-[13px] font-semibold text-foreground">
              <span className="grid size-5 place-items-center rounded-full bg-primary/15 text-[11px] font-bold text-primary ring-1 ring-primary/25">2</span>
              {t("central.categorizeIntent")}
            </div>
            <div
              role="tablist"
              aria-label={t("central.categorizeIntent")}
              className="mb-3 grid grid-cols-3 rounded-xl border border-border/60 bg-muted/10 p-1"
            >
              {(["manual", "ai", "review"] as const).map((tab) => (
                <button
                  key={tab}
                  type="button"
                  role="tab"
                  aria-selected={categorizeTab === tab}
                  onClick={() => setCategorizeTab(tab)}
                  className={cn(
                    "h-8 rounded-lg text-xs font-medium transition-colors",
                    categorizeTab === tab
                      ? "bg-primary text-primary-foreground shadow-sm"
                      : "text-foreground/70 hover:bg-background hover:text-foreground"
                  )}
                >
                  {t(`central.categorizeTab.${tab}`)}
                </button>
              ))}
            </div>
            {categorizeTab === "manual" && renderManualCategorize()}
            {categorizeTab === "ai" && renderAiCategorize()}
            {categorizeTab === "review" && renderReviewQueue()}
          </section>
        </div>

        <div className="border-t border-border/80 bg-background p-4 shadow-lg">
          <div className="mb-2 flex items-center gap-2 text-xs font-semibold text-foreground">
            <span className="grid size-5 place-items-center rounded-full bg-primary/15 text-[11px] font-bold text-primary ring-1 ring-primary/25">3</span>
            {t("central.categorizeApply")}
          </div>
          {categorizeTab === "manual" && (
            <>
              <Button
                className="w-full"
                disabled={isManualActionDisabled}
                onClick={handleApplyManualTags}
                data-testid="categorize-primary-action"
              >
                <Check className="size-3.5" />
                {getManualPrimaryLabel()}
              </Button>
              {manualDisabledReason && (
                <p className="mt-2 text-xs leading-relaxed text-foreground/75" data-testid="categorize-action-reason">
                  {manualDisabledReason}
                </p>
              )}
            </>
          )}
          {categorizeTab === "ai" && (
            <>
              <Button
                className="w-full"
                disabled={isAiActionDisabled}
                onClick={handleBulkSuggestTags}
                data-testid="categorize-primary-action"
              >
                <Wand2 className="size-3.5" />
                {getAiPrimaryLabel()}
              </Button>
              {aiDisabledReason && (
                <p className="mt-2 text-xs leading-relaxed text-foreground/75" data-testid="categorize-action-reason">
                  {aiDisabledReason}
                </p>
              )}
            </>
          )}
          {categorizeTab === "review" && (
            <p className="text-xs leading-relaxed text-foreground/70">
              {t("central.reviewApplyHint")}
            </p>
          )}
        </div>
      </aside>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b border-border px-6 py-4 flex items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-semibold">{t("central.title")}</h1>
            <Button
              variant="ghost"
              size="icon"
              onClick={handleRefresh}
              disabled={isLoading}
              aria-label={t("central.refresh")}
            >
              <RefreshCw className={`size-4 ${isLoading ? "animate-spin" : ""}`} />
            </Button>
          </div>
          <p className="text-xs text-muted-foreground mt-1">
            {t("central.scope")}
          </p>
          <p className="text-sm text-muted-foreground mt-0.5">
            {centralSkillsDir}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            onClick={handleCheckUpdates}
            disabled={isRemoteTarget || isCheckingUpdates || updateJob.status === "running"}
            title={isRemoteTarget ? t("targets.centralUpdatesUnsupported") : undefined}
          >
            <RefreshCw className={`size-3.5 ${isCheckingUpdates ? "animate-spin" : ""}`} />
            {t("central.checkUpdates")}
          </Button>
          {updateAvailableSkillIds.length > 0 && (
            <Button
              variant="default"
              onClick={() => void handleUpdateSkills(updateTargetSkillIds)}
              disabled={isRemoteTarget || updateTargetSkillIds.length === 0 || updatingSkillIds.length > 0}
              title={isRemoteTarget ? t("targets.centralUpdatesUnsupported") : undefined}
            >
              <Download className="size-3.5" />
              {selectedSkillIds.length > 0
                ? t("central.updateSelected", { count: updateTargetSkillIds.length })
                : t("central.updateAvailable", { count: updateTargetSkillIds.length })}
            </Button>
          )}
          <Button
            variant="outline"
            data-testid="central-portability-open"
            onClick={() => setIsPortabilityOpen(true)}
          >
            <FileJson className="size-3.5" />
            {t("central.portabilityOpen")}
          </Button>
          <Button variant="outline" onClick={() => setIsGitHubImportOpen(true)}>
            {t("marketplace.githubImportSecondaryCta")}
          </Button>
        </div>
      </div>

      {/* Search bar */}
      <div className="px-6 py-3 border-b border-border">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center">
          <div className="relative flex-1">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground pointer-events-none" />
            <Input
              placeholder={t("central.searchPlaceholder")}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 bg-muted/40"
              aria-label={t("central.searchPlaceholder")}
            />
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <ArrowUpDown className="size-3.5" />
              <span>{t("central.sortLabel")}</span>
            </div>
            <div
              role="group"
              aria-label={t("central.sortFieldLabel")}
              className="flex rounded-xl bg-muted/40 p-1"
            >
              {sortFieldOptions.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  aria-pressed={sortField === option.value}
                  onClick={() => setSortField(option.value)}
                  className={cn(
                    "h-7 rounded-lg px-3 text-xs font-medium transition-colors cursor-pointer",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
                    sortField === option.value
                      ? "bg-background/95 text-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-background/60 hover:text-foreground"
                  )}
                >
                  {option.label}
                </button>
              ))}
            </div>
            <div
              role="group"
              aria-label={t("central.sortDirectionLabel")}
              className="flex rounded-xl bg-muted/40 p-1"
            >
              {sortDirectionOptions.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  aria-pressed={sortDirection === option.value}
                  onClick={() => setSortDirection(option.value)}
                  className={cn(
                    "h-7 rounded-lg px-3 text-xs font-medium transition-colors cursor-pointer",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
                    sortDirection === option.value
                      ? "bg-background/95 text-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-background/60 hover:text-foreground"
                  )}
                >
                  {option.label}
                </button>
              ))}
            </div>
            <select
              value={tagFilter}
              onChange={(event) => setTagFilter(event.target.value)}
              aria-label={t("central.tagFilterLabel")}
              className="h-9 rounded-xl border border-border bg-background px-3 text-xs text-foreground"
            >
              <option value="all">{t("central.allTags")}</option>
              <option value="updates">{t("central.updatesAvailableOnly")}</option>
              <option value="uncategorized">{t("central.uncategorizedOnly")}</option>
              <option value="ai-review">{t("central.aiReviewOnly")}</option>
              {tags.map((tag) => (
                <option key={tag.id} value={tag.id}>
                  {tag.name}
                </option>
              ))}
            </select>
            <Button
              variant={repositoryFilter === "unassigned" ? "default" : "outline"}
              size="sm"
              onClick={() => setRepositoryFilter(repositoryFilter === "unassigned" ? "all" : "unassigned")}
            >
              {t("central.unassignedOnly")}
            </Button>
            <Button
              variant={tagFilter === "uncategorized" ? "default" : "outline"}
              size="sm"
              onClick={() => setTagFilter(tagFilter === "uncategorized" ? "all" : "uncategorized")}
            >
              {t("central.uncategorizedOnly")}
            </Button>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex min-h-0 flex-1">
        <aside
          data-testid="central-filter-sidebar"
          className="relative hidden min-h-0 shrink-0 border-r border-border/80 bg-muted/15 md:flex md:flex-col"
          style={{ width: filterSidebarWidth }}
        >
          <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-3 py-4 pr-2 [scrollbar-width:thin] [&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-border [&::-webkit-scrollbar-track]:bg-transparent">
            <FilterSection
              icon={<FolderOpen className="size-3.5" />}
              title={t("central.repositories")}
            >
              <RepositoryFilterButton
                active={repositoryFilter === "all"}
                count={skills.length}
                label={t("central.allRepositories")}
                sourceKind="all"
                testId="repository-filter-all"
                onClick={() => setRepositoryFilter("all")}
              />
              {repositories.map((repository) => {
                const sourceKind = isGithubRepository(repository)
                  ? "github"
                  : "local";
                const isActive =
                  (repository.is_unknown && repositoryFilter === "unassigned") ||
                  repositoryFilter === repository.id;

                return (
                  <RepositoryFilterButton
                    key={repository.id}
                    active={isActive}
                    count={getRepositorySkillCount(repository)}
                    label={repository.name}
                    sourceKind={sourceKind}
                    testId={`repository-filter-${repository.id}`}
                    onClick={() =>
                      setRepositoryFilter(repository.is_unknown ? "unassigned" : repository.id)
                    }
                  />
                );
              })}
            </FilterSection>

            <FilterSection
              icon={<Tag className="size-3.5" />}
              title={t("central.categoryNav")}
            >
              <TagFilterButton
                active={tagFilter === "all"}
                count={skills.length}
                label={t("central.allTags")}
                tone="all"
                testId="tag-filter-all"
                onClick={() => setTagFilter("all")}
              />
              <TagFilterButton
                active={tagFilter === "uncategorized"}
                count={uncategorizedCount}
                label={t("central.uncategorizedOnly")}
                tone="uncategorized"
                testId="tag-filter-uncategorized"
                onClick={() => setTagFilter("uncategorized")}
              />
              <TagFilterButton
                active={tagFilter === "updates"}
                count={updateAvailableSkillIds.length}
                label={t("central.updatesAvailableOnly")}
                tone="updates"
                testId="tag-filter-updates"
                onClick={() => setTagFilter("updates")}
              />
              <TagFilterButton
                active={tagFilter === "ai-review"}
                count={aiTagReviews.length}
                label={t("central.aiReviewOnly")}
                tone="ai"
                testId="tag-filter-ai-review"
                onClick={() => {
                  setTagFilter("ai-review");
                  setCategorizeTab("review");
                }}
              />
              {tagCounts.map(({ tag, count }) => (
                <TagFilterButton
                  key={tag.id}
                  active={tagFilter === tag.id}
                  count={count}
                  label={tag.name}
                  tone="tag"
                  testId={`tag-filter-${tag.id}`}
                  onClick={() => setTagFilter(tag.id)}
                />
              ))}
            </FilterSection>
          </div>
          <div
            role="separator"
            aria-label={t("central.resizeFilterSidebar")}
            aria-orientation="vertical"
            tabIndex={0}
            onPointerDown={startFilterSidebarResize}
            onKeyDown={handleFilterSidebarResizeKeyDown}
            className="absolute right-0 top-0 z-20 h-full w-1.5 translate-x-1/2 cursor-col-resize rounded-full bg-transparent transition-colors hover:bg-primary/30 focus:outline-none focus-visible:bg-primary/40"
          />
        </aside>

        <div ref={contentRef} className="flex-1 overflow-auto p-6">
        {isLoading ? (
          <EmptyState message={t("central.loading")} />
        ) : skills.length === 0 ? (
          <FirstVisitEmptyState />
        ) : filteredSkills.length === 0 ? (
          <EmptyState message={t("central.noMatch", { query: searchQuery })} />
        ) : isSearchActive ? (
          sortedSkills.length > 60 ? (
            <VirtualizedList
              items={sortedSkills}
              itemHeight={132}
              itemGap={12}
              overscan={8}
              scrollContainerRef={contentRef}
              itemKey={(skill) => skill.id}
              renderItem={(skill) => renderSearchResult(skill)}
            />
          ) : (
            <div className="space-y-3">
              {sortedSkills.map((skill) => renderSearchResult(skill))}
            </div>
          )
        ) : sortedSkills.length > 40 ? (
          <VirtualizedGrid
            items={sortedSkills}
            itemHeight={212}
            rowGap={16}
            columnGap={16}
            overscanRows={3}
            minColumnWidth={420}
            maxColumns={2}
            scrollContainerRef={contentRef}
            itemKey={(skill) => skill.id}
            renderItem={(skill) => renderGridCard(skill)}
          />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {sortedSkills.map((skill) => renderGridCard(skill))}
          </div>
        )}
        </div>

        {skills.length > 0 && renderCategorizePanel()}
      </div>

      {aiTagJob.status !== "idle" && (
        <div className="sticky bottom-0 z-10 border-t border-border bg-background/95 px-6 py-3 shadow-lg backdrop-blur">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-3 text-xs">
            <div className="flex items-center gap-2 font-medium text-foreground">
              <Wand2 className="size-3.5" />
              <span>{t("central.aiTagProgressTitle")}</span>
              {aiTagJob.currentSkillName && aiTagJob.status === "running" && (
                <span className="text-muted-foreground">
                  {t("central.aiTagCurrent", { name: aiTagJob.currentSkillName })}
                </span>
              )}
            </div>
            <div className="flex flex-wrap items-center gap-3 text-muted-foreground">
              <span>{t("central.aiTagCompleted", { completed: aiTagJob.completed, total: aiTagJob.total })}</span>
              <span>{t("central.aiTagSucceeded", { count: aiTagJob.succeeded })}</span>
              <span>{t("central.aiTagFailed", { count: aiTagJob.failed })}</span>
              <span>{t("central.aiTagLowConfidence", { count: aiTagJob.lowConfidenceCount })}</span>
              {aiTagJob.status === "completed" && (
                <button
                  type="button"
                  className="inline-flex items-center gap-1 text-foreground underline-offset-4 hover:underline"
                  onClick={() => {
                    setCategorizeTab("review");
                    setTagFilter("ai-review");
                  }}
                >
                  <Eye className="size-3.5" />
                  {t("central.viewFailuresAndReviews")}
                </button>
              )}
              {aiTagJob.status === "running" && (
                <Button variant="outline" size="sm" onClick={handleCancelAiTagJob}>
                  <X className="size-3.5" />
                  {t("central.cancelAiTagJob")}
                </Button>
              )}
            </div>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full w-full origin-left rounded-full bg-primary transition-transform duration-300 ease-out"
              style={{
                transform: `scaleX(${aiTagJob.total > 0 ? aiTagJob.completed / aiTagJob.total : 0})`,
              }}
            />
          </div>
          {aiTagJob.error && (
            <p className="mt-2 text-xs text-destructive">{aiTagJob.error}</p>
          )}
        </div>
      )}

      {updateJob.status !== "idle" && (
        <div className="sticky bottom-0 z-10 border-t border-border bg-background/95 px-6 py-3 shadow-lg backdrop-blur">
          <div className="mb-2 flex flex-wrap items-center justify-between gap-3 text-xs">
            <div className="flex items-center gap-2 font-medium text-foreground">
              <Download className="size-3.5" />
              <span>{t("central.updateProgressTitle")}</span>
              {updateJob.currentSkillName && updateJob.status === "running" && (
                <span className="text-muted-foreground">
                  {t("central.updateCurrent", { name: updateJob.currentSkillName })}
                </span>
              )}
            </div>
            <div className="flex flex-wrap items-center gap-3 text-muted-foreground">
              <span>{t("central.updateCompleted", { completed: updateJob.completed, total: updateJob.total })}</span>
              <span>{t("central.updateSucceeded", { count: updateJob.succeeded })}</span>
              <span>{t("central.updateSkipped", { count: updateJob.skipped })}</span>
              <span>{t("central.updateFailed", { count: updateJob.failed })}</span>
            </div>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full w-full origin-left rounded-full bg-primary transition-transform duration-300 ease-out"
              style={{
                transform: `scaleX(${updateJob.total > 0 ? updateJob.completed / updateJob.total : 0})`,
              }}
            />
          </div>
          {updateJob.error && (
            <p className="mt-2 text-xs text-destructive">{updateJob.error}</p>
          )}
        </div>
      )}

      {/* Install Dialog */}
      <InstallDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
        skill={installTargetSkill}
        agents={availableInstallAgents}
        onInstall={handleInstall}
      />

      <BatchInstallCentralSkillsDialog
        open={isBatchInstallDialogOpen}
        onOpenChange={setIsBatchInstallDialogOpen}
        skillCount={selectedSkillIds.length}
        agents={availableInstallAgents}
        isInstalling={isInstalling}
        onInstall={handleBatchInstallCentralSkills}
      />

      <DeleteCentralSkillDialog
        open={isDeleteDialogOpen}
        onOpenChange={handleDeleteDialogOpenChange}
        skill={deleteTargetSkill}
        detail={deletePreview}
        agents={agents}
        isPreviewLoading={isDeletePreviewLoading}
        isDeleting={isDeleting}
        error={deletePreviewError}
        onConfirm={handleDeleteCentralSkill}
      />

      <BatchDeleteCentralSkillsDialog
        open={isBatchDeleteDialogOpen}
        onOpenChange={handleBatchDeleteDialogOpenChange}
        skillIds={selectedSkillIds}
        preview={batchDeletePreview}
        agents={agents}
        isPreviewLoading={isBatchDeletePreviewLoading}
        isDeleting={isDeleting}
        error={batchDeletePreviewError}
        onConfirm={handleBatchDeleteCentralSkills}
      />

      <SkillDetailDrawer
        open={isDrawerOpen}
        skillId={drawerSkillId}
        onOpenChange={(open) => {
          setIsDrawerOpen(open);
          if (!open) {
            setDrawerSkillId(null);
          }
        }}
        returnFocusRef={
          drawerSkillId
            ? {
                current: detailButtonRefs.current[drawerSkillId] ?? null,
              }
            : undefined
        }
      />

      <GitHubRepoImportWizard
        open={isGitHubImportOpen}
        onOpenChange={setIsGitHubImportOpen}
        repoUrl={githubRepoUrl}
        onRepoUrlChange={setGitHubRepoUrl}
        preview={githubImport.preview}
        previewError={githubImport.error}
        isPreviewLoading={githubImport.isPreviewLoading}
        isImporting={githubImport.isImporting}
        importResult={githubImport.importResult}
        onPreview={handleGitHubPreview}
        onImport={handleGitHubImport}
        availableAgents={availableInstallAgents}
        installableSkills={installableImportedSkills}
        onInstallImportedSkill={handleInstallImportedSkill}
        onAfterImportSuccess={handleAfterImportSuccess}
        onReset={() => {
          resetGitHubImport();
          setGitHubRepoUrl("");
        }}
        launcherLabel={t("central.title")}
      />

      <CentralStatePortabilityDialog
        open={isPortabilityOpen}
        onOpenChange={setIsPortabilityOpen}
        exportState={exportSkillportState}
        previewImport={previewSkillportStateImport}
        importState={importSkillportState}
        onAfterImport={async () => {
          await Promise.all([refreshCounts(), loadCentralSkills()]);
        }}
      />
    </div>
  );
}
