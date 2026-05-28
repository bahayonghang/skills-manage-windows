import { useEffect, useMemo, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { Blocks, Search } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillStore } from "@/stores/skillStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useSkillDetailStore } from "@/stores/skillDetailStore";
import { useUpdateCenterStore } from "@/stores/updateCenterStore";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { SkillDetailDrawer } from "@/components/skill/SkillDetailDrawer";
import { BatchDeletePlatformSkillsDialog } from "@/components/platform/BatchDeletePlatformSkillsDialog";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { PlatformGroupedSkillList } from "@/components/platform/PlatformGroupedSkillList";
import { PlatformTransferActionBar, PlatformTransferRail } from "@/components/platform/PlatformTransferRail";
import { PlatformSkillSortMenu, PlatformSkillViewMenu } from "@/components/platform/PlatformSkillToolbarMenus";
import { InstallDialog, type InstallMethod } from "@/components/central/InstallDialog";
import { BatchInstallCentralSkillsDialog } from "@/components/central/BatchInstallCentralSkillsDialog";
import { VirtualizedGrid } from "@/components/ui/virtualized-grid";
import { usePlatformSkillSelection } from "@/hooks/usePlatformSkillSelection";
import { useSkillExplanationSummaries } from "@/hooks/useSkillExplanationSummaries";
import { useSkillCallCounts } from "@/hooks/useSkillCallCounts";
import { createPlatformBatchUninstallRequest, getFailedBatchUninstallActionKeys, getPlatformSkillActionKey, getPlatformSkillRowKey, getPlatformSkillSummaryKeys } from "@/lib/platformBatchActions";
import { formatPathForDisplay } from "@/lib/path";
import { cn } from "@/lib/utils";
import { ScannedSkill, SkillWithLinks } from "@/types";
import { selectMostUniversalSkills } from "@/lib/centralInstalledFilters";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  filterVisiblePlatformAgents,
} from "@/lib/platformVisibility";
import {
  getPlatformTargetMemberIds,
  getPlatformTargetGroups,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";
import {
  derivePlatformSkillRows,
  type PlatformGroupBy,
  type PlatformSortDirection,
  type PlatformSortField,
} from "@/lib/platformSkillViewModel";

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

type ClaudeSourceFilter = "all" | "user" | "plugin";

// ─── PlatformView ─────────────────────────────────────────────────────────────

export function PlatformView() {
  const { agentId } = useParams<{ agentId: string }>();
  const { t, i18n } = useTranslation();
  const agents = usePlatformStore((state) => state.agents);
  const categoryVisibility = usePlatformStore((state) => state.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const scanGeneration = usePlatformStore((state) => state.scanGeneration ?? 0);
  const rescan = usePlatformStore((state) => state.rescan);

  const skillsByAgent = useSkillStore((state) => state.skillsByAgent);
  const loadingByAgent = useSkillStore((state) => state.loadingByAgent);
  const pendingSkillActionKeys = useSkillStore((state) => state.pendingSkillActionKeys);
  const getSkillsByAgent = useSkillStore((state) => state.getSkillsByAgent);
  const uninstallSkillFromAgent = useSkillStore((state) => state.uninstallSkillFromAgent);
  const batchUninstallSkillsFromAgent = useSkillStore(
    (state) => state.batchUninstallSkillsFromAgent
  );

  const centralSkills = useCentralSkillsStore((state) => state.skills);
  const loadCentralSkills = useCentralSkillsStore((state) => state.loadCentralSkills);
  const installSkill = useCentralSkillsStore((state) => state.installSkill);
  const batchInstallSkills = useCentralSkillsStore((state) => state.batchInstallSkills);
  const currentDetail = useSkillDetailStore((state) => state.detail);
  const refreshDetailInstallations = useSkillDetailStore((state) => state.refreshInstallations);
  const refreshCounts = usePlatformStore((state) => state.refreshCounts);
  const scanDuplicates = useUpdateCenterStore((state) => state.scanDuplicates);
  const openUpdateCenter = useUpdateCenterStore((state) => state.openDialog);

  const [searchQuery, setSearchQuery] = useState("");
  const [sourceFilter, setSourceFilter] = useState<ClaudeSourceFilter>("all");
  const [sortField, setSortField] = useState<PlatformSortField>("name");
  const [sortDirection, setSortDirection] = useState<PlatformSortDirection>("asc");
  const [groupBy, setGroupBy] = useState<PlatformGroupBy>("none");
  const [installTargetSkill, setInstallTargetSkill] = useState<SkillWithLinks | null>(null);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [isBatchInstallDialogOpen, setIsBatchInstallDialogOpen] = useState(false);
  const [isBatchInstalling, setIsBatchInstalling] = useState(false);
  const [isBatchDeleteDialogOpen, setIsBatchDeleteDialogOpen] = useState(false);
  const [isBatchDeleting, setIsBatchDeleting] = useState(false);
  const [drawerSkill, setDrawerSkill] = useState<ScannedSkill | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isDuplicateScanning, setIsDuplicateScanning] = useState(false);
  const [returnFocusRowKey, setReturnFocusRowKey] = useState<string | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);
  const detailButtonRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  const platformTargets = useMemo(
    () => getPlatformTargetGroups(agents, categoryVisibility),
    [agents, categoryVisibility]
  );
  const visibleAgents = useMemo(
    () => filterVisiblePlatformAgents(agents, categoryVisibility),
    [agents, categoryVisibility]
  );
  const installTargetAgents = platformTargets;
  const platformTarget = platformTargets.find((a) => a.id === agentId);
  const directAgent = visibleAgents.find((a) => a.id === agentId);
  const agent = platformTarget ?? directAgent;
  const isUniversalPage = agent ? isUniversalPlatformTarget(agent) : false;
  const resolvedAgentId = agent
    ? isUniversalPlatformTarget(agent)
      ? agent.install_agent_id
      : agent.id
    : undefined;
  const platformDisplayName = agent
    ? isUniversalPage
      ? t("platformTargets.universalShortLabel")
      : agent.display_name
    : "";
  const isClaudePage = !isUniversalPage && agent?.id === "claude-code";
  const isCodingPlatformPage = agent?.category === "coding";
  const defaultBatchExcludedAgentIds = useMemo(
    () =>
      agent
        ? installTargetAgents
            .filter((target) => {
              if (target.id === agent.id) return true;
              return resolvedAgentId
                ? getPlatformTargetMemberIds(target).includes(resolvedAgentId)
                : false;
            })
            .map((target) => target.id)
        : [],
    [agent, installTargetAgents, resolvedAgentId]
  );

  // Load skills for this agent when the route changes or a fresh scan completes.
  useEffect(() => {
    if (resolvedAgentId) {
      getSkillsByAgent(resolvedAgentId);
    }
  }, [resolvedAgentId, getSkillsByAgent, scanGeneration]);

  useEffect(() => {
    if (!contentRef.current) return;
    contentRef.current.scrollTop = 0;
  }, [agentId]);

  useEffect(() => {
    setSourceFilter("all");
  }, [agentId]);

  async function handleInstallClick(skillId: string) {
    if (centralSkills.length === 0) {
      await loadCentralSkills();
    }

    const target = useCentralSkillsStore
      .getState()
      .skills.find((skill) => skill.id === skillId);
    if (!target) {
      toast.error(t("central.installError", { error: t("platform.notFound") }));
      return;
    }
    setInstallTargetSkill(target);
    setIsDialogOpen(true);
  }

  async function handleInstall(
    skillId: string,
    agentIds: string[],
    method: string,
    projectPath?: string | null
  ) {
    try {
      const result = await installSkill(skillId, agentIds, method, projectPath);
      await refreshCounts();
      if (resolvedAgentId) {
        await getSkillsByAgent(resolvedAgentId);
      }
      if (currentDetail?.id === skillId) {
        await refreshDetailInstallations(skillId);
      }
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

  async function handleUninstall(skill: ScannedSkill) {
    if (!resolvedAgentId) return;
    const request = createPlatformBatchUninstallRequest(skill, resolvedAgentId);
    try {
      if (request.row_id) {
        await uninstallSkillFromAgent(skill.id, resolvedAgentId, request.row_id);
      } else {
        await uninstallSkillFromAgent(skill.id, resolvedAgentId);
      }
      await refreshCounts();
      await getSkillsByAgent(resolvedAgentId);
    } catch (err) {
      toast.error(t("detail.uninstallError", { error: String(err) }));
    }
  }

  async function handleScanDuplicates() {
    if (!resolvedAgentId) return;
    setIsDuplicateScanning(true);
    try {
      await rescan();
      await getSkillsByAgent(resolvedAgentId);
      await scanDuplicates([resolvedAgentId]);
      const inventory = useUpdateCenterStore.getState().inventory;
      const groups = inventory?.platformDuplicates ?? [];
      if (groups.length === 0) {
        toast.info(t("platform.duplicatesNone"));
        return;
      }

      const rowCount = groups.reduce(
        (sum, group) => sum + group.writablePaths.length,
        0
      );
      openUpdateCenter("duplicates");
      toast.success(
        t("platform.duplicatesFound", {
          skillCount: groups.length,
          rowCount,
        })
      );
    } catch (err) {
      toast.error(t("platform.duplicatesScanError", { error: String(err) }));
    } finally {
      setIsDuplicateScanning(false);
    }
  }

  const isLoading = resolvedAgentId ? (loadingByAgent[resolvedAgentId] ?? false) : false;

  // Memoize skills to avoid changing dependency reference on every render
  const skills = useMemo(
    () => (resolvedAgentId ? (skillsByAgent[resolvedAgentId] ?? []) : []),
    [resolvedAgentId, skillsByAgent]
  );

  const sourceCounts = useMemo(() => {
    const counts: Record<ClaudeSourceFilter, number> = {
      all: skills.length,
      user: 0,
      plugin: 0,
    };

    for (const skill of skills) {
      if (skill.source_kind === "user") {
        counts.user += 1;
      } else if (skill.source_kind === "plugin") {
        counts.plugin += 1;
      }
    }

    return counts;
  }, [skills]);

  const platformGroupLabels = useMemo(
    () => ({
      all: t("platform.group.all"),
      localSource: t("platform.group.localSource"),
      pluginSource: t("platform.group.pluginSource"),
      unknownSource: t("platform.group.unknownSource"),
    }),
    [t]
  );
  const platformRows = useMemo(
    () =>
      derivePlatformSkillRows({
        skills,
        searchQuery,
        sourceFilter: isClaudePage ? sourceFilter : "all",
        sort: { field: sortField, direction: sortDirection },
        groupBy,
        labels: platformGroupLabels,
      }),
    [
      groupBy,
      isClaudePage,
      platformGroupLabels,
      searchQuery,
      skills,
      sortDirection,
      sortField,
      sourceFilter,
    ]
  );
  const sourceFilteredSkills = platformRows.sourceFilteredSkills;
  const filteredSkills = platformRows.sortedSkills;
  const summarySkillIds = useMemo(
    () => filteredSkills.flatMap((skill) => getPlatformSkillSummaryKeys(skill)),
    [filteredSkills]
  );
  const aiSummaries = useSkillExplanationSummaries(summarySkillIds, "zh");
  const skillNamesForUsage = useMemo(
    () => Array.from(new Set(filteredSkills.map((s) => s.name))),
    [filteredSkills]
  );
  const usageCounts = useSkillCallCounts(skillNamesForUsage, 30);

  function getAiSummary(skill: ScannedSkill) {
    return (skill.row_id ? aiSummaries[skill.row_id] : undefined) ?? aiSummaries[skill.id];
  }
  const {
    selectedSkillKeys,
    setSelectedSkillKeys,
    selectableFilteredSkills,
    selectedPlatformSkills,
    toggleSelectedSkill,
    selectCurrentResults,
    clearSelectedSkills,
  } = usePlatformSkillSelection({
    agentId,
    filteredSkills,
  });
  const selectedBatchSkillIds = useMemo(
    () => Array.from(new Set(selectedPlatformSkills.map((skill) => skill.id))),
    [selectedPlatformSkills]
  );

  async function handleSelectMostUniversal() {
    if (selectableFilteredSkills.length === 0) return;

    if (useCentralSkillsStore.getState().skills.length === 0) {
      await loadCentralSkills();
    }

    const centralSkillById = new Map(
      useCentralSkillsStore.getState().skills.map((skill) => [skill.id, skill])
    );
    const centralCandidates = selectableFilteredSkills
      .map((skill) => centralSkillById.get(skill.id))
      .filter((skill): skill is SkillWithLinks => Boolean(skill));
    const selectedCentralSkillIds = new Set(
      selectMostUniversalSkills(centralCandidates).map((skill) => skill.id)
    );

    if (selectedCentralSkillIds.size === 0) {
      toast.info(t("platform.transferNoUniversalCoverage"));
      setSelectedSkillKeys(new Set());
      return;
    }

    setSelectedSkillKeys(
      new Set(
        selectableFilteredSkills
          .filter((skill) => selectedCentralSkillIds.has(skill.id))
          .map((skill) => getPlatformSkillRowKey(skill))
      )
    );
  }

  function handleClearTransferSelection() {
    clearSelectedSkills();
  }

  async function handleBatchInstall(
    agentIds: string[],
    method: InstallMethod,
    projectPath?: string | null
  ) {
    setIsBatchInstalling(true);
    try {
      const result = await batchInstallSkills(
        selectedBatchSkillIds,
        agentIds,
        method,
        projectPath
      );
      await refreshCounts();
      if (resolvedAgentId) {
        await getSkillsByAgent(resolvedAgentId);
      }
      await loadCentralSkills();
      if (currentDetail?.id && selectedBatchSkillIds.includes(currentDetail.id)) {
        await refreshDetailInstallations(currentDetail.id);
      }
      if (result.failed.length === 0) {
        setSelectedSkillKeys(new Set());
      }
      return result;
    } catch (err) {
      toast.error(t("platform.transferInstallError", { error: String(err) }));
      throw err;
    } finally {
      setIsBatchInstalling(false);
    }
  }

  async function handleBatchDelete() {
    if (!resolvedAgentId || selectedPlatformSkills.length === 0) return;

    const requests = selectedPlatformSkills.map((skill) =>
      createPlatformBatchUninstallRequest(skill, resolvedAgentId)
    );
    setIsBatchDeleting(true);
    try {
      const result = await batchUninstallSkillsFromAgent(resolvedAgentId, requests);
      await refreshCounts();
      await getSkillsByAgent(resolvedAgentId);
      await loadCentralSkills();

      const failedKeys = getFailedBatchUninstallActionKeys(resolvedAgentId, result);
      if (result.failed.length === 0) {
        clearSelectedSkills();
        setIsBatchDeleteDialogOpen(false);
        toast.success(t("platform.batchDeleteSuccess", { count: result.succeeded.length }));
      } else {
        setSelectedSkillKeys(failedKeys);
        toast.error(
          t("platform.batchDeletePartial", {
            succeeded: result.succeeded.length,
            failed: result.failed.length,
          })
        );
      }
    } catch (err) {
      toast.error(t("platform.batchDeleteError", { error: String(err) }));
      throw err;
    } finally {
      setIsBatchDeleting(false);
    }
  }

  useEffect(() => {
    if (!drawerSkill) return;

    const rowKey = getPlatformSkillRowKey(drawerSkill);
    const refreshedSkill = skills.find((skill) => getPlatformSkillRowKey(skill) === rowKey);

    if (!refreshedSkill) {
      setIsDrawerOpen(false);
      setDrawerSkill(null);
      return;
    }

    if (refreshedSkill !== drawerSkill) {
      setDrawerSkill(refreshedSkill);
    }
  }, [drawerSkill, skills]);

  function setDetailButtonRef(rowKey: string, node: HTMLButtonElement | null) {
    if (node) {
      detailButtonRefs.current[rowKey] = node;
      return;
    }
    delete detailButtonRefs.current[rowKey];
  }

  function handleOpenDrawer(skill: ScannedSkill) {
    setReturnFocusRowKey(getPlatformSkillRowKey(skill));
    setDrawerSkill(skill);
    setIsDrawerOpen(true);
  }

  if (!agent) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        {t("platform.notFound")}
      </div>
    );
  }

  const sourceTabs: { id: ClaudeSourceFilter; label: string; count: number }[] = [
    {
      id: "all",
      label: t("platform.sourceFilter.all", {
        defaultValue: i18n.language.startsWith("zh") ? "全部" : "All",
      }),
      count: sourceCounts.all,
    },
    {
      id: "user",
      label: t("platform.sourceFilter.user", {
        defaultValue: i18n.language.startsWith("zh") ? "用户来源" : "User source",
      }),
      count: sourceCounts.user,
    },
    {
      id: "plugin",
      label: t("platform.sourceFilter.plugin", {
        defaultValue: i18n.language.startsWith("zh") ? "插件来源" : "Plugin source",
      }),
      count: sourceCounts.plugin,
    },
  ];
  const activeSourceLabel = sourceTabs.find((tab) => tab.id === sourceFilter)?.label ?? sourceTabs[0].label;
  const sortFieldOptions: Array<{ value: PlatformSortField; label: string }> = [
    { value: "repository", label: t("platform.sortFields.repository") },
    { value: "name", label: t("platform.sortFields.name") },
    { value: "installedAt", label: t("platform.sortFields.installedAt") },
    { value: "updatedAt", label: t("platform.sortFields.updatedAt") },
  ];
  const sortDirectionOptions: Array<{ value: PlatformSortDirection; label: string }> = [
    { value: "asc", label: t("platform.sortDirections.asc") },
    { value: "desc", label: t("platform.sortDirections.desc") },
  ];
  const groupByOptions: Array<{ value: PlatformGroupBy; label: string }> = [
    { value: "none", label: t("platform.groupBy.none") },
    { value: "repository", label: t("platform.groupBy.repository") },
  ];
  const showTransferActions = isCodingPlatformPage;

  const renderSkillCard = (skill: ScannedSkill, className?: string) => (
    <UnifiedSkillCard
      key={getPlatformSkillRowKey(skill)}
      name={skill.name}
      description={skill.description}
      aiSummary={getAiSummary(skill)}
      sourceType={skill.link_type as "symlink" | "copy" | "native"}
      originKind={skill.source_kind ?? null}
      isReadOnly={skill.is_read_only ?? false}
      publisher={skill.repository?.name}
      usageBadge={usageCounts?.[skill.name]}
      checkbox={
        !skill.is_read_only
          ? {
              checked: selectedSkillKeys.has(getPlatformSkillRowKey(skill)),
              onChange: () => toggleSelectedSkill(skill),
            }
          : undefined
      }
      isLoading={
        resolvedAgentId
          ? (pendingSkillActionKeys[getPlatformSkillActionKey(skill, resolvedAgentId)] ?? false)
          : false
      }
      onDetail={() => handleOpenDrawer(skill)}
      onInstallTo={
        skill.is_read_only
          ? undefined
          : () => void handleInstallClick(skill.id)
      }
      onUninstallFromPlatform={
        skill.is_read_only
          ? undefined
          : () => void handleUninstall(skill)
      }
      uninstallFromLabel={t("platform.uninstallFromLabel", {
        skill: skill.name,
        platform: platformDisplayName,
        defaultValue: i18n.language.startsWith("zh")
          ? `从 ${platformDisplayName} 卸载 ${skill.name}`
          : `Uninstall ${skill.name} from ${platformDisplayName}`,
      })}
      detailButtonRef={(node) => setDetailButtonRef(getPlatformSkillRowKey(skill), node)}
      className={className}
    />
  );

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b border-border px-6 py-4">
        <div className="flex items-center gap-2.5">
          <PlatformIcon agentId={agent.id} className="size-6 text-primary/70" size={24} />
          <h1 className="text-xl font-semibold">{platformDisplayName}</h1>
        </div>
        {isUniversalPage && (
          <p className="text-xs text-muted-foreground mt-1">
            {t("platformTargets.universalScope")}
          </p>
        )}
        <p className="text-sm text-muted-foreground mt-0.5">
          {formatPathForDisplay(agent.global_skills_dir)}
        </p>
      </div>

      {isClaudePage && (
        <div
          role="tablist"
          aria-label={t("platform.sourceFilterTabsLabel", {
            defaultValue: i18n.language.startsWith("zh") ? "Claude 来源筛选" : "Claude source filters",
          })}
          className="flex items-center gap-1 px-6 py-3 border-b border-border"
        >
          {sourceTabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={sourceFilter === tab.id}
              onClick={() => setSourceFilter(tab.id)}
              className={cn(
                "inline-flex items-center gap-1.5 px-4 py-1.5 rounded-md text-sm transition-colors cursor-pointer",
                sourceFilter === tab.id
                  ? "bg-primary/15 text-foreground font-medium"
                  : "text-muted-foreground hover:bg-muted/40"
              )}
            >
              <span>{tab.label}</span>
              <span className="text-xs opacity-75">({tab.count})</span>
            </button>
          ))}
        </div>
      )}

      {/* Search bar */}
      <div className="px-6 py-3 border-b border-border">
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground pointer-events-none" />
            <Input
              placeholder={t("platform.searchPlaceholder")}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 bg-muted/40"
            />
          </div>
          <PlatformSkillSortMenu
            t={t}
            sortField={sortField}
            sortDirection={sortDirection}
            sortFieldOptions={sortFieldOptions}
            sortDirectionOptions={sortDirectionOptions}
            onChange={(next) => {
              setSortField(next.field);
              setSortDirection(next.direction);
            }}
          />
          <PlatformSkillViewMenu
            t={t}
            groupBy={groupBy}
            groupByOptions={groupByOptions}
            onChangeGroupBy={setGroupBy}
          />
          <Button
            type="button"
            variant="outline"
            disabled={!resolvedAgentId || isDuplicateScanning || isLoading}
            onClick={() => void handleScanDuplicates()}
            aria-label={t("platform.scanDuplicatesLabel", { platform: platformDisplayName })}
          >
            {isDuplicateScanning ? t("platform.scanningDuplicates") : t("platform.scanDuplicates")}
          </Button>
        </div>
      </div>

      <PlatformTransferRail
        selectedCount={selectedSkillKeys.size}
        selectableCount={selectableFilteredSkills.length}
        onSelectCurrentResults={selectCurrentResults}
        onSelectMostUniversal={
          showTransferActions
            ? () => {
                void handleSelectMostUniversal();
              }
            : undefined
        }
        onClearSelection={handleClearTransferSelection}
      />

      {/* Content */}
      <div ref={contentRef} className="flex-1 overflow-auto p-6">
        {isLoading ? (
          <EmptyState message={t("platform.loading")} />
        ) : skills.length === 0 ? (
          <EmptyState
            message={t("platform.noSkills", { name: platformDisplayName })}
          />
        ) : sourceFilteredSkills.length === 0 ? (
          <EmptyState
            message={t("platform.noSourceSkills", {
              name: platformDisplayName,
              source: activeSourceLabel,
              defaultValue: i18n.language.startsWith("zh")
                ? `${platformDisplayName} 下暂无${activeSourceLabel}技能`
                : `No ${activeSourceLabel} skills installed for ${platformDisplayName}`,
            })}
          />
        ) : filteredSkills.length === 0 ? (
          <EmptyState
            message={t("platform.noMatch", { query: searchQuery })}
          />
        ) : groupBy === "repository" ? (
          <PlatformGroupedSkillList
            groups={platformRows.groups}
            renderSkillCard={(skill) => renderSkillCard(skill)}
          />
        ) : filteredSkills.length > 40 ? (
          <VirtualizedGrid
            items={filteredSkills}
            itemHeight={196}
            rowGap={16}
            columnGap={16}
            overscanRows={3}
            minColumnWidth={420}
            maxColumns={2}
            scrollContainerRef={contentRef}
            itemKey={(skill) => getPlatformSkillRowKey(skill)}
            renderItem={(skill) => renderSkillCard(skill)}
          />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {filteredSkills.map((skill) => renderSkillCard(skill))}
          </div>
        )}
      </div>

      <PlatformTransferActionBar
        selectedCount={selectedSkillKeys.size}
        isInstalling={isBatchInstalling}
        isDeleting={isBatchDeleting}
        onInstall={
          showTransferActions ? () => setIsBatchInstallDialogOpen(true) : undefined
        }
        onDelete={() => setIsBatchDeleteDialogOpen(true)}
        onClearSelection={handleClearTransferSelection}
      />

      {/* Install Dialog */}
      <InstallDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
        skill={installTargetSkill}
        agents={installTargetAgents}
        onInstall={handleInstall}
      />

      <BatchInstallCentralSkillsDialog
        open={isBatchInstallDialogOpen}
        onOpenChange={setIsBatchInstallDialogOpen}
        skillCount={selectedBatchSkillIds.length}
        agents={installTargetAgents}
        isInstalling={isBatchInstalling}
        title={t("platform.transferDialogTitle")}
        description={t("platform.transferDialogDesc", {
          count: selectedBatchSkillIds.length,
          platform: platformDisplayName,
        })}
        defaultExcludedAgentIds={defaultBatchExcludedAgentIds}
        onInstall={handleBatchInstall}
      />

      <BatchDeletePlatformSkillsDialog
        open={isBatchDeleteDialogOpen}
        onOpenChange={setIsBatchDeleteDialogOpen}
        platformName={platformDisplayName}
        skills={selectedPlatformSkills}
        isDeleting={isBatchDeleting}
        onConfirm={handleBatchDelete}
      />

      <SkillDetailDrawer
        open={isDrawerOpen}
        skillId={drawerSkill?.id ?? null}
        agentId={resolvedAgentId ?? null}
        rowId={drawerSkill?.row_id ?? null}
        onOpenChange={(open) => {
          setIsDrawerOpen(open);
          if (!open) {
            setDrawerSkill(null);
          }
        }}
        returnFocusRef={
          returnFocusRowKey
            ? {
                current: detailButtonRefs.current[returnFocusRowKey] ?? null,
              }
            : undefined
        }
        onInstallClick={(skillId) => {
          void handleInstallClick(skillId);
        }}
      />
    </div>
  );
}
