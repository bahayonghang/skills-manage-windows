import { useEffect, useMemo, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { Search, Blocks } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillStore } from "@/stores/skillStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { Input } from "@/components/ui/input";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { SkillDetailDrawer } from "@/components/skill/SkillDetailDrawer";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { InstallDialog } from "@/components/central/InstallDialog";
import { VirtualizedGrid } from "@/components/ui/virtualized-grid";
import { formatPathForDisplay } from "@/lib/path";
import { cn } from "@/lib/utils";
import { ScannedSkill, SkillWithLinks } from "@/types";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  filterVisiblePlatformAgents,
} from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";

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

  const skillsByAgent = useSkillStore((state) => state.skillsByAgent);
  const loadingByAgent = useSkillStore((state) => state.loadingByAgent);
  const pendingSkillActionKeys = useSkillStore((state) => state.pendingSkillActionKeys);
  const getSkillsByAgent = useSkillStore((state) => state.getSkillsByAgent);
  const uninstallSkillFromAgent = useSkillStore((state) => state.uninstallSkillFromAgent);

  const centralSkills = useCentralSkillsStore((state) => state.skills);
  const loadCentralSkills = useCentralSkillsStore((state) => state.loadCentralSkills);
  const installSkill = useCentralSkillsStore((state) => state.installSkill);
  const refreshCounts = usePlatformStore((state) => state.refreshCounts);

  const [searchQuery, setSearchQuery] = useState("");
  const [sourceFilter, setSourceFilter] = useState<ClaudeSourceFilter>("all");
  const [installTargetSkill, setInstallTargetSkill] = useState<SkillWithLinks | null>(null);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [drawerSkill, setDrawerSkill] = useState<ScannedSkill | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [returnFocusRowKey, setReturnFocusRowKey] = useState<string | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);
  const detailButtonRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  function getSkillRowKey(skill: ScannedSkill) {
    return skill.row_id ?? skill.id;
  }

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

  function getPendingSkillActionKey(skill: ScannedSkill) {
    return getClaudeUserRowId(skill) ?? (resolvedAgentId ? `${resolvedAgentId}::${skill.id}` : skill.id);
  }

  function getClaudeUserRowId(skill: ScannedSkill) {
    return resolvedAgentId === "claude-code" && skill.source_kind === "user" && !skill.is_read_only
      ? skill.row_id
      : undefined;
  }

  async function handleUninstall(skill: ScannedSkill) {
    if (!resolvedAgentId) return;
    const rowId = getClaudeUserRowId(skill);
    try {
      if (rowId) {
        await uninstallSkillFromAgent(skill.id, resolvedAgentId, rowId);
      } else {
        await uninstallSkillFromAgent(skill.id, resolvedAgentId);
      }
      await refreshCounts();
      await getSkillsByAgent(resolvedAgentId);
    } catch (err) {
      toast.error(t("detail.uninstallError", { error: String(err) }));
    }
  }
  const isLoading = resolvedAgentId ? (loadingByAgent[resolvedAgentId] ?? false) : false;

  // Memoize skills to avoid changing dependency reference on every render
  const skills = useMemo(
    () => (resolvedAgentId ? (skillsByAgent[resolvedAgentId] ?? []) : []),
    [resolvedAgentId, skillsByAgent]
  );

  const sourceFilteredSkills = useMemo(() => {
    if (!isClaudePage || sourceFilter === "all") {
      return skills;
    }
    return skills.filter((skill) => skill.source_kind === sourceFilter);
  }, [isClaudePage, skills, sourceFilter]);

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

  // Filter skills by search query using useMemo
  const filteredSkills = useMemo(() => {
    if (!searchQuery.trim()) return sourceFilteredSkills;
    const q = searchQuery.toLowerCase();
    return sourceFilteredSkills.filter(
      (skill) =>
        skill.id.toLowerCase().includes(q) ||
        skill.name.toLowerCase().includes(q) ||
        skill.description?.toLowerCase().includes(q)
    );
  }, [sourceFilteredSkills, searchQuery]);

  useEffect(() => {
    if (!drawerSkill) return;

    const rowKey = getSkillRowKey(drawerSkill);
    const refreshedSkill = skills.find((skill) => getSkillRowKey(skill) === rowKey);

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
    setReturnFocusRowKey(getSkillRowKey(skill));
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
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground pointer-events-none" />
          <Input
            placeholder={t("platform.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8 bg-muted/40"
          />
        </div>
      </div>

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
        ) : filteredSkills.length > 40 ? (
          <VirtualizedGrid
            items={filteredSkills}
            itemHeight={132}
            rowGap={16}
            columnGap={16}
            overscanRows={3}
            minColumnWidth={420}
            maxColumns={2}
            scrollContainerRef={contentRef}
            itemKey={(skill) => getSkillRowKey(skill)}
            renderItem={(skill) => (
              <UnifiedSkillCard
                key={getSkillRowKey(skill)}
                name={skill.name}
                description={skill.description}
                sourceType={skill.link_type as "symlink" | "copy" | "native"}
                originKind={skill.source_kind ?? null}
                isReadOnly={skill.is_read_only ?? false}
                isLoading={
                  resolvedAgentId
                    ? (pendingSkillActionKeys[getPendingSkillActionKey(skill)] ?? false)
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
                detailButtonRef={(node) => setDetailButtonRef(getSkillRowKey(skill), node)}
                className="h-[132px]"
              />
            )}
          />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {filteredSkills.map((skill) => (
              <UnifiedSkillCard
                key={getSkillRowKey(skill)}
                name={skill.name}
                description={skill.description}
                sourceType={skill.link_type as "symlink" | "copy" | "native"}
                originKind={skill.source_kind ?? null}
                isReadOnly={skill.is_read_only ?? false}
                isLoading={
                  resolvedAgentId
                    ? (pendingSkillActionKeys[getPendingSkillActionKey(skill)] ?? false)
                    : false
                }
                onDetail={() => handleOpenDrawer(skill)}
                onInstallTo={
                  skill.is_read_only
                    ? undefined
                    : () => handleInstallClick(skill.id)
                }
                onUninstallFromPlatform={
                  skill.is_read_only
                    ? undefined
                    : () => handleUninstall(skill)
                }
                uninstallFromLabel={t("platform.uninstallFromLabel", {
                  skill: skill.name,
                  platform: platformDisplayName,
                  defaultValue: i18n.language.startsWith("zh")
                    ? `从 ${platformDisplayName} 卸载 ${skill.name}`
                    : `Uninstall ${skill.name} from ${platformDisplayName}`,
                })}
                detailButtonRef={(node) => setDetailButtonRef(getSkillRowKey(skill), node)}
              />
            ))}
          </div>
        )}
      </div>

      {/* Install Dialog */}
      <InstallDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
        skill={installTargetSkill}
        agents={installTargetAgents}
        onInstall={handleInstall}
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
      />
    </div>
  );
}
