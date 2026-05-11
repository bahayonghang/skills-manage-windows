import { useState, type ReactNode } from "react";
import {
  ChevronDown,
  ChevronRight,
  CircleAlert,
  FolderGit2,
  FolderOpen,
  Inbox,
  Layers,
  Sparkles,
  Tag,
} from "lucide-react";
import type { TFunction } from "i18next";

import { FacetItem } from "@/components/central/v2/FacetItem";
import { FacetSection } from "@/components/central/v2/FacetSection";
import { groupRepositoriesForSidebar } from "@/lib/centralRepositoryGroups";
import { cn } from "@/lib/utils";
import type { FacetCounts } from "@/lib/centralFacetCounts";
import type { SkillRepositoryWithStats, SkillTag, TagGroup } from "@/types";

/**
 * Central Skills V2 三段式 sidebar：智能视图 / 仓库（按 owner 折叠） / 标签。
 *
 * - 多选语义：维度内 OR，跨维度 AND
 * - 任意 facet 项使用 `FacetItem`，section 使用 `FacetSection`
 * - "全部技能" 提供 quick-clear（点击清空所有 facet 选择）
 */

interface CommonSidebarProps {
  t: TFunction;
  width: number;
  startResize: (event: React.PointerEvent<HTMLElement>) => void;
  handleResizeKeyDown: (event: React.KeyboardEvent<HTMLElement>) => void;

  facetCounts: FacetCounts;
  repositories: readonly SkillRepositoryWithStats[];
  tags: readonly SkillTag[];
  /** 标签分组（M3）。为空数组时退化为 M1 的平铺展示。 */
  tagGroups?: readonly TagGroup[];

  selectedRepos: readonly string[];
  selectedTags: readonly string[];

  onToggleRepo: (repoId: string) => void;
  onToggleTag: (tagId: string) => void;
  onClearAll: () => void;
  /** 智能视图改写为 tag facet 中的特殊值。 */
  onSelectSmartView: (view: "all" | "uncategorized" | "updates" | "ai-review") => void;
  /** 分配 tag 到指定分组（M6）。传 null 为移出分组。 */
  onAssignTagToGroup?: (tagId: string, groupId: string | null) => void;
  /** Sidebar 顶部插槽（M2 加入。M3 中用于注入合并后的 Saved Views + Tag Groups header）。 */
  savedViewsSlot?: ReactNode;
}

export function CentralSidebarV2({
  t,
  width,
  startResize,
  handleResizeKeyDown,
  facetCounts,
  repositories,
  tags,
  tagGroups = [],
  selectedRepos,
  selectedTags,
  onToggleRepo,
  onToggleTag,
  onClearAll,
  onSelectSmartView,
  onAssignTagToGroup,
  savedViewsSlot,
}: CommonSidebarProps) {
  const repoSelectionSet = new Set(selectedRepos);
  const tagSelectionSet = new Set(selectedTags);

  const repositorySections = groupRepositoriesForSidebar(repositories);

  const hasAnySelection = selectedRepos.length > 0 || selectedTags.length > 0;

  const tagsSorted = [...tags].sort((a, b) =>
    a.name.localeCompare(b.name, undefined, { sensitivity: "base" })
  );

  // 按 group 分桶。未分组的进入虚拟“Other”，放最后。
  const tagGroupSorted = [...tagGroups].sort((a, b) => a.sort_order - b.sort_order);
  const tagsByGroup = new Map<string, SkillTag[]>();
  for (const t of tagsSorted) {
    const key = t.group_id ?? "__ungrouped__";
    const list = tagsByGroup.get(key);
    if (list) list.push(t);
    else tagsByGroup.set(key, [t]);
  }

  return (
    <aside
      data-testid="central-sidebar-v2"
      className="relative hidden min-h-0 shrink-0 border-r border-border/80 bg-muted/15 md:flex md:flex-col"
      style={{ width }}
    >
      <div className="scrollbar-subtle min-h-0 flex-1 overflow-y-auto px-3 pb-6 pr-2">
        {savedViewsSlot ? <div className="mb-2">{savedViewsSlot}</div> : null}
        {/* Smart views ─────────────────────────────────────────────────── */}
        <FacetSection
          title={t("central.v2.smartViews")}
          icon={<Sparkles className="size-3.5" />}
          testId="sidebar-section-smart"
        >
          <FacetItem
            label={t("central.v2.allSkills")}
            active={!hasAnySelection}
            count={facetCounts.smartViews.all}
            icon={<Layers className="size-3.5" />}
            testId="smart-view-all"
            onClick={() => {
              onSelectSmartView("all");
              onClearAll();
            }}
          />
          <FacetItem
            label={t("central.v2.uncategorized")}
            active={tagSelectionSet.has("uncategorized")}
            count={facetCounts.smartViews.uncategorized}
            icon={<Inbox className="size-3.5" />}
            testId="smart-view-uncategorized"
            onClick={() => onToggleTag("uncategorized")}
          />
          <FacetItem
            label={t("central.v2.updates")}
            active={tagSelectionSet.has("updates")}
            count={facetCounts.smartViews.updates}
            icon={<CircleAlert className="size-3.5" />}
            testId="smart-view-updates"
            onClick={() => onToggleTag("updates")}
          />
          <FacetItem
            label={t("central.v2.aiReview")}
            active={tagSelectionSet.has("ai-review")}
            count={facetCounts.smartViews.aiReview}
            icon={<Sparkles className="size-3.5" />}
            testId="smart-view-ai-review"
            onClick={() => onToggleTag("ai-review")}
          />
        </FacetSection>

        {/* Repositories ────────────────────────────────────────────────── */}
        <div className="mt-4">
          <FacetSection
            title={t("central.v2.repositories")}
            icon={<FolderGit2 className="size-3.5" />}
            testId="sidebar-section-repos"
          >
            {repositorySections.length === 0 ? (
              <p className="px-2 text-[11px] text-muted-foreground">
                {t("central.v2.facetEmpty")}
              </p>
            ) : (
              repositorySections.map((section) => (
                <RepositorySectionBlock
                  key={section.kind}
                  section={section}
                  facetCounts={facetCounts}
                  selectionSet={repoSelectionSet}
                  onToggleRepo={onToggleRepo}
                  t={t}
                />
              ))
            )}
          </FacetSection>
        </div>

        {/* Tags ─────────────────────────────────────────────────────── */}
        <div className="mt-4">
          <FacetSection
            title={t("central.v2.tags")}
            icon={<Tag className="size-3.5" />}
            testId="sidebar-section-tags"
          >
            {tagsSorted.length === 0 ? (
              <p className="px-2 text-[11px] text-muted-foreground">
                {t("central.v2.facetEmpty")}
              </p>
            ) : tagGroupSorted.length === 0 ? (
              tagsSorted.map((tag) => (
                <SidebarTagRow
                  key={tag.id}
                  tag={tag}
                  active={tagSelectionSet.has(tag.id)}
                  count={facetCounts.tags[tag.id] ?? 0}
                  onToggle={() => onToggleTag(tag.id)}
                  tagGroups={tagGroupSorted}
                  onAssignTagToGroup={onAssignTagToGroup}
                  t={t}
                />
              ))
            ) : (
              <>
                {tagGroupSorted.map((group) => {
                  const groupTags = tagsByGroup.get(group.id) ?? [];
                  if (groupTags.length === 0) return null;
                  return (
                    <TagGroupBlock
                      key={group.id}
                      group={group}
                      tags={groupTags}
                      tagSelectionSet={tagSelectionSet}
                      facetCounts={facetCounts}
                      onToggleTag={onToggleTag}
                      allTagGroups={tagGroupSorted}
                      onAssignTagToGroup={onAssignTagToGroup}
                      t={t}
                    />
                  );
                })}
                {(() => {
                  const ungrouped = tagsByGroup.get("__ungrouped__") ?? [];
                  if (ungrouped.length === 0) return null;
                  return (
                    <TagGroupBlock
                      key="__ungrouped__"
                      group={null}
                      tags={ungrouped}
                      tagSelectionSet={tagSelectionSet}
                      facetCounts={facetCounts}
                      onToggleTag={onToggleTag}
                      ungroupedLabel={t("central.v2.tagGroupsUngrouped")}
                      allTagGroups={tagGroupSorted}
                      onAssignTagToGroup={onAssignTagToGroup}
                      t={t}
                    />
                  );
                })()}
              </>
            )}
          </FacetSection>
        </div>

        {/* Quick clear footer */}
        {hasAnySelection && (
          <div className="sticky bottom-0 -mx-3 mt-4 border-t border-border/60 bg-background/90 px-4 py-2 backdrop-blur-md">
            <button
              type="button"
              onClick={onClearAll}
              className="w-full rounded-md border border-border/70 bg-background px-2.5 py-1.5 text-[11px] font-medium text-muted-foreground hover:border-border hover:bg-muted/40 hover:text-foreground"
            >
              {t("central.v2.selectionClear")}
            </button>
            <p className="mt-1.5 text-center text-[10px] text-muted-foreground">
              {t("central.v2.selectionApplied", {
                count: selectedRepos.length + selectedTags.length,
              })}
            </p>
          </div>
        )}
      </div>
      <div
        role="separator"
        aria-label={t("central.resizeFilterSidebar")}
        aria-orientation="vertical"
        tabIndex={0}
        onPointerDown={startResize}
        onKeyDown={handleResizeKeyDown}
        className="absolute right-0 top-0 z-20 h-full w-1.5 translate-x-1/2 cursor-col-resize rounded-full bg-transparent transition-colors hover:bg-primary/30 focus:outline-none focus-visible:bg-primary/40"
      />
    </aside>
  );
}

interface RepositorySectionBlockProps {
  section: ReturnType<typeof groupRepositoriesForSidebar>[number];
  facetCounts: FacetCounts;
  selectionSet: Set<string>;
  onToggleRepo: (id: string) => void;
  t: TFunction;
}

function RepositorySectionBlock({
  section,
  facetCounts,
  selectionSet,
  onToggleRepo,
  t,
}: RepositorySectionBlockProps) {
  const sectionLabel =
    section.kind === "github" ? t("central.v2.githubSection") : t("central.v2.localSection");

  return (
    <div data-testid={`repo-source-${section.kind}`} className="space-y-1">
      <div className="px-2 pt-2 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground/70">
        {sectionLabel}
      </div>
      {section.groups.map((group) => {
        if (group.kind === "owner") {
          return (
            <OwnerGroup
              key={group.owner}
              owner={group.owner}
              repositories={group.repositories}
              totalCount={group.totalSkillCount}
              facetCounts={facetCounts}
              selectionSet={selectionSet}
              onToggleRepo={onToggleRepo}
              t={t}
            />
          );
        }
        return group.repositories.map((repo) => (
          <FacetItem
            key={repo.id}
            label={repo.name}
            active={selectionSet.has(repo.is_unknown ? "unassigned" : repo.id)}
            count={facetCounts.repositories[repo.id] ?? 0}
            icon={<FolderOpen className="size-3.5" />}
            testId={`repo-${repo.id}`}
            onClick={() => onToggleRepo(repo.is_unknown ? "unassigned" : repo.id)}
          />
        ));
      })}
    </div>
  );
}

interface OwnerGroupProps {
  owner: string;
  repositories: SkillRepositoryWithStats[];
  totalCount: number;
  facetCounts: FacetCounts;
  selectionSet: Set<string>;
  onToggleRepo: (id: string) => void;
  t: TFunction;
}

function OwnerGroup({
  owner,
  repositories,
  totalCount,
  facetCounts,
  selectionSet,
  onToggleRepo,
  t,
}: OwnerGroupProps) {
  const [expanded, setExpanded] = useState(true);
  const Caret = expanded ? ChevronDown : ChevronRight;
  const hasSelected = repositories.some((r) => selectionSet.has(r.id));

  return (
    <div className="space-y-1">
      <button
        type="button"
        data-testid={`owner-${owner}`}
        onClick={() => setExpanded((v) => !v)}
        aria-expanded={expanded}
        aria-label={expanded ? t("central.v2.ownerCollapse", { owner }) : t("central.v2.ownerExpand", { owner })}
        className={cn(
          "flex w-full items-center gap-1.5 rounded-md border px-2 py-1.5 text-left text-xs transition-colors",
          hasSelected
            ? "border-primary/20 bg-background text-foreground"
            : "border-transparent text-muted-foreground hover:border-border/70 hover:bg-background/70 hover:text-foreground"
        )}
      >
        <Caret className="size-3 shrink-0" />
        <FolderGit2 className="size-3.5 shrink-0 text-sky-600 dark:text-sky-300" />
        <span className="min-w-0 flex-1 truncate font-medium">{owner}</span>
        <span className="shrink-0 rounded-md border border-border/80 bg-background px-1.5 py-0.5 font-mono text-[10px] tabular-nums">
          {totalCount}
        </span>
      </button>
      {expanded && (
        <div>
          {repositories.map((repo) => (
            <FacetItem
              key={repo.id}
              label={repo.repo ?? repo.name}
              active={selectionSet.has(repo.id)}
              count={facetCounts.repositories[repo.id] ?? 0}
              icon={<FolderOpen className="size-3 text-muted-foreground/70" />}
              indentLevel={1}
              testId={`repo-${repo.id}`}
              onClick={() => onToggleRepo(repo.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface TagGroupBlockProps {
  /** `null` 表示「未分组」虚拟分组。 */
  group: TagGroup | null;
  tags: SkillTag[];
  tagSelectionSet: Set<string>;
  facetCounts: FacetCounts;
  onToggleTag: (tagId: string) => void;
  /** 仅 `group === null` 时使用。 */
  ungroupedLabel?: string;
  /** M6：本列表中可选的全部分组（供 assign select 使用）。 */
  allTagGroups?: readonly TagGroup[];
  onAssignTagToGroup?: (tagId: string, groupId: string | null) => void;
  t?: TFunction;
}

function TagGroupBlock({
  group,
  tags,
  tagSelectionSet,
  facetCounts,
  onToggleTag,
  ungroupedLabel,
  allTagGroups,
  onAssignTagToGroup,
  t,
}: TagGroupBlockProps) {
  const [expanded, setExpanded] = useState(true);
  const Caret = expanded ? ChevronDown : ChevronRight;
  const totalCount = tags.reduce((acc, tag) => acc + (facetCounts.tags[tag.id] ?? 0), 0);
  const label = group?.name ?? ungroupedLabel ?? "Other";
  const testId = group ? `tag-group-${group.id}` : "tag-group-ungrouped";

  return (
    <div className="space-y-1">
      <button
        type="button"
        data-testid={testId}
        onClick={() => setExpanded((v) => !v)}
        aria-expanded={expanded}
        className={cn(
          "flex w-full items-center gap-1.5 rounded-md border border-transparent px-2 py-1.5 text-left text-xs text-muted-foreground hover:border-border/70 hover:bg-background/70 hover:text-foreground"
        )}
      >
        <Caret className="size-3 shrink-0" />
        <span
          className="size-2.5 shrink-0 rounded-sm border border-border/60"
          style={group?.color ? { backgroundColor: group.color } : undefined}
        />
        <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
        <span className="shrink-0 rounded-md border border-border/80 bg-background px-1.5 py-0.5 font-mono text-[10px] tabular-nums">
          {totalCount}
        </span>
      </button>
      {expanded && (
        <div>
          {tags.map((tag) => (
            <SidebarTagRow
              key={tag.id}
              tag={tag}
              active={tagSelectionSet.has(tag.id)}
              count={facetCounts.tags[tag.id] ?? 0}
              indentLevel={1}
              onToggle={() => onToggleTag(tag.id)}
              tagGroups={allTagGroups ?? []}
              onAssignTagToGroup={onAssignTagToGroup}
              t={t}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface SidebarTagRowProps {
  tag: SkillTag;
  active: boolean;
  count: number;
  indentLevel?: 0 | 1;
  onToggle: () => void;
  tagGroups: readonly TagGroup[];
  onAssignTagToGroup?: (tagId: string, groupId: string | null) => void;
  t?: TFunction;
}

function SidebarTagRow({
  tag,
  active,
  count,
  indentLevel = 0,
  onToggle,
  tagGroups,
  onAssignTagToGroup,
  t,
}: SidebarTagRowProps) {
  const showAssign = onAssignTagToGroup !== undefined && tagGroups.length > 0;
  const trailing = showAssign ? (
    <select
      aria-label={t ? t("central.v2.tagGroupsAssignTo") : "Assign tag to group"}
      data-testid={`tag-assign-${tag.id}`}
      value={tag.group_id ?? ""}
      onChange={(e) => onAssignTagToGroup(tag.id, e.target.value === "" ? null : e.target.value)}
      onClick={(e) => e.stopPropagation()}
      className="ml-1 rounded-md border border-border/60 bg-background px-1 py-0.5 text-[10px] text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 focus:opacity-100"
    >
      <option value="">{t ? t("central.v2.tagGroupsAssignNone") : "None"}</option>
      {tagGroups.map((g) => (
        <option key={g.id} value={g.id}>{g.name}</option>
      ))}
    </select>
  ) : undefined;

  return (
    <FacetItem
      label={tag.name}
      active={active}
      count={count}
      icon={<Tag className={indentLevel === 1 ? "size-3 text-muted-foreground/70" : "size-3.5"} />}
      indentLevel={indentLevel}
      testId={`tag-${tag.id}`}
      onClick={onToggle}
      trailingAction={trailing}
    />
  );
}
