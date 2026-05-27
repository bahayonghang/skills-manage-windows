import { Search, X } from "lucide-react";
import type { TFunction } from "i18next";

import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
  getInstalledFilterPlatformId,
  type InstalledSkillsFilterValue,
} from "@/lib/centralInstalledFilters";
import {
  getPlatformTargetMemberNames,
  isUniversalPlatformTarget,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";
import type {
  CentralQueryAst,
  CentralQueryFilter,
  SkillRepositoryWithStats,
  SkillTag,
} from "@/types";

/**
 * 搜索栏：
 * - 输入框接受结构化语法（tag:foo repo:bar 等），由 view-model 解析
 * - chip 行同时承载：① queryAst.filters；② 已选 repo（viewState.repos）；
 *   ③ 已选 tag（viewState.tags，含 `uncategorized`）；④ installed 快筛
 * - invalid token 以警告 hint 形式提示
 */

export interface CentralSearchBarProps {
  t: TFunction;
  value: string;
  onChange: (next: string) => void;
  queryAst: CentralQueryAst;
  onRemoveFilter: (filterIndex: number) => void;
  onClear: () => void;
  onOpenPalette?: () => void;
  placeholder?: string;
  inputId?: string;

  /** 已选仓库 ID 列表（来自 viewState.repos）。 */
  selectedRepoIds?: readonly string[];
  /** 已选标签 ID 列表（来自 viewState.tags，可能含 `uncategorized`）。 */
  selectedTagIds?: readonly string[];
  /** 仓库元数据，用于映射 ID → 显示名。 */
  repositories?: readonly SkillRepositoryWithStats[];
  /** 标签元数据。 */
  tags?: readonly SkillTag[];
  onRemoveRepoFilter?: (repoId: string) => void;
  onRemoveTagFilter?: (tagId: string) => void;

  /** Installed 快筛当前值。 */
  installedFilterValue?: InstalledSkillsFilterValue;
  /** Installed 快筛命中数（仅当 value === "installed" 时显示）。 */
  installedFilterMatchCount?: number;
  /** 平台元数据，用于解析 platform:xxx 的显示名。 */
  availableInstallAgents?: readonly PlatformTarget[];
  onClearInstalledFilter?: () => void;
}

export function CentralSearchBar({
  t,
  value,
  onChange,
  queryAst,
  onRemoveFilter,
  onClear,
  onOpenPalette,
  placeholder,
  inputId,
  selectedRepoIds,
  selectedTagIds,
  repositories,
  tags,
  onRemoveRepoFilter,
  onRemoveTagFilter,
  installedFilterValue,
  installedFilterMatchCount,
  availableInstallAgents,
  onClearInstalledFilter,
}: CentralSearchBarProps) {
  const hasInvalid = queryAst.invalid.length > 0;
  const repoChips = (selectedRepoIds ?? []).map((id) => ({
    id,
    label: resolveRepoLabel(id, repositories ?? []),
  }));
  const tagChips = (selectedTagIds ?? []).map((id) => ({
    id,
    label: resolveTagLabel(id, tags ?? [], t),
  }));
  const installedChip = resolveInstalledChip(
    installedFilterValue,
    installedFilterMatchCount,
    availableInstallAgents ?? [],
    t
  );

  const hasAnyChip =
    queryAst.filters.length > 0 ||
    repoChips.length > 0 ||
    tagChips.length > 0 ||
    Boolean(installedChip) ||
    hasInvalid;

  return (
    <div data-testid="central-search-bar-v2" className="flex flex-col gap-2">
      <div className="relative flex w-full items-center">
        <Search className="pointer-events-none absolute left-3 size-4 text-muted-foreground/80" />
        <Input
          id={inputId}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          placeholder={placeholder ?? t("central.v2.searchPlaceholder")}
          className="pl-10 pr-24"
          data-testid="central-search-input-v2"
        />
        <div className="absolute right-2 flex items-center gap-1">
          {value.length > 0 && (
            <button
              type="button"
              onClick={onClear}
              aria-label={t("central.v2.selectionClear")}
              className="grid size-7 place-items-center rounded-md text-muted-foreground hover:bg-muted/60 hover:text-foreground"
            >
              <X className="size-4" />
            </button>
          )}
          {onOpenPalette && (
            <button
              type="button"
              data-testid="central-open-palette"
              onClick={onOpenPalette}
              aria-label={t("central.v2.commandPaletteOpen")}
              className="inline-flex items-center gap-1 rounded-md border border-border/80 bg-background px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground hover:border-border hover:bg-muted/40 hover:text-foreground"
            >
              ⌘K
            </button>
          )}
        </div>
      </div>

      {hasAnyChip && (
        <div
          data-testid="central-search-chip-row"
          className="flex flex-wrap items-center gap-1.5 text-[11px]"
        >
          {queryAst.filters.map((filter, idx) => (
            <FilterChip
              key={`q-${filter.kind}-${getChipValue(filter)}-${idx}`}
              filter={filter}
              onRemove={() => onRemoveFilter(idx)}
              t={t}
            />
          ))}
          {repoChips.map((chip) => (
            <SimpleChip
              key={`repo-${chip.id}`}
              testId={`filter-chip-repo-selected-${chip.id}`}
              label={t("central.toolbarChipRepo", { name: chip.label })}
              ariaLabel={t("central.toolbarChipRepoRemove", { name: chip.label })}
              onRemove={onRemoveRepoFilter ? () => onRemoveRepoFilter(chip.id) : undefined}
              tone="primary"
            />
          ))}
          {tagChips.map((chip) => (
            <SimpleChip
              key={`tag-${chip.id}`}
              testId={`filter-chip-tag-selected-${chip.id}`}
              label={
                chip.id === "uncategorized"
                  ? t("central.toolbarChipUncategorized")
                  : t("central.toolbarChipTag", { name: chip.label })
              }
              ariaLabel={t("central.toolbarChipTagRemove", { name: chip.label })}
              onRemove={onRemoveTagFilter ? () => onRemoveTagFilter(chip.id) : undefined}
              tone={chip.id === "uncategorized" ? "muted" : "primary"}
            />
          ))}
          {installedChip && (
            <SimpleChip
              testId="filter-chip-installed"
              label={installedChip.label}
              ariaLabel={installedChip.ariaLabel}
              onRemove={onClearInstalledFilter}
              tone="accent"
            />
          )}
          {hasInvalid && (
            <span
              data-testid="central-search-invalid-hint"
              className="rounded-md border border-amber-500/40 bg-amber-500/10 px-1.5 py-0.5 text-amber-700 dark:text-amber-300"
            >
              {t("central.v2.searchInvalidTokens", { tokens: queryAst.invalid.join(", ") })}
            </span>
          )}
        </div>
      )}
    </div>
  );
}

function getChipValue(filter: CentralQueryFilter): string {
  if (filter.kind === "created" || filter.kind === "updated") {
    return `${filter.op}${filter.value}`;
  }
  return String(filter.value);
}

function resolveRepoLabel(
  repoId: string,
  repositories: readonly SkillRepositoryWithStats[]
): string {
  if (repoId === "unassigned") return "unassigned";
  const repo = repositories.find((r) => r.id === repoId);
  if (!repo) return repoId;
  return repo.name || repo.owner || repo.id;
}

function resolveTagLabel(
  tagId: string,
  tags: readonly SkillTag[],
  t: TFunction
): string {
  if (tagId === "uncategorized") return t("central.toolbarChipUncategorized");
  const tag = tags.find((tg) => tg.id === tagId);
  return tag?.name ?? tagId;
}

function resolveInstalledChip(
  value: InstalledSkillsFilterValue | undefined,
  matchCount: number | undefined,
  agents: readonly PlatformTarget[],
  t: TFunction
): { label: string; ariaLabel: string } | null {
  if (!value || value === "all") return null;
  if (value === "installed") {
    const label = t("central.toolbarChipInstalled", { count: matchCount ?? 0 });
    return { label, ariaLabel: label };
  }
  const platformId = getInstalledFilterPlatformId(value);
  if (!platformId) return null;
  const agent = agents.find((a) => a.id === platformId);
  const displayName = !agent
    ? platformId
    : isUniversalPlatformTarget(agent)
    ? t("platformTargets.universalLabel")
    : agent.display_name;
  const titleAttr = agent && isUniversalPlatformTarget(agent)
    ? getPlatformTargetMemberNames(agent).join(", ")
    : displayName;
  const label = t("central.toolbarChipInstalledPlatform", { platform: displayName });
  return { label, ariaLabel: titleAttr };
}

function FilterChip({
  filter,
  onRemove,
  t,
}: {
  filter: CentralQueryFilter;
  onRemove: () => void;
  t: TFunction;
}) {
  const value = getChipValue(filter);
  const labelKey = filter.negated ? "central.v2.filtersChipNegated" : "central.v2.filtersChip";
  return (
    <span
      data-testid={`filter-chip-${filter.kind}`}
      className={cn(
        "inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 font-mono",
        filter.negated
          ? "border-destructive/40 bg-destructive/10 text-destructive"
          : "border-primary/30 bg-primary/10 text-primary"
      )}
    >
      <span>{t(labelKey, { key: filter.kind, value })}</span>
      <button
        type="button"
        onClick={onRemove}
        aria-label={t("central.v2.filtersRemove", { key: filter.kind, value })}
        className="grid size-3.5 place-items-center rounded-sm hover:bg-foreground/10"
      >
        <X className="size-2.5" />
      </button>
    </span>
  );
}

function SimpleChip({
  testId,
  label,
  ariaLabel,
  onRemove,
  tone,
}: {
  testId: string;
  label: string;
  ariaLabel: string;
  onRemove?: () => void;
  tone: "primary" | "muted" | "accent";
}) {
  return (
    <span
      data-testid={testId}
      className={cn(
        "inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5",
        tone === "primary" && "border-primary/30 bg-primary/10 text-primary",
        tone === "muted" && "border-border/70 bg-muted/40 text-muted-foreground",
        tone === "accent" && "border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300"
      )}
    >
      <span>{label}</span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          aria-label={ariaLabel}
          className="grid size-3.5 place-items-center rounded-sm hover:bg-foreground/10"
        >
          <X className="size-2.5" />
        </button>
      )}
    </span>
  );
}
