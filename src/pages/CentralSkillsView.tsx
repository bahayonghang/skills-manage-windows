import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { Search, RefreshCw, Blocks, FolderOpen, Settings } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";

import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillStore } from "@/stores/skillStore";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { SkillDetailDrawer } from "@/components/skill/SkillDetailDrawer";
import { InstallDialog } from "@/components/central/InstallDialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { AgentWithStatus, BatchInstallResult, SkillWithLinks } from "@/types";
import { GitHubRepoImportWizard } from "@/components/marketplace/GitHubRepoImportWizard";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { VirtualizedList } from "@/components/ui/virtualized-list";
import { VirtualizedGrid } from "@/components/ui/virtualized-grid";
import { buildSearchText, normalizeSearchQuery } from "@/lib/search";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  filterVisiblePlatformAgents,
} from "@/lib/platformVisibility";
import { isTauriRuntime } from "@/lib/tauri";
import { markAppPerformance } from "@/lib/performance";

const BROWSER_FIXTURE_SKILLS: SkillWithLinks[] = [
  {
    id: "fixture-central-skill",
    name: "fixture-central-skill",
    description: "Browser validation fixture for Central and drawer entry flows.",
    file_path: "~/.agents/skills/fixture-central-skill/SKILL.md",
    canonical_path: "~/.agents/skills/fixture-central-skill",
    is_central: true,
    source: "browser-fixture",
    scanned_at: "2026-04-17T00:00:00.000Z",
    linked_agents: ["claude-code"],
  },
];

const EMPTY_SKILLS: SkillWithLinks[] = [];
const EMPTY_AGENTS: AgentWithStatus[] = [];
const EMPTY_SKILLS_BY_AGENT: Record<string, number> = {};
const EMPTY_BATCH_INSTALL_RESULT: BatchInstallResult = {
  succeeded: [],
  failed: [],
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

async function noopBatchInstall(): Promise<BatchInstallResult> {
  return EMPTY_BATCH_INSTALL_RESULT;
}

async function noopPreviewGitHubRepoImport() {
  return null;
}

async function noopImportGitHubRepoSkills() {
  throw new Error("GitHub import is unavailable");
}

async function noopGetSkillsByAgent(_agentId: string): Promise<void> {}

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
            {t("empty.createHint")} <code className="font-mono">~/.agents/skills/my-skill/SKILL.md</code>
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

// ─── CentralSkillsView ────────────────────────────────────────────────────────

export function CentralSkillsView() {
  const { t } = useTranslation();
  const rawSkills = useCentralSkillsStore((state) => state.skills);
  const rawAgents = useCentralSkillsStore((state) => state.agents);
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
  const isLoading = shouldUseBrowserFixtures ? false : rawIsLoading ?? false;
  const loadCentralSkills = rawLoadCentralSkills ?? noopAsync;
  const installSkill = useCentralSkillsStore((state) => state.installSkill) ?? noopBatchInstall;
  const togglePlatformLink = useCentralSkillsStore((state) => state.togglePlatformLink) ?? noopAsync;
  const togglingAgentId = useCentralSkillsStore((state) => state.togglingAgentId);

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
    useMarketplaceStore((state) => state.resetGitHubImport) ?? (() => {});

  const [searchQuery, setSearchQuery] = useState("");
  const [installTargetSkill, setInstallTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [drawerSkillId, setDrawerSkillId] = useState<string | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isGitHubImportOpen, setIsGitHubImportOpen] = useState(false);
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
        searchText: buildSearchText([skill.name, skill.description]),
      })),
    [skills]
  );
  const isSearchActive = normalizedSearchQuery.length > 0;

  // Load central skills on mount.
  useEffect(() => {
    loadCentralSkills();
  }, [loadCentralSkills]);

  // Filter skills by search query.
  const filteredSkills = useMemo(() => {
    if (!normalizedSearchQuery) return skills;
    return searchableSkills
      .filter(({ searchText }) => searchText.includes(normalizedSearchQuery))
      .map(({ skill }) => skill);
  }, [normalizedSearchQuery, searchableSkills, skills]);

  useEffect(() => {
    if (!isSearchActive || !contentRef.current) return;
    contentRef.current.scrollTop = 0;
  }, [isSearchActive, normalizedSearchQuery]);

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
        const failedNames = result.failed.map((f) => f.agent_id).join(", ");
        toast.error(t("central.installPartialFail", { platforms: failedNames }));
      }
    } catch (err) {
      toast.error(t("central.installError", { error: String(err) }));
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
      toast.error(t("marketplace.installError", { error: String(err) }));
      throw err;
    }
  }

  async function handleInstallImportedSkill(
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy"
  ) {
    await handleInstall(skillId, agentIds, method);
    await Promise.all(agentIds.map((agentId) => getSkillsByAgent(agentId)));
  }

  const installableImportedSkills = useMemo(() => {
    if (!githubImport.importResult) return [];
    const importedIds = new Set(
      githubImport.importResult.importedSkills.map((skill) => skill.importedSkillId)
    );
    return skills.filter((skill) => importedIds.has(skill.id));
  }, [githubImport.importResult, skills]);

  const availableInstallAgents = useMemo(
    () => filterVisiblePlatformAgents(platformAgents, categoryVisibility),
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
        onDetail={() => handleOpenDrawer(skill.id)}
        onInstallTo={() => handleInstallClick(skill)}
        detailButtonRef={(node) => setDetailButtonRef(skill.id, node)}
        className="h-[104px]"
      />
    );
  }

  function renderGridCard(skill: SkillWithLinks) {
    return (
      <UnifiedSkillCard
        key={skill.id}
        name={skill.name}
        description={skill.description}
        onDetail={() => handleOpenDrawer(skill.id)}
        onInstallTo={() => handleInstallClick(skill)}
        detailButtonRef={(node) => setDetailButtonRef(skill.id, node)}
        platformIcons={{
          agents: availableInstallAgents,
          linkedAgents: skill.linked_agents,
          skillId: skill.id,
          onToggle: handleTogglePlatform,
          togglingAgentId,
        }}
        className="h-[188px]"
      />
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
          <p className="text-sm text-muted-foreground mt-0.5">
            {t("central.path")}
          </p>
        </div>
        <Button variant="outline" onClick={() => setIsGitHubImportOpen(true)}>
          {t("marketplace.githubImportSecondaryCta")}
        </Button>
      </div>

      {/* Search bar */}
      <div className="px-6 py-3 border-b border-border">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground pointer-events-none" />
          <Input
            placeholder={t("central.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8 bg-muted/40"
            aria-label={t("central.searchPlaceholder")}
          />
        </div>
      </div>

      {/* Content */}
      <div ref={contentRef} className="flex-1 overflow-auto p-6">
        {isLoading ? (
          <EmptyState message={t("central.loading")} />
        ) : skills.length === 0 ? (
          <FirstVisitEmptyState />
        ) : filteredSkills.length === 0 ? (
          <EmptyState message={t("central.noMatch", { query: searchQuery })} />
        ) : isSearchActive ? (
          filteredSkills.length > 60 ? (
            <VirtualizedList
              items={filteredSkills}
              itemHeight={104}
              itemGap={12}
              overscan={8}
              scrollContainerRef={contentRef}
              itemKey={(skill) => skill.id}
              renderItem={(skill) => renderSearchResult(skill)}
            />
          ) : (
            <div className="space-y-3">
              {filteredSkills.map((skill) => renderSearchResult(skill))}
            </div>
          )
        ) : filteredSkills.length > 40 ? (
          <VirtualizedGrid
            items={filteredSkills}
            itemHeight={188}
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
            {filteredSkills.map((skill) => renderGridCard(skill))}
          </div>
        )}
      </div>

      {/* Install Dialog */}
      <InstallDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
        skill={installTargetSkill}
        agents={availableInstallAgents}
        onInstall={handleInstall}
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
    </div>
  );
}
