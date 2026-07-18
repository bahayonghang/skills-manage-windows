import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { Blocks, Keyboard, Moon, Plus, Search, Sun } from "lucide-react";

import { usePlatformStore } from "@/stores/platformStore";
import { useThemeStore, type ThemeFlavor } from "@/stores/themeStore";
import { TargetQuickSwitcher } from "@/components/layout/TargetQuickSwitcher";
import { Button } from "@/components/ui/button";
import { ShortcutHint } from "@/components/ui/shortcut-hint";
import { cn } from "@/lib/utils";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";
import {
  getPlatformTargetCountAgentId,
  getPlatformTargetGroups,
  getPlatformTargetLabel,
} from "@/lib/platformTargetGroups";

interface TopBarProps {
  onSearchClick: () => void;
  onShortcutsClick: () => void;
}

const LIGHT_FLAVORS: ThemeFlavor[] = ["latte", "claude-light"];
const FLAVOR_TOGGLE_MAP: Record<ThemeFlavor, ThemeFlavor> = {
  mocha: "latte",
  macchiato: "latte",
  frappe: "latte",
  "claude-dark": "claude-light",
  latte: "mocha",
  "claude-light": "claude-dark",
};

export function TopBar({ onSearchClick, onShortcutsClick }: TopBarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { pathname } = useLocation();
  const flavor = useThemeStore((s) => s.flavor);
  const setFlavor = useThemeStore((s) => s.setFlavor);

  const agents = usePlatformStore((s) => s.agents);
  const skillsByAgent = usePlatformStore((s) => s.skillsByAgent);
  const categoryVisibility =
    usePlatformStore((s) => s.categoryVisibility) ??
    DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const platformTargets = getPlatformTargetGroups(agents, categoryVisibility);

  const isLightFlavor = LIGHT_FLAVORS.includes(flavor);

  // Determine current view label and count
  const viewInfo = (() => {
    if (pathname === "/dashboard" || pathname === "/") {
      return { label: t("sidebar.dashboard"), count: undefined };
    }
    if (pathname === "/central") {
      const count = skillsByAgent["central"] ?? 0;
      return { label: t("sidebar.centralSkills"), count };
    }
    if (pathname.startsWith("/platform/")) {
      const agentId = pathname.split("/platform/")[1];
      const agent =
        platformTargets.find((a) => a.id === agentId) ??
        agents.find((a) => a.id === agentId);
      const countAgentId = agent
        ? getPlatformTargetCountAgentId(agent)
        : agentId;
      return {
        label: agent ? getPlatformTargetLabel(agent, t, "short") : agentId,
        count: skillsByAgent[countAgentId] ?? 0,
      };
    }
    if (pathname === "/marketplace") {
      return { label: t("marketplace.title"), count: undefined };
    }
    if (pathname === "/collections") {
      return { label: t("sidebar.collections"), count: undefined };
    }
    if (pathname === "/logs") {
      return { label: t("logs.title"), count: undefined };
    }
    if (pathname.startsWith("/settings")) {
      return { label: t("sidebar.settings"), count: undefined };
    }
    if (pathname.startsWith("/skill/")) {
      return { label: t("globalSearch.skillDetail"), count: undefined };
    }
    return { label: "", count: undefined };
  })();

  function handleToggleTheme() {
    setFlavor(FLAVOR_TOGGLE_MAP[flavor] ?? "latte");
  }

  return (
    <header className="relative flex items-center h-12 px-4 border-b border-border bg-sidebar text-sidebar-foreground shrink-0">
      {/* App icon */}
      <button
        onClick={() => navigate("/dashboard")}
        className="z-10 p-1.5 rounded-md transition-colors cursor-pointer text-sidebar-primary hover:bg-muted/60 shrink-0"
        aria-label={t("app.name")}
        title={t("app.name")}
      >
        <Blocks className="size-4" />
      </button>

      <div className="hidden flex-1 lg:block" />

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
            <ShortcutHint
              shortcut="mod+k"
              className="hidden rounded border border-border/50 px-1 py-0.5 text-ui-meta text-muted-foreground sm:inline-flex"
            />
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
          <span className="hidden truncate text-sm text-muted-foreground sm:inline">
            {viewInfo.label}
          </span>
        )}
      </div>

      <Button
        size="sm"
        className="ml-2 hidden lg:inline-flex"
        onClick={() => navigate("/marketplace")}
        title={t("topbar.importSkill")}
      >
        <Plus className="size-3.5" />
        <span>{t("topbar.importSkill")}</span>
      </Button>

      <TargetQuickSwitcher />

      <button
        onClick={onShortcutsClick}
        className={cn(
          "z-10 p-1.5 rounded-md transition-colors cursor-pointer shrink-0 focus-ring",
          "text-muted-foreground hover:text-foreground hover:bg-muted/60",
        )}
        aria-label={t("shortcuts.openButton")}
        title={t("shortcuts.openButton")}
      >
        <Keyboard className="size-4" />
      </button>

      <button
        onClick={handleToggleTheme}
        className={cn(
          "z-10 p-1.5 rounded-md transition-colors cursor-pointer shrink-0",
          "text-muted-foreground hover:text-foreground hover:bg-muted/60",
        )}
        aria-label={
          isLightFlavor
            ? t("topbar.themeToggleDark")
            : t("topbar.themeToggleLight")
        }
        title={
          isLightFlavor
            ? t("topbar.themeToggleDark")
            : t("topbar.themeToggleLight")
        }
      >
        {isLightFlavor ? (
          <Moon className="size-4" />
        ) : (
          <Sun className="size-4" />
        )}
      </button>
    </header>
  );
}
