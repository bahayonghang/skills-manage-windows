import { useEffect, useState } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import {
  Loader2,
  Blocks,
  FolderOpen,
  LayoutDashboard,
  Layers,
  ScrollText,
  Settings,
  Store,
  ChevronLeft,
  ChevronRight,
  BarChart3,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { usePlatformStore } from "@/stores/platformStore";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
} from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";
import { cn } from "@/lib/utils";
import { markAppPerformance } from "@/lib/performance";
import { useResizableWidth } from "@/hooks/useResizableWidth";

const EXPANDED_SIDEBAR_WIDTH = 208;
const COLLAPSED_SIDEBAR_WIDTH = 56;
const MIN_SIDEBAR_WIDTH = 168;
const MAX_SIDEBAR_WIDTH = 360;

// ─── Nav Item ────────────────────────────────────────────────────────────────

function NavItem({
  label,
  isActive,
  onClick,
  icon,
  expanded,
  count,
}: {
  label: string;
  isActive: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  expanded: boolean;
  count?: number;
}) {
  return (
    <div className="relative">
      <button
        onClick={onClick}
        title={label}
        aria-label={label}
        className={cn(
          "flex items-center w-full rounded-md transition-colors active:scale-[0.98] cursor-pointer",
          !isActive && "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
          isActive &&
            "bg-sidebar-primary text-sidebar-primary-foreground shadow-[inset_0_1px_0_color-mix(in_oklch,white_22%,transparent),0_10px_28px_color-mix(in_oklch,var(--sidebar-primary)_22%,transparent)] bg-[linear-gradient(145deg,color-mix(in_oklch,var(--sidebar-primary)_94%,white_12%),color-mix(in_oklch,var(--sidebar-primary)_82%,black_14%))]",
          expanded ? "gap-2.5 px-2.5 py-1.5 text-sm" : "justify-center py-2 px-1.5"
        )}
      >
        <span className="shrink-0">{icon}</span>
        {expanded && (
          <>
            <span className="truncate flex-1 text-left">{label}</span>
            {count !== undefined && count > 0 && (
              <span className={cn(
                "text-[10px] font-mono tabular-nums px-1.5 py-0.5 rounded-full shrink-0",
                isActive
                  ? "bg-sidebar-primary-foreground/15 text-sidebar-primary-foreground"
                  : "bg-muted/60 text-muted-foreground"
              )}>
                {count}
              </span>
            )}
          </>
        )}
      </button>
      {isActive && (
        <span
          className="absolute left-0 top-1.5 bottom-1.5 w-0.5 rounded-r bg-sidebar-primary-foreground/80"
          aria-hidden="true"
        />
      )}
    </div>
  );
}

// ─── Sidebar ────────────────────────────────────────────────────────────────

export function Sidebar() {
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const { t } = useTranslation();
  const {
    agents,
    skillsByAgent,
    collectionCount,
    isLoading,
    isRefreshing,
    scanState,
  } = usePlatformStore();
  const categoryVisibility = usePlatformStore((s) => s.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;

  const [expanded, setExpanded] = useState(() =>
    typeof window === "undefined" ? true : window.innerWidth >= 768
  );
  const {
    width: sidebarWidth,
    startResize,
    handleResizeKeyDown,
  } = useResizableWidth({
    defaultWidth: EXPANDED_SIDEBAR_WIDTH,
    minWidth: MIN_SIDEBAR_WIDTH,
    maxWidth: MAX_SIDEBAR_WIDTH,
  });
  const resolvedSidebarWidth = expanded ? sidebarWidth : COLLAPSED_SIDEBAR_WIDTH;

  useEffect(() => {
    if (!isLoading && agents.length > 0) {
      markAppPerformance("sidebar_ready");
    }
  }, [agents.length, isLoading]);

  const platformAgents = getPlatformTargetGroups(agents, categoryVisibility);
  const lobsterAgents = platformAgents.filter((a) => a.category === "lobster");
  const codingAgents = platformAgents.filter((a) => a.category !== "lobster");

  const isDashboardActive = pathname === "/dashboard" || pathname === "/";
  const isCollectionActive = pathname === "/collections";
  const isLogsActive = pathname === "/logs";
  const isSettingsActive = pathname.startsWith("/settings");

  function platformRoute(agent: (typeof platformAgents)[number]) {
    return `/platform/${agent.id}`;
  }

  function platformCount(agent: (typeof platformAgents)[number]) {
    if (!isUniversalPlatformTarget(agent)) {
      return skillsByAgent[agent.id];
    }

    const primaryCount = skillsByAgent[agent.install_agent_id];
    if (primaryCount !== undefined) {
      return primaryCount;
    }

    const memberCounts = agent.member_agents
      .map((member) => skillsByAgent[member.id])
      .filter((count): count is number => count !== undefined);

    return memberCounts.length > 0 ? Math.max(...memberCounts) : undefined;
  }

  function handleCollectionClick() {
    navigate("/collections");
  }

  return (
    <nav
      className={cn(
        "relative flex flex-col shrink-0 h-full border-r border-border bg-sidebar text-sidebar-foreground"
      )}
      style={{ width: resolvedSidebarWidth }}
      aria-label={t("sidebar.mainNav")}
    >
      {/* Toggle button */}
      <div
        className={cn(
          "flex items-center border-b border-border",
          expanded ? "justify-between px-3 py-2" : "justify-center py-2"
        )}
      >
        {expanded && (
          <span className="text-sm font-bold tracking-tight text-sidebar-primary">
            {t("app.name")}
          </span>
        )}
        <button
          onClick={() => setExpanded((e) => !e)}
          className={cn(
            "p-1 rounded-md transition-colors cursor-pointer",
            "text-muted-foreground hover:text-foreground hover:bg-muted/60"
          )}
          aria-label={expanded ? t("sidebar.collapseSidebar") : t("sidebar.expandSidebar")}
          title={expanded ? t("sidebar.collapseSidebar") : t("sidebar.expandSidebar")}
        >
          {expanded ? (
            <ChevronLeft className="size-4" />
          ) : (
            <ChevronRight className="size-4" />
          )}
        </button>
      </div>

      {/* Scrollable nav items */}
      <div className="flex-1 overflow-y-auto py-2 px-1.5 space-y-0.5">
        {/* Dashboard */}
        <NavItem
          label={t("sidebar.dashboard")}
          isActive={isDashboardActive}
          onClick={() => navigate("/dashboard")}
          icon={<LayoutDashboard className="size-4" />}
          expanded={expanded}
        />

        {/* Central Skills */}
        <NavItem
          label={t("sidebar.centralSkills")}
          isActive={pathname === "/central"}
          onClick={() => navigate("/central")}
          icon={<Blocks className="size-4" />}
          expanded={expanded}
          count={skillsByAgent["central"]}
        />

          {/* Projects (project-level skill management) */}
          <NavItem
            label={t("sidebar.projects")}
            isActive={pathname.startsWith("/projects")}
            onClick={() => navigate("/projects")}
            icon={<FolderOpen className="size-4" />}
            expanded={expanded}
          />

          <NavItem
            label={t("sidebar.obsidian")}
            isActive={pathname.startsWith("/obsidian")}
            onClick={() => navigate("/obsidian")}
            icon={<Blocks className="size-4" />}
            expanded={expanded}
          />

          {/* Marketplace */}
        <NavItem
          label={t("marketplace.title")}
          isActive={pathname === "/marketplace"}
          onClick={() => navigate("/marketplace")}
          icon={<Store className="size-4" />}
          expanded={expanded}
        />

        {/* Collections */}
        <NavItem
          label={t("sidebar.collections")}
          isActive={isCollectionActive}
          onClick={handleCollectionClick}
          icon={<Layers className="size-4" />}
          expanded={expanded}
          count={collectionCount}
        />

        {/* Operation Logs */}
        <NavItem
          label={t("logs.title")}
          isActive={isLogsActive}
          onClick={() => navigate("/logs")}
          icon={<ScrollText className="size-4" />}
          expanded={expanded}
        />

        {/* Skill Usage — skilled-style aggregated call stats across AI tools */}
        <NavItem
          label={t("sidebar.skillUsage")}
          isActive={pathname === "/usage"}
          onClick={() => navigate("/usage")}
          icon={<BarChart3 className="size-4" />}
          expanded={expanded}
        />

        {/* Divider */}
        <div className="border-t border-sidebar-border/70 my-2" />

        {/* Platform icons */}
        {isLoading ? (
          <div className={cn(
            "flex items-center py-2 text-muted-foreground text-sm",
            expanded ? "gap-2 px-2.5" : "justify-center"
          )}>
            <Loader2 className="size-4 animate-spin shrink-0" />
            {expanded && <span>{t("sidebar.scanning")}</span>}
          </div>
        ) : (
          <>
            {/* Lobster agents */}
            {lobsterAgents.length > 0 && (
              <>
                {expanded ? (
                  <div className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider px-2.5 pt-2 pb-1">
                    {t("sidebar.categoryLobster")}
                  </div>
                ) : (
                  <div className="border-t border-sidebar-border/40 my-1.5" />
                )}
                {lobsterAgents.map((agent) => (
                  <NavItem
                    key={agent.id}
                    label={
                      isUniversalPlatformTarget(agent)
                        ? t("platformTargets.universalShortLabel")
                        : agent.display_name
                    }
                    isActive={pathname === platformRoute(agent)}
                    onClick={() => navigate(platformRoute(agent))}
                    icon={<PlatformIcon agentId={agent.id} className="size-4" />}
                    expanded={expanded}
                    count={platformCount(agent)}
                  />
                ))}
              </>
            )}

            {/* Coding agents */}
            {codingAgents.length > 0 && (
              <>
                {expanded ? (
                  <div className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider px-2.5 pt-2 pb-1">
                    {t("sidebar.categoryCoding")}
                  </div>
                ) : (
                  <div className="border-t border-sidebar-border/40 my-1.5" />
                )}
                {codingAgents.map((agent) => (
                  <NavItem
                    key={agent.id}
                    label={
                      isUniversalPlatformTarget(agent)
                        ? t("platformTargets.universalShortLabel")
                        : agent.display_name
                    }
                    isActive={pathname === platformRoute(agent)}
                    onClick={() => navigate(platformRoute(agent))}
                    icon={<PlatformIcon agentId={agent.id} className="size-4" />}
                    expanded={expanded}
                    count={platformCount(agent)}
                  />
                ))}
              </>
            )}
          </>
        )}

        {!isLoading && (isRefreshing || scanState === "refreshing") && (
          <div className={cn(
            "flex items-center py-2 text-muted-foreground text-sm",
            expanded ? "gap-2 px-2.5" : "justify-center"
          )}>
            <Loader2 className="size-4 animate-spin shrink-0" />
            {expanded && <span>{t("sidebar.scanning")}</span>}
          </div>
        )}
      </div>

      <div className="border-t border-sidebar-border/70 px-1.5 py-2">
        <NavItem
          label={t("sidebar.settings")}
          isActive={isSettingsActive}
          onClick={() => navigate("/settings")}
          icon={<Settings className="size-4" />}
          expanded={expanded}
        />
      </div>

      {expanded && (
        <div
          role="separator"
          aria-label={t("sidebar.resizeSidebar")}
          aria-orientation="vertical"
          tabIndex={0}
          onPointerDown={startResize}
          onKeyDown={handleResizeKeyDown}
          className="absolute right-0 top-0 z-20 h-full w-1.5 translate-x-1/2 cursor-col-resize rounded-full bg-transparent transition-colors hover:bg-primary/30 focus:outline-none focus-visible:bg-primary/40"
        />
      )}
    </nav>
  );
}
