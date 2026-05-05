import { useMemo } from "react";
import type { TFunction } from "i18next";

import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillStore } from "@/stores/skillStore";
import { useTargetStore } from "@/stores/targetStore";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
} from "@/lib/platformVisibility";
import { getPlatformTargetGroups } from "@/lib/platformTargetGroups";
import { formatPathForDisplay } from "@/lib/path";
import { buildSearchText, normalizeSearchQuery } from "@/lib/search";
import { isTauriRuntime } from "@/lib/tauri";
import type {
  AgentWithStatus,
  AiTagJob,
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillResult,
  CentralBatchInstallResult,
  CentralSkillUpdateJob,
  CentralSkillUpdateState,
  DeleteSkillRepositoryPreview,
  DeleteSkillRepositoryResult,
  ScannedSkill,
  SkillAiTagReview,
  SkillDetail,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
  SkillportStateImportPreview,
  SkillportStateImportResolution,
  SkillportStateImportResult,
} from "@/types";

export type CentralSortField = "name" | "createdAt" | "updatedAt";
export type CentralSortDirection = "asc" | "desc";
export type CentralCategorizeTab = "manual" | "ai" | "review";

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
  return { succeeded: [], failed: [] };
}

async function noopBatchInstallSkills(): Promise<CentralBatchInstallResult> {
  return { succeeded: [], failed: [] };
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

async function noopLoadRepositoryDeletePreview(): Promise<DeleteSkillRepositoryPreview> {
  throw new Error("Repository deletion is unavailable");
}

async function noopDeleteCentralSkill(): Promise<void> {}

async function noopDeleteCentralSkills(): Promise<BatchDeleteCentralSkillResult> {
  return { succeeded: [], failed: [] };
}

async function noopDeleteSkillRepository(): Promise<DeleteSkillRepositoryResult> {
  throw new Error("Repository deletion is unavailable");
}

async function noopUnlisten() {
  return () => {};
}

async function noopCheckUpdates(): Promise<CentralSkillUpdateState[]> {
  return [];
}

async function noopUpdateSkills() {
  return { succeeded: [], failed: [], skipped: [], states: [] };
}

async function noopKeepRemoteMissingSkills(): Promise<string[]> {
  return [];
}

function noopResetGitHubImport() {}

function parseSortableTimestamp(value?: string | null): number {
  if (!value) return 0;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? timestamp : 0;
}

function getSkillSortTimestamp(
  skill: SkillWithLinks,
  field: CentralSortField
): number {
  return parseSortableTimestamp(
    field === "createdAt"
      ? skill.created_at ?? skill.scanned_at
      : skill.updated_at ?? skill.scanned_at
  );
}

export function useCentralSkillsStoreBindings(t: TFunction) {
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
  const rawLoadCentralSkills = useCentralSkillsStore((state) => state.loadCentralSkills);
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
  const activeTarget = useTargetStore((state) => state.activeTarget);
  const categoryVisibility =
    usePlatformStore((state) => state.categoryVisibility) ??
    DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const platformAgents = usePlatformStore((state) => state.agents) ?? EMPTY_AGENTS;

  return {
    skills,
    agents,
    repositories,
    tags,
    aiTagReviews,
    aiTagJob,
    updateStatuses,
    updateJob,
    activeTarget,
    isRemoteTarget: activeTarget.kind === "ssh",
    aiTaggingAvailable: rawAiTaggingAvailable ?? false,
    centralSkillsDir: formatPathForDisplay(
      agents.find((agent) => agent.id === "central")?.global_skills_dir ?? t("central.path")
    ),
    isLoading: shouldUseBrowserFixtures ? false : rawIsLoading ?? false,
    loadCentralSkills: rawLoadCentralSkills ?? noopAsync,
    installSkill: useCentralSkillsStore((state) => state.installSkill) ?? noopInstallSkill,
    batchInstallSkills:
      useCentralSkillsStore((state) => state.batchInstallSkills) ?? noopBatchInstallSkills,
    loadDeletePreview:
      useCentralSkillsStore((state) => state.loadDeletePreview) ?? noopLoadDeletePreview,
    loadBatchDeletePreview:
      useCentralSkillsStore((state) => state.loadBatchDeletePreview) ??
      noopLoadBatchDeletePreview,
    loadRepositoryDeletePreview:
      useCentralSkillsStore((state) => state.loadRepositoryDeletePreview) ??
      noopLoadRepositoryDeletePreview,
    deleteCentralSkill:
      useCentralSkillsStore((state) => state.deleteCentralSkill) ?? noopDeleteCentralSkill,
    deleteCentralSkills:
      useCentralSkillsStore((state) => state.deleteCentralSkills) ?? noopDeleteCentralSkills,
    deleteSkillRepository:
      useCentralSkillsStore((state) => state.deleteSkillRepository) ??
      noopDeleteSkillRepository,
    togglePlatformLink:
      useCentralSkillsStore((state) => state.togglePlatformLink) ?? noopAsync,
    createTag: useCentralSkillsStore((state) => state.createTag),
    assignSkillTags: useCentralSkillsStore((state) => state.assignSkillTags),
    bulkSuggestSkillTags: useCentralSkillsStore((state) => state.bulkSuggestSkillTags),
    cancelAiTagJob: useCentralSkillsStore((state) => state.cancelAiTagJob),
    cancelCentralUpdates: useCentralSkillsStore((state) => state.cancelCentralUpdates),
    acceptAiTagReview: useCentralSkillsStore((state) => state.acceptAiTagReview),
    skipAiTagReview: useCentralSkillsStore((state) => state.skipAiTagReview),
    checkSkillUpdates:
      useCentralSkillsStore((state) => state.checkSkillUpdates) ?? noopCheckUpdates,
    updateSkills: useCentralSkillsStore((state) => state.updateSkills) ?? noopUpdateSkills,
    keepRemoteMissingSkills:
      useCentralSkillsStore((state) => state.keepRemoteMissingSkills) ??
      noopKeepRemoteMissingSkills,
    subscribeAiTagProgress:
      useCentralSkillsStore((state) => state.subscribeAiTagProgress) ?? noopUnlisten,
    subscribeUpdateProgress:
      useCentralSkillsStore((state) => state.subscribeUpdateProgress) ?? noopUnlisten,
    isMetadataUpdating: useCentralSkillsStore((state) => state.isMetadataUpdating) ?? false,
    isSuggestingTags: useCentralSkillsStore((state) => state.isSuggestingTags) ?? false,
    isCheckingUpdates: useCentralSkillsStore((state) => state.isCheckingUpdates) ?? false,
    updatingSkillIds: useCentralSkillsStore((state) => state.updatingSkillIds) ?? [],
    isInstalling: useCentralSkillsStore((state) => state.isInstalling) ?? false,
    isDeleting: useCentralSkillsStore((state) => state.isDeleting) ?? false,
    togglingAgentId: useCentralSkillsStore((state) => state.togglingAgentId),
    refreshCounts: usePlatformStore((state) => state.refreshCounts) ?? noopAsync,
    availableInstallAgents: getPlatformTargetGroups(platformAgents, categoryVisibility),
    skillsByAgent: useSkillStore((state) => state.skillsByAgent) ?? EMPTY_SKILLS_BY_AGENT,
    getSkillsByAgent: useSkillStore((state) => state.getSkillsByAgent) ?? noopGetSkillsByAgent,
    githubImport: useMarketplaceStore((state) => state.githubImport) ?? EMPTY_GITHUB_IMPORT_STATE,
    previewGitHubRepoImport:
      useMarketplaceStore((state) => state.previewGitHubRepoImport) ??
      noopPreviewGitHubRepoImport,
    importGitHubRepoSkills:
      useMarketplaceStore((state) => state.importGitHubRepoSkills) ??
      noopImportGitHubRepoSkills,
    resetGitHubImport:
      useMarketplaceStore((state) => state.resetGitHubImport) ?? noopResetGitHubImport,
    exportSkillportState:
      useCentralSkillsStore((state) => state.exportSkillportState) ?? noopExportSkillportState,
    previewSkillportStateImport:
      useCentralSkillsStore((state) => state.previewSkillportStateImport) ??
      noopPreviewSkillportStateImport,
    importSkillportState:
      useCentralSkillsStore((state) => state.importSkillportState) ?? noopImportSkillportState,
  };
}

export function useCentralSkillsDerivedData({
  skills,
  tags,
  aiTagReviews,
  updateStatuses,
  selectedSkillIds,
  searchQuery,
  effectiveSearchQuery,
  manualTagQuery,
  repositoryFilter,
  tagFilter,
  sortField,
  sortDirection,
}: {
  skills: SkillWithLinks[];
  tags: SkillTag[];
  aiTagReviews: SkillAiTagReview[];
  updateStatuses: Record<string, CentralSkillUpdateState>;
  selectedSkillIds: string[];
  searchQuery: string;
  effectiveSearchQuery: string;
  manualTagQuery: string;
  repositoryFilter: string;
  tagFilter: string;
  sortField: CentralSortField;
  sortDirection: CentralSortDirection;
}) {
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
    () =>
      selectedSkillIds.filter(
        (skillId) => updateStatuses[skillId]?.status === "update_available"
      ),
    [selectedSkillIds, updateStatuses]
  );
  const selectedSkillIdSet = useMemo(
    () => new Set(selectedSkillIds),
    [selectedSkillIds]
  );
  const uncategorizedCount = useMemo(
    () =>
      skills.filter((skill) => {
        const skillTags = skill.tags ?? [];
        return skillTags.length === 0 || skillTags.some((tag) => tag.id === "uncategorized");
      }).length,
    [skills]
  );
  const tagCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const skill of skills) {
      for (const tag of skill.tags ?? []) {
        counts.set(tag.id, (counts.get(tag.id) ?? 0) + 1);
      }
    }
    return tags.map((tag) => ({ tag, count: counts.get(tag.id) ?? 0 }));
  }, [skills, tags]);
  const filteredManualTags = useMemo(() => {
    const query = normalizeSearchQuery(manualTagQuery);
    return query ? tags.filter((tag) => normalizeSearchQuery(tag.name).includes(query)) : tags;
  }, [manualTagQuery, tags]);
  const canCreateManualTag = useMemo(() => {
    const name = manualTagQuery.trim();
    if (!name) return false;
    return !tags.some((tag) => normalizeSearchQuery(tag.name) === normalizeSearchQuery(name));
  }, [manualTagQuery, tags]);
  const skillIdsKey = useMemo(() => skills.map((skill) => skill.id).join("\0"), [skills]);
  const isSearchActive = normalizedSearchQuery.length > 0;

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
          (tagFilter === "ai-review" ? aiReviewSkillIds.has(skill.id) : false) ||
          (tagFilter === "uncategorized"
            ? skillTags.length === 0 || skillTags.some((tag) => tag.id === "uncategorized")
            : skillTags.some((tag) => tag.id === tagFilter));
        return matchesRepository && matchesTag;
      });
  }, [
    aiReviewSkillIds,
    normalizedSearchQuery,
    repositoryFilter,
    searchableSkills,
    tagFilter,
    updateStatuses,
  ]);
  const sortedSkills = useMemo(() => {
    const list = [...filteredSkills];
    const direction = sortDirection === "asc" ? 1 : -1;
    return list.sort((a, b) => {
      const nameComparison = a.name.localeCompare(b.name, undefined, {
        numeric: true,
        sensitivity: "base",
      });

      if (sortField === "name") return nameComparison * direction;

      const timeComparison =
        getSkillSortTimestamp(a, sortField) - getSkillSortTimestamp(b, sortField);
      return timeComparison === 0 ? nameComparison : timeComparison * direction;
    });
  }, [filteredSkills, sortDirection, sortField]);

  return {
    canCreateManualTag,
    filteredManualTags,
    filteredSkills,
    isSearchActive,
    normalizedSearchQuery,
    searchQuery,
    selectedSkillIdSet,
    selectedUpdatableSkillIds,
    skillIdsKey,
    sortedSkills,
    tagCounts,
    uncategorizedCount,
    updateAvailableSkillIds,
    updateTargetSkillIds:
      selectedSkillIds.length > 0 ? selectedUpdatableSkillIds : updateAvailableSkillIds,
  };
}
