import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { Blocks, Search, Server, Settings } from "lucide-react";

import { usePlatformStore } from "@/stores/platformStore";
import { useDiscoverStore } from "@/stores/discoverStore";
import { useTargetStore } from "@/stores/targetStore";
import { cn } from "@/lib/utils";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
} from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";

interface TopBarProps {
  onSearchClick: () => void;
}

export function TopBar({ onSearchClick }: TopBarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { pathname } = useLocation();

  const agents = usePlatformStore((s) => s.agents);
  const skillsByAgent = usePlatformStore((s) => s.skillsByAgent);
  const categoryVisibility =
    usePlatformStore((s) => s.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const totalDiscovered = usePlatformStore((s) => s.discoveredCount);
  const isScanning = useDiscoverStore((s) => s.isScanning);
  const activeTarget = useTargetStore((s) => s.activeTarget);
  const platformTargets = getPlatformTargetGroups(agents, categoryVisibility);

  // Determine current view label and count
  const viewInfo = (() => {
    if (pathname === "/central" || pathname === "/") {
      const count = skillsByAgent["central"] ?? 0;
      return { label: t("sidebar.centralSkills"), count };
    }
    if (pathname.startsWith("/platform/")) {
      const agentId = pathname.split("/platform/")[1];
      const agent =
        platformTargets.find((a) => a.id === agentId) ??
        agents.find((a) => a.id === agentId);
      const countAgentId =
        agent && isUniversalPlatformTarget(agent) ? agent.install_agent_id : agentId;
      return {
        label:
          agent && isUniversalPlatformTarget(agent)
            ? t("platformTargets.universalShortLabel")
            : agent?.display_name ?? agentId,
        count: skillsByAgent[countAgentId] ?? 0,
      };
    }
    if (pathname.startsWith("/discover")) {
      return { label: t("sidebar.discovered"), count: totalDiscovered };
    }
    if (pathname === "/marketplace") {
      return { label: t("marketplace.title"), count: undefined };
    }
    if (pathname === "/collections") {
      return { label: t("sidebar.collections"), count: undefined };
    }
    if (pathname === "/settings") {
      return { label: t("sidebar.settings"), count: undefined };
    }
    if (pathname.startsWith("/skill/")) {
      return { label: t("globalSearch.skillDetail"), count: undefined };
    }
    return { label: "", count: undefined };
  })();

  const isMac =
    typeof navigator !== "undefined" &&
    navigator.platform.toUpperCase().includes("MAC");

  return (
    <header className="relative flex items-center h-12 px-4 border-b border-border bg-sidebar text-sidebar-foreground shrink-0">
      {/* App icon */}
      <button
        onClick={() => navigate("/central")}
        className="z-10 p-1.5 rounded-md transition-colors cursor-pointer text-sidebar-primary hover:bg-muted/60 shrink-0"
        aria-label={t("app.name")}
        title={t("app.name")}
      >
        <Blocks className="size-4" />
      </button>

      <div className="flex-1" />

      <div className="pointer-events-none absolute inset-0 hidden items-center justify-center px-16 lg:flex">
        <div className="pointer-events-auto flex items-center gap-3 max-w-[min(56rem,calc(100vw-14rem))]">
          <button
            onClick={onSearchClick}
            className={cn(
              "flex items-center gap-2 h-8 w-[min(26rem,40vw)] min-w-[14rem] px-3 rounded-md text-sm",
              "bg-muted/40 text-muted-foreground border border-border/50",
              "hover:bg-muted/60 hover:border-border transition-colors cursor-pointer",
            )}
          >
            <Search className="size-3.5 shrink-0" />
            <span className="truncate flex-1 text-left">
              {t("globalSearch.trigger")}
            </span>
            <kbd className="hidden sm:inline-flex items-center gap-0.5 text-[10px] font-mono text-muted-foreground/60 border border-border/50 rounded px-1 py-0.5">
              {isMac ? "⌘" : "Ctrl"}K
            </kbd>
          </button>
        </div>
      </div>

      <div className="ml-3 flex min-w-0 flex-1 items-center gap-2 lg:hidden">
        <button
          onClick={onSearchClick}
          className={cn(
            "flex min-w-0 flex-1 items-center gap-2 h-8 px-3 rounded-md text-sm",
            "bg-muted/40 text-muted-foreground border border-border/50",
            "hover:bg-muted/60 hover:border-border transition-colors cursor-pointer",
          )}
        >
          <Search className="size-3.5 shrink-0" />
          <span className="truncate flex-1 text-left">
            {t("globalSearch.trigger")}
          </span>
        </button>
        {viewInfo.label && (
          <span className="truncate text-sm text-muted-foreground">
            {viewInfo.label}
          </span>
        )}
      </div>

      {/* Scan indicator */}
      {isScanning && (
        <div className="mr-2 flex items-center gap-1.5 text-xs text-primary shrink-0">
          <span className="relative flex size-2">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75" />
            <span className="relative inline-flex rounded-full size-2 bg-primary" />
          </span>
          <span className="text-primary/70">{t("discover.scanning")}</span>
        </div>
      )}

      <button
        onClick={() => navigate("/settings")}
        className={cn(
          "z-10 mr-2 hidden max-w-44 items-center gap-1.5 rounded-md px-2 py-1.5 text-xs transition-colors sm:flex",
          activeTarget.kind === "ssh"
            ? "bg-primary/10 text-primary hover:bg-primary/15"
            : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
        )}
        title={
          activeTarget.kind === "ssh"
            ? `${activeTarget.username ?? ""}@${activeTarget.host ?? ""}`
            : t("targets.local")
        }
      >
        <Server className="size-3.5 shrink-0" />
        <span className="truncate">
          {activeTarget.kind === "ssh" ? activeTarget.label : t("targets.local")}
        </span>
      </button>

      {/* Settings */}
      <button
        onClick={() => navigate("/settings")}
        className={cn(
          "z-10 p-1.5 rounded-md transition-colors cursor-pointer shrink-0",
          "text-muted-foreground hover:text-foreground hover:bg-muted/60",
          pathname === "/settings" && "bg-muted/60 text-foreground",
        )}
        aria-label={t("sidebar.settings")}
        title={t("sidebar.settings")}
      >
        <Settings className="size-4" />
      </button>
    </header>
  );
}
