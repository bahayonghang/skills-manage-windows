import { useEffect, useState } from "react";
import { ChevronDown, ChevronRight, FolderGit2, FolderOpen, Tag, Trash2 } from "lucide-react";
import type { TFunction } from "i18next";

import { FacetItem } from "@/components/central/FacetItem";
import { useSidebarExpansionSignal } from "@/components/central/useSidebarExpansionSignal";
import { groupRepositoriesForSidebar } from "@/lib/centralRepositoryGroups";
import { cn } from "@/lib/utils";
import type { FacetCounts } from "@/lib/centralFacetCounts";
import type { SkillRepositoryWithStats, SkillTag, TagGroup } from "@/types";

// ─── Repository blocks ─────────────────────────────────────────────────────────

interface RepositorySectionBlockProps {
  section: ReturnType<typeof groupRepositoriesForSidebar>[number];
  facetCounts: FacetCounts;
  selectionSet: Set<string>;
  onToggleRepo: (id: string) => void;
  onDeleteRepository?: (repository: SkillRepositoryWithStats) => void;
  t: TFunction;
}

export function RepositorySectionBlock({
  section,
  facetCounts,
  selectionSet,
  onToggleRepo,
  onDeleteRepository,
  t,
}: RepositorySectionBlockProps) {
  const sectionLabel =
    section.kind === "github" ? t("central.v2.githubSection") : t("central.v2.localSection");

  return (
    <div data-testid={`repo-source-${section.kind}`} className="space-y-1">
      <div className="px-2 pt-2 text-[11px] font-medium tracking-normal text-muted-foreground/70">
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
              onDeleteRepository={onDeleteRepository}
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
            trailingAction={
              onDeleteRepository && !repo.is_unknown ? (
                <RepoDeleteTrailingAction
                  repository={repo}
                  onDelete={onDeleteRepository}
                  t={t}
                />
              ) : undefined
            }
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
  onDeleteRepository?: (repository: SkillRepositoryWithStats) => void;
  t: TFunction;
}

function OwnerGroup({
  owner,
  repositories,
  totalCount,
  facetCounts,
  selectionSet,
  onToggleRepo,
  onDeleteRepository,
  t,
}: OwnerGroupProps) {
  const [expanded, setExpanded] = useState(true);
  const expansionSignal = useSidebarExpansionSignal();
  const expansionToken = expansionSignal?.token;
  const forcedExpanded = expansionSignal?.expanded;
  const Caret = expanded ? ChevronDown : ChevronRight;
  const hasSelected = repositories.some((r) => selectionSet.has(r.id));

  useEffect(() => {
    if (forcedExpanded === undefined) return;
    setExpanded(forcedExpanded);
  }, [forcedExpanded, expansionToken]);

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
              trailingAction={
                onDeleteRepository ? (
                  <RepoDeleteTrailingAction
                    repository={repo}
                    onDelete={onDeleteRepository}
                    t={t}
                  />
                ) : undefined
              }
            />
          ))}
        </div>
      )}
    </div>
  );
}

function RepoDeleteTrailingAction({
  repository,
  onDelete,
  t,
}: {
  repository: SkillRepositoryWithStats;
  onDelete: (repository: SkillRepositoryWithStats) => void;
  t: TFunction;
}) {
  const displayName = repository.repo ?? repository.name ?? repository.id;
  return (
    <button
      type="button"
      data-testid={`repo-delete-${repository.id}`}
      aria-label={t("central.v2.sidebarDeleteRepoLabel", { name: displayName })}
      title={t("central.v2.sidebarDeleteRepo")}
      onClick={(event) => {
        event.stopPropagation();
        onDelete(repository);
      }}
      className="ml-1 grid size-6 place-items-center rounded-md text-muted-foreground opacity-0 transition-opacity hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100 focus-visible:opacity-100"
    >
      <Trash2 className="size-3" />
    </button>
  );
}

// ─── Tag blocks ────────────────────────────────────────────────────────────────

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

export function TagGroupBlock({
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
  const expansionSignal = useSidebarExpansionSignal();
  const expansionToken = expansionSignal?.token;
  const forcedExpanded = expansionSignal?.expanded;
  const Caret = expanded ? ChevronDown : ChevronRight;
  const totalCount = tags.reduce((acc, tag) => acc + (facetCounts.tags[tag.id] ?? 0), 0);
  const label = group?.name ?? ungroupedLabel ?? "Other";
  const testId = group ? `tag-group-${group.id}` : "tag-group-ungrouped";

  useEffect(() => {
    if (forcedExpanded === undefined) return;
    setExpanded(forcedExpanded);
  }, [forcedExpanded, expansionToken]);

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

export function SidebarTagRow({
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
