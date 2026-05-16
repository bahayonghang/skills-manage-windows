import { useMemo } from "react";
import type { TFunction } from "i18next";

import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
} from "@/lib/platformVisibility";
import { getPlatformTargetGroups } from "@/lib/platformTargetGroups";
import { formatPathForDisplay } from "@/lib/path";
import {
  BROWSER_PLATFORM_PATHS,
  getPlatformSkillDir,
  getPlatformSkillFilePath,
} from "@/lib/platformPathPolicy";
import { normalizeSearchQuery } from "@/lib/search";
import { isTauriRuntime } from "@/lib/tauri";
import type {
  AgentWithStatus,
  AiTagJob,
  CentralSkillUpdateJob,
  CentralSkillUpdateState,
  SkillportStatePortabilityJob,
  SkillAiTagReview,
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
    file_path: getPlatformSkillFilePath(
      BROWSER_PLATFORM_PATHS,
      "central",
      "fixture-central-skill"
    ),
    canonical_path: getPlatformSkillDir(
      BROWSER_PLATFORM_PATHS,
      "central",
      "fixture-central-skill"
    ),
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
const IDLE_PORTABILITY_JOB: SkillportStatePortabilityJob = {
  phase: null,
  status: "idle",
  total: 0,
  completed: 0,
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

async function noopUnlisten() {
  return () => {};
}

async function noopCancelPortability() {}

function noopResetGitHubImport() {}

export function useCentralSkillsStoreBindings(t: TFunction) {
  const rawSkills = useCentralSkillsStore((state) => state.skills);
  const rawAgents = useCentralSkillsStore((state) => state.agents);
  const rawRepositories = useCentralSkillsStore((state) => state.repositories);
  const rawTags = useCentralSkillsStore((state) => state.tags);
  const rawAiTagReviews = useCentralSkillsStore((state) => state.aiTagReviews);
  const rawAiTagJob = useCentralSkillsStore((state) => state.aiTagJob);
  const rawUpdateStatuses = useCentralSkillsStore((state) => state.updateStatuses);
  const rawUpdateJob = useCentralSkillsStore((state) => state.updateJob);
  const rawPortabilityJob = useCentralSkillsStore((state) => state.portabilityJob);
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
  const portabilityJob = rawPortabilityJob ?? IDLE_PORTABILITY_JOB;
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
    portabilityJob,
    activeTarget,
    isRemoteTarget: activeTarget.kind === "ssh",
    aiTaggingAvailable: rawAiTaggingAvailable ?? false,
    centralSkillsDir: formatPathForDisplay(
      agents.find((agent) => agent.id === "central")?.global_skills_dir ?? t("central.path")
    ),
    isLoading: shouldUseBrowserFixtures ? false : rawIsLoading ?? false,
    loadCentralSkills: rawLoadCentralSkills ?? noopAsync,
    subscribeAiTagProgress:
      useCentralSkillsStore((state) => state.subscribeAiTagProgress) ?? noopUnlisten,
    subscribeUpdateProgress:
      useCentralSkillsStore((state) => state.subscribeUpdateProgress) ?? noopUnlisten,
    subscribePortabilityProgress:
      useCentralSkillsStore((state) => state.subscribePortabilityProgress) ?? noopUnlisten,
    cancelSkillportStatePortability:
      useCentralSkillsStore((state) => state.cancelSkillportStatePortability) ??
      noopCancelPortability,
    isMetadataUpdating: useCentralSkillsStore((state) => state.isMetadataUpdating) ?? false,
    isSuggestingTags: useCentralSkillsStore((state) => state.isSuggestingTags) ?? false,
    isCheckingUpdates: useCentralSkillsStore((state) => state.isCheckingUpdates) ?? false,
    updatingSkillIds: useCentralSkillsStore((state) => state.updatingSkillIds) ?? [],
    isInstalling: useCentralSkillsStore((state) => state.isInstalling) ?? false,
    isDeleting: useCentralSkillsStore((state) => state.isDeleting) ?? false,
    togglingAgentId: useCentralSkillsStore((state) => state.togglingAgentId),
    refreshCounts: usePlatformStore((state) => state.refreshCounts) ?? noopAsync,
    availableInstallAgents: getPlatformTargetGroups(platformAgents, categoryVisibility),
    githubImport: useMarketplaceStore((state) => state.githubImport) ?? EMPTY_GITHUB_IMPORT_STATE,
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
  updateStatuses,
  selectedSkillIds,
  manualTagQuery,
}: {
  skills: SkillWithLinks[];
  tags: SkillTag[];
  updateStatuses: Record<string, CentralSkillUpdateState>;
  selectedSkillIds: string[];
  manualTagQuery: string;
}) {
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

  return {
    canCreateManualTag,
    filteredManualTags,
    selectedSkillIdSet,
    selectedUpdatableSkillIds,
    skillIdsKey,
    updateAvailableSkillIds,
    updateTargetSkillIds:
      selectedSkillIds.length > 0 ? selectedUpdatableSkillIds : updateAvailableSkillIds,
  };
}
