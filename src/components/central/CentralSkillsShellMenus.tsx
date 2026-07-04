import {
  ActivityIcon,
  ArrowUpDown,
  Check,
  FileJson,
  LayoutGrid,
  List,
  MoreHorizontal,
  Rows3,
  Rows4,
  SlidersHorizontal,
} from "lucide-react";
import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";

export function ToolbarMoreMenu({
  t,
  activeTaskCount,
  onOpenTaskCenter,
  onOpenPlatformManage,
  onOpenPortability,
}: {
  t: TFunction;
  activeTaskCount: number;
  onOpenTaskCenter: () => void;
  onOpenPlatformManage: () => void;
  onOpenPortability: () => void;
}) {
  return (
    <MenuPrimitive.Root>
      <MenuPrimitive.Trigger
        render={
          <Button
            variant="outline"
            size="icon"
            className="h-9 w-9 rounded-xl transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
            aria-label={t("central.toolbarMoreAriaLabel")}
            data-testid="central-toolbar-more"
          >
            <MoreHorizontal className="size-4" />
          </Button>
        }
      />
      <MenuPrimitive.Portal>
        <MenuPrimitive.Positioner align="end" sideOffset={6} className="z-50 outline-none">
          <MenuPrimitive.Popup data-testid="central-toolbar-more-menu" className={menuPopupClassName()}>
            <MenuPrimitive.Item
              data-testid="central-toolbar-task-center"
              onClick={onOpenTaskCenter}
              className={menuItemClassName()}
            >
              <ActivityIcon className="size-3.5 shrink-0" />
              <span>{t("central.taskCenterMenuItem")}</span>
              {activeTaskCount > 0 && (
                <span
                  data-testid="central-toolbar-task-center-badge"
                  className="ml-auto inline-flex min-w-[18px] items-center justify-center rounded-full bg-primary/15 px-1.5 text-[10px] font-semibold text-primary ring-1 ring-primary/30"
                >
                  {activeTaskCount}
                </span>
              )}
            </MenuPrimitive.Item>
            <div role="separator" className="my-1 h-px bg-border/60" />
            <MenuPrimitive.Item
              onClick={onOpenPlatformManage}
              className={menuItemClassName()}
            >
              {t("central.platformManageButton")}
            </MenuPrimitive.Item>
            <MenuPrimitive.Item
              data-testid="central-portability-open"
              onClick={onOpenPortability}
              className={menuItemClassName()}
            >
              <FileJson className="size-3.5 shrink-0" />
              {t("central.portabilityOpen")}
            </MenuPrimitive.Item>
          </MenuPrimitive.Popup>
        </MenuPrimitive.Positioner>
      </MenuPrimitive.Portal>
    </MenuPrimitive.Root>
  );
}
import type { CentralInstalledSkillsQuickFilterProps } from "@/components/central/CentralInstalledSkillsQuickFilter";
import { cn } from "@/lib/utils";
import type { CentralViewState, GroupByMode } from "@/lib/centralViewState";
import {
  getInstalledFilterPlatformId,
  platformInstalledFilterValue,
} from "@/lib/centralInstalledFilters";
import {
  getPlatformTargetLabel,
  getPlatformTargetTitleHint,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";
import type {
  CentralSortDirection,
  CentralSortField,
} from "@/pages/centralSkillsViewModel";

type InstalledSkillsFilterProps = Omit<CentralInstalledSkillsQuickFilterProps, "t">;

// ─── 排序菜单 ──────────────────────────────────────────────────────

export function ToolbarSortMenu({
  t,
  sortField,
  sortDir,
  sortFieldOptions,
  sortDirectionOptions,
  onChange,
}: {
  t: TFunction;
  sortField: CentralSortField;
  sortDir: CentralSortDirection;
  sortFieldOptions: Array<{ value: CentralSortField; label: string }>;
  sortDirectionOptions: Array<{ value: CentralSortDirection; label: string }>;
  onChange: (next: { field: CentralSortField; dir: CentralSortDirection }) => void;
}) {
  const currentField = sortFieldOptions.find((o) => o.value === sortField);
  const currentDir = sortDirectionOptions.find((o) => o.value === sortDir);
  const currentFieldLabel = currentField?.label ?? sortField;
  const currentDirLabel = currentDir?.label ?? sortDir;
  return (
    <MenuPrimitive.Root>
      <MenuPrimitive.Trigger
        render={
          <Button
            variant="outline"
            size="sm"
            className="h-9 shrink-0 rounded-xl pl-3 pr-3.5 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
            data-testid="central-toolbar-sort"
            aria-label={t("central.toolbarSortAriaLabel")}
            title={t("central.toolbarSortCurrent", {
              field: currentFieldLabel,
              direction: currentDirLabel,
            })}
          >
            <ArrowUpDown className="size-3.5" />
            <span className="text-xs">{currentFieldLabel}</span>
            <span aria-hidden className="text-[10px] text-muted-foreground">
              {sortDir === "asc" ? "↑" : "↓"}
            </span>
          </Button>
        }
      />
      <MenuPrimitive.Portal>
        <MenuPrimitive.Positioner align="end" sideOffset={6} className="z-50 outline-none">
          <MenuPrimitive.Popup className={menuPopupClassName("min-w-[200px]")}>
            {sortFieldOptions.flatMap((field) =>
              sortDirectionOptions.map((dir) => {
                const active = field.value === sortField && dir.value === sortDir;
                return (
                  <MenuPrimitive.Item
                    key={`${field.value}-${dir.value}`}
                    data-testid={`central-toolbar-sort-${field.value}-${dir.value}`}
                    onClick={() => onChange({ field: field.value, dir: dir.value })}
                    className={menuItemClassName(active && "bg-accent/60 text-accent-foreground")}
                  >
                    <span>{field.label}</span>
                    <span className="ml-auto text-[10px] text-muted-foreground">{dir.label}</span>
                    {active && <Check className="size-3" aria-hidden />}
                  </MenuPrimitive.Item>
                );
              })
            )}
          </MenuPrimitive.Popup>
        </MenuPrimitive.Positioner>
      </MenuPrimitive.Portal>
    </MenuPrimitive.Root>
  );
}

// ─── 视图菜单 ──────────────────────────────────────────────────────

export function ToolbarViewMenu({
  t,
  viewState,
  setViewState,
  groupByOptions,
  installedSkillsFilter,
}: {
  t: TFunction;
  viewState: CentralViewState;
  setViewState: (next: CentralViewState) => void;
  groupByOptions?: Array<{ value: GroupByMode; label: string }>;
  installedSkillsFilter: InstalledSkillsFilterProps;
}) {
  const hasGroupOptions = (groupByOptions?.length ?? 0) > 1;
  const installedValue = installedSkillsFilter.value;
  const installedPlatformId = getInstalledFilterPlatformId(installedValue);
  const installedCount = installedSkillsFilter.installedCount;
  const uncategorizedActive = viewState.tags.includes("uncategorized");
  const hasActiveModifiers =
    viewState.group !== "none" ||
    installedValue !== "all" ||
    uncategorizedActive ||
    viewState.view !== "grid" ||
    viewState.density !== "comfortable";
  const platformOptions = (installedSkillsFilter.availableInstallAgents ?? []).filter(
    (agent) => agent.id !== "central"
  );

  const handleSelectGroup = (group: GroupByMode) => {
    setViewState({ ...viewState, group });
  };
  const handleSelectLayout = (view: "grid" | "list") => {
    setViewState({ ...viewState, view });
  };
  const handleSelectDensity = (density: "comfortable" | "compact") => {
    setViewState({ ...viewState, density });
  };
  const handleToggleUncategorized = () => {
    const next = uncategorizedActive
      ? viewState.tags.filter((id) => id !== "uncategorized")
      : [...viewState.tags, "uncategorized"];
    setViewState({ ...viewState, tags: next });
  };
  const handleInstalledAll = () => installedSkillsFilter.onChange("all");
  const handleInstalledAny = () => installedSkillsFilter.onChange("installed");
  const handleInstalledPlatform = (agentId: string) =>
    installedSkillsFilter.onChange(platformInstalledFilterValue(agentId));

  return (
    <MenuPrimitive.Root>
      <MenuPrimitive.Trigger
        render={
          <Button
            variant="outline"
            size="sm"
            className="h-9 shrink-0 rounded-xl pl-3 pr-3.5 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
            data-testid="central-toolbar-view"
            aria-label={t("central.toolbarViewAriaLabel")}
          >
            <SlidersHorizontal className="size-3.5" />
            <span className="text-xs">{t("central.toolbarView")}</span>
            {hasActiveModifiers && (
              <span
                data-testid="central-toolbar-view-dot"
                aria-label={t("central.toolbarViewBadgeActive")}
                className="size-1.5 rounded-full bg-primary"
              />
            )}
          </Button>
        }
      />
      <MenuPrimitive.Portal>
        <MenuPrimitive.Positioner align="end" sideOffset={6} className="z-50 outline-none">
          <MenuPrimitive.Popup className={menuPopupClassName("min-w-[240px]")}>
            <MenuPrimitive.Group>
              <MenuPrimitive.GroupLabel className={menuLabelClassName()}>
                {t("central.toolbarViewSectionLayout", {
                  defaultValue: "布局",
                })}
              </MenuPrimitive.GroupLabel>
              <MenuPrimitive.Item
                data-testid="central-toolbar-view-layout-grid"
                onClick={() => handleSelectLayout("grid")}
                className={menuItemClassName(
                  viewState.view === "grid" && "bg-accent/60 text-accent-foreground",
                )}
              >
                <LayoutGrid className="size-3.5 shrink-0" />
                <span>
                  {t("central.toolbarViewLayoutGrid", { defaultValue: "网格（双列）" })}
                </span>
                {viewState.view === "grid" && (
                  <Check className="ml-auto size-3" aria-hidden />
                )}
              </MenuPrimitive.Item>
              <MenuPrimitive.Item
                data-testid="central-toolbar-view-layout-list"
                onClick={() => handleSelectLayout("list")}
                className={menuItemClassName(
                  viewState.view === "list" && "bg-accent/60 text-accent-foreground",
                )}
              >
                <List className="size-3.5 shrink-0" />
                <span>
                  {t("central.toolbarViewLayoutList", { defaultValue: "列表（单列）" })}
                </span>
                {viewState.view === "list" && (
                  <Check className="ml-auto size-3" aria-hidden />
                )}
              </MenuPrimitive.Item>
            </MenuPrimitive.Group>

            <div role="separator" className="my-1 h-px bg-border/60" />

            <MenuPrimitive.Group>
              <MenuPrimitive.GroupLabel className={menuLabelClassName()}>
                {t("central.toolbarViewSectionDensity", {
                  defaultValue: "密度",
                })}
              </MenuPrimitive.GroupLabel>
              <MenuPrimitive.Item
                data-testid="central-toolbar-view-density-comfortable"
                onClick={() => handleSelectDensity("comfortable")}
                className={menuItemClassName(
                  viewState.density === "comfortable" &&
                    "bg-accent/60 text-accent-foreground",
                )}
              >
                <Rows3 className="size-3.5 shrink-0" />
                <span>
                  {t("central.toolbarViewDensityComfortable", {
                    defaultValue: "宽松",
                  })}
                </span>
                {viewState.density === "comfortable" && (
                  <Check className="ml-auto size-3" aria-hidden />
                )}
              </MenuPrimitive.Item>
              <MenuPrimitive.Item
                data-testid="central-toolbar-view-density-compact"
                onClick={() => handleSelectDensity("compact")}
                className={menuItemClassName(
                  viewState.density === "compact" &&
                    "bg-accent/60 text-accent-foreground",
                )}
              >
                <Rows4 className="size-3.5 shrink-0" />
                <span>
                  {t("central.toolbarViewDensityCompact", {
                    defaultValue: "紧凑",
                  })}
                </span>
                {viewState.density === "compact" && (
                  <Check className="ml-auto size-3" aria-hidden />
                )}
              </MenuPrimitive.Item>
            </MenuPrimitive.Group>

            <div role="separator" className="my-1 h-px bg-border/60" />

            {hasGroupOptions && groupByOptions && (
              <MenuPrimitive.Group>
                <MenuPrimitive.GroupLabel className={menuLabelClassName()}>
                  {t("central.toolbarViewSectionGroup")}
                </MenuPrimitive.GroupLabel>
                {groupByOptions.map((opt) => {
                  const active = viewState.group === opt.value;
                  return (
                    <MenuPrimitive.Item
                      key={opt.value}
                      data-testid={`central-toolbar-view-group-${opt.value}`}
                      onClick={() => handleSelectGroup(opt.value)}
                      className={menuItemClassName(active && "bg-accent/60 text-accent-foreground")}
                    >
                      <span>{opt.label}</span>
                      {active && <Check className="ml-auto size-3" aria-hidden />}
                    </MenuPrimitive.Item>
                  );
                })}
              </MenuPrimitive.Group>
            )}

            {hasGroupOptions && <div role="separator" className="my-1 h-px bg-border/60" />}

            <MenuPrimitive.Group>
              <MenuPrimitive.GroupLabel className={menuLabelClassName()}>
                {t("central.toolbarViewSectionInstalled")}
              </MenuPrimitive.GroupLabel>
              <MenuPrimitive.Item
                data-testid="central-toolbar-view-installed-all"
                onClick={handleInstalledAll}
                className={menuItemClassName(installedValue === "all" && "bg-accent/60 text-accent-foreground")}
              >
                <span>{t("central.toolbarViewInstalledAll")}</span>
                {installedValue === "all" && <Check className="ml-auto size-3" aria-hidden />}
              </MenuPrimitive.Item>
              <MenuPrimitive.Item
                data-testid="central-toolbar-view-installed-any"
                onClick={handleInstalledAny}
                className={menuItemClassName(installedValue === "installed" && "bg-accent/60 text-accent-foreground")}
              >
                <span>{t("central.toolbarViewInstalledAny", { count: installedCount })}</span>
                {installedValue === "installed" && <Check className="ml-auto size-3" aria-hidden />}
              </MenuPrimitive.Item>
              {platformOptions.length > 0 && (
                <MenuPrimitive.SubmenuRoot>
                  <MenuPrimitive.SubmenuTrigger
                    data-testid="central-toolbar-view-installed-platform-trigger"
                    className={menuItemClassName(Boolean(installedPlatformId) && "bg-accent/60 text-accent-foreground")}
                  >
                    <span>{t("central.toolbarViewInstalledPlatform")}</span>
                    <span aria-hidden className="ml-auto text-[10px] text-muted-foreground">▸</span>
                  </MenuPrimitive.SubmenuTrigger>
                  <MenuPrimitive.Portal>
                    <MenuPrimitive.Positioner align="start" sideOffset={4} className="z-50 outline-none">
                      <MenuPrimitive.Popup className={menuPopupClassName("min-w-[220px] max-h-[300px] overflow-y-auto")}>
                        {platformOptions.map((agent) => {
                          const active = installedPlatformId === agent.id;
                          const displayName = resolvePlatformDisplay(agent, t);
                          const title = resolvePlatformTitle(agent, t);
                          return (
                            <MenuPrimitive.Item
                              key={agent.id}
                              data-testid={`central-toolbar-view-installed-platform-${agent.id}`}
                              title={title}
                              onClick={() => handleInstalledPlatform(agent.id)}
                              className={menuItemClassName(active && "bg-accent/60 text-accent-foreground")}
                            >
                              <span>{displayName}</span>
                              {active && <Check className="ml-auto size-3" aria-hidden />}
                            </MenuPrimitive.Item>
                          );
                        })}
                      </MenuPrimitive.Popup>
                    </MenuPrimitive.Positioner>
                  </MenuPrimitive.Portal>
                </MenuPrimitive.SubmenuRoot>
              )}
            </MenuPrimitive.Group>

            <div role="separator" className="my-1 h-px bg-border/60" />

            <MenuPrimitive.Group>
              <MenuPrimitive.GroupLabel className={menuLabelClassName()}>
                {t("central.toolbarViewSectionQuickFilters")}
              </MenuPrimitive.GroupLabel>
              <MenuPrimitive.CheckboxItem
                checked={uncategorizedActive}
                onCheckedChange={() => handleToggleUncategorized()}
                data-testid="central-toolbar-view-uncategorized"
                className={menuItemClassName()}
              >
                <span>{t("central.toolbarViewUncategorizedOnly")}</span>
                {uncategorizedActive && <Check className="ml-auto size-3" aria-hidden />}
              </MenuPrimitive.CheckboxItem>
            </MenuPrimitive.Group>
          </MenuPrimitive.Popup>
        </MenuPrimitive.Positioner>
      </MenuPrimitive.Portal>
    </MenuPrimitive.Root>
  );
}

function resolvePlatformDisplay(agent: PlatformTarget, t: TFunction): string {
  return getPlatformTargetLabel(agent, t, "full");
}

function resolvePlatformTitle(agent: PlatformTarget, t: TFunction): string {
  return getPlatformTargetTitleHint(agent) || resolvePlatformDisplay(agent, t);
}

function menuPopupClassName(extra?: string): string {
  return cn(
    "min-w-[200px] rounded-xl bg-popover p-1 text-sm text-popover-foreground shadow-[0_0_0_1px_color-mix(in_srgb,var(--foreground)_10%,transparent),0_16px_40px_-18px_color-mix(in_srgb,var(--background)_85%,transparent)] outline-none",
    "data-[starting-style]:animate-in data-[starting-style]:fade-in-0 data-[starting-style]:zoom-in-95",
    "data-[ending-style]:animate-out data-[ending-style]:fade-out-0 data-[ending-style]:zoom-out-95",
    "animation-duration-100",
    extra
  );
}

function menuItemClassName(extra?: string | false): string {
  return cn(
    "flex min-h-8 cursor-pointer items-center gap-2 rounded-lg px-2.5 py-1.5 outline-none transition-[background-color,color] data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground",
    extra
  );
}

function menuLabelClassName(): string {
  return "px-2 pt-1 pb-0.5 text-[10px] uppercase tracking-wide text-muted-foreground/80";
}
