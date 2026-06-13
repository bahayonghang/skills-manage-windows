import { useTranslation } from "react-i18next";
import { Loader2, RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import type { UpdateCenterTab } from "@/stores/updateCenterStore";
import type {
  SkillRefreshMode,
  SkillRefreshScopeKind,
} from "@/types/skillUpdateInventory";

interface UpdateCenterToolbarProps {
  scopeKind: SkillRefreshScopeKind;
  onScopeKindChange: (next: SkillRefreshScopeKind) => void;
  refreshMode: SkillRefreshMode;
  onRefreshModeChange: (next: SkillRefreshMode) => void;
  isRefreshing: boolean;
  onRefresh: () => void;
  lastRefreshedAt: string | null;
  activeTab: UpdateCenterTab;
  onTabChange: (tab: UpdateCenterTab) => void;
  tabOrder: readonly UpdateCenterTab[];
  counts: Record<UpdateCenterTab, number>;
  scopeEnabled: Record<SkillRefreshScopeKind, boolean>;
}

export function UpdateCenterToolbar({
  scopeKind,
  onScopeKindChange,
  refreshMode,
  onRefreshModeChange,
  isRefreshing,
  onRefresh,
  lastRefreshedAt,
  activeTab,
  onTabChange,
  tabOrder,
  counts,
  scopeEnabled,
}: UpdateCenterToolbarProps) {
  const { t } = useTranslation();
  return (
    <>
      <div className="flex flex-wrap items-center gap-2">
        <label className="sr-only" htmlFor="update-center-scope">
          {t("central.updateCenter.scopeLabel")}
        </label>
        <select
          id="update-center-scope"
          className="h-7 rounded-lg border border-border bg-background px-2 text-sm"
          value={scopeKind}
          onChange={(event) =>
            onScopeKindChange(event.target.value as SkillRefreshScopeKind)
          }
          disabled={isRefreshing}
        >
          <option value="all" disabled={!scopeEnabled.all}>
            {t("central.updateCenter.scopeAll")}
          </option>
          <option value="repositories" disabled={!scopeEnabled.repositories}>
            {t("central.updateCenter.scopeRepository")}
          </option>
          <option value="platform" disabled={!scopeEnabled.platform}>
            {t("central.updateCenter.scopePlatform")}
          </option>
          <option value="skills" disabled={!scopeEnabled.skills}>
            {t("central.updateCenter.scopeCurrent")}
          </option>
        </select>
        <label className="sr-only" htmlFor="update-center-refresh-mode">
          {t("central.updateCenter.modeLabel")}
        </label>
        <select
          id="update-center-refresh-mode"
          className="h-7 rounded-lg border border-border bg-background px-2 text-sm"
          value={refreshMode}
          onChange={(event) =>
            onRefreshModeChange(event.target.value as SkillRefreshMode)
          }
          disabled={isRefreshing}
        >
          <option value="regular">
            {t("central.updateCenter.modeRegular")}
          </option>
          <option value="sync">
            {t("central.updateCenter.modeSync")}
          </option>
        </select>
        <Button size="sm" onClick={onRefresh} disabled={isRefreshing}>
          {isRefreshing ? (
            <>
              <Loader2 className="size-3.5 animate-spin" />
              {t("central.updateCenter.refreshing")}
            </>
          ) : (
            <>
              <RefreshCw className="size-3.5" />
              {t("central.updateCenter.refreshButton")}
            </>
          )}
        </Button>
        {lastRefreshedAt ? (
          <span className="text-xs text-muted-foreground">
            {t("central.updateCenter.lastRefreshedAt", {
              time: new Date(lastRefreshedAt).toLocaleString(),
            })}
          </span>
        ) : null}
      </div>

      <div role="tablist" className="flex flex-wrap gap-1 border-b border-border">
        {tabOrder.map((tab) => (
          <button
            key={tab}
            type="button"
            role="tab"
            aria-selected={activeTab === tab}
            onClick={() => onTabChange(tab)}
            className={
              activeTab === tab
                ? "rounded-t-lg bg-primary/10 px-3 py-2 text-sm font-medium text-primary"
                : "rounded-t-lg px-3 py-2 text-sm text-muted-foreground hover:bg-muted"
            }
          >
            {t(`central.updateCenter.tabs.${tab}`, { count: counts[tab] })}
          </button>
        ))}
      </div>
    </>
  );
}
