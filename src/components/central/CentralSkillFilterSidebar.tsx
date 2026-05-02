import type { ReactNode } from "react";
import { FolderGit2, FolderOpen, Tag, Trash2 } from "lucide-react";
import type { TFunction } from "i18next";

import { cn } from "@/lib/utils";
import type {
  SkillAiTagReview,
  SkillRepositoryWithStats,
  SkillTag,
} from "@/types";

type TagCount = {
  tag: SkillTag;
  count: number;
};

function isGithubRepository(repository: SkillRepositoryWithStats): boolean {
  return (
    repository.source_type === "github" ||
    Boolean(repository.owner && repository.repo)
  );
}

function getRepositorySkillCount(repository: SkillRepositoryWithStats): number {
  return repository.is_unknown
    ? repository.unknown_skill_count
    : repository.skill_count;
}

function FilterSection({
  icon,
  title,
  children,
}: {
  icon: ReactNode;
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="space-y-2.5">
      <div className="flex items-center gap-2 px-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
        {icon}
        <span>{title}</span>
      </div>
      <div className="space-y-1.5">{children}</div>
    </section>
  );
}

function RepositoryFilterButton({
  active,
  count,
  label,
  sourceKind,
  onClick,
  onDelete,
  deleteLabel,
  deleteDisabled,
  isDeleteLoading,
  testId,
}: {
  active: boolean;
  count: number;
  label: string;
  sourceKind: "all" | "github" | "local";
  onClick: () => void;
  onDelete?: () => void;
  deleteLabel?: string;
  deleteDisabled?: boolean;
  isDeleteLoading?: boolean;
  testId: string;
}) {
  const Icon = sourceKind === "github" ? FolderGit2 : FolderOpen;

  return (
    <div
      className={cn(
        "group flex min-h-10 w-full items-center gap-2.5 rounded-lg border px-2.5 py-2 text-left text-xs transition-colors",
        active
          ? "border-primary/25 bg-background text-foreground shadow-sm ring-1 ring-primary/10"
          : "border-transparent text-muted-foreground hover:border-border/70 hover:bg-background/70 hover:text-foreground",
        sourceKind === "local" && !active
          ? "border-dashed border-border/80 bg-muted/20"
          : null
      )}
    >
      <button
        type="button"
        data-testid={testId}
        data-source-kind={sourceKind}
        onClick={onClick}
        className="flex min-w-0 flex-1 items-center gap-2.5 text-left"
      >
        <span
          className={cn(
            "grid size-6 shrink-0 place-items-center rounded-md border",
            sourceKind === "github"
              ? "border-sky-500/20 bg-sky-500/10 text-sky-600 dark:text-sky-300"
              : sourceKind === "local"
                ? "border-dashed border-border bg-muted text-muted-foreground"
                : "border-border bg-background text-muted-foreground"
          )}
        >
          <Icon className="size-3.5" />
        </span>
        <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
      </button>
      <span className="shrink-0 rounded-md border border-border/80 bg-background px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-muted-foreground">
        {count}
      </span>
      {onDelete && (
        <button
          type="button"
          data-testid={`${testId}-delete`}
          aria-label={deleteLabel}
          title={deleteLabel}
          disabled={deleteDisabled || isDeleteLoading}
          onClick={(event) => {
            event.stopPropagation();
            onDelete();
          }}
          className="grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground opacity-0 transition hover:bg-destructive/10 hover:text-destructive focus:opacity-100 disabled:pointer-events-none disabled:opacity-40 group-hover:opacity-100"
        >
          <Trash2 className={cn("size-3.5", isDeleteLoading && "animate-pulse")} />
        </button>
      )}
    </div>
  );
}

function TagFilterButton({
  active,
  count,
  label,
  tone,
  onClick,
  testId,
}: {
  active: boolean;
  count: number;
  label: string;
  tone: "all" | "tag" | "uncategorized" | "updates" | "ai";
  onClick: () => void;
  testId: string;
}) {
  const dotClassName =
    tone === "updates"
      ? "bg-amber-500"
      : tone === "ai"
        ? "bg-violet-500"
        : tone === "uncategorized"
          ? "bg-slate-400"
          : tone === "tag"
            ? "bg-emerald-500"
            : "bg-muted-foreground/60";

  return (
    <button
      type="button"
      data-testid={testId}
      data-filter-kind="tag"
      onClick={onClick}
      className={cn(
        "flex min-h-9 w-full items-center gap-2 rounded-full border px-2.5 py-1.5 text-left text-xs transition-colors",
        active
          ? "border-primary/30 bg-primary/10 text-primary ring-1 ring-primary/10"
          : "border-border/70 bg-background/35 text-muted-foreground hover:bg-background hover:text-foreground"
      )}
    >
      <span className={cn("size-2 shrink-0 rounded-full", dotClassName)} />
      <span className="min-w-0 flex-1 truncate font-medium">{label}</span>
      <span className="shrink-0 rounded-full bg-muted px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-muted-foreground">
        {count}
      </span>
    </button>
  );
}

export function CentralSkillFilterSidebar({
  aiTagReviews,
  filterSidebarWidth,
  isDeleting,
  isRepositoryDeletePreviewLoading,
  repositoryDeleteTargetId,
  repositoryFilter,
  repositories,
  setCategorizeTab,
  setRepositoryFilter,
  setTagFilter,
  skillsCount,
  startFilterSidebarResize,
  tagCounts,
  tagFilter,
  t,
  uncategorizedCount,
  updateAvailableSkillCount,
  handleFilterSidebarResizeKeyDown,
  onRepositoryDelete,
}: {
  aiTagReviews: SkillAiTagReview[];
  filterSidebarWidth: number;
  isDeleting: boolean;
  isRepositoryDeletePreviewLoading: boolean;
  repositoryDeleteTargetId?: string;
  repositoryFilter: string;
  repositories: SkillRepositoryWithStats[];
  setCategorizeTab: (tab: "manual" | "ai" | "review") => void;
  setRepositoryFilter: (filter: string) => void;
  setTagFilter: (filter: string) => void;
  skillsCount: number;
  startFilterSidebarResize: (event: React.PointerEvent<HTMLElement>) => void;
  tagCounts: TagCount[];
  tagFilter: string;
  t: TFunction;
  uncategorizedCount: number;
  updateAvailableSkillCount: number;
  handleFilterSidebarResizeKeyDown: (event: React.KeyboardEvent<HTMLElement>) => void;
  onRepositoryDelete: (repository: SkillRepositoryWithStats) => void;
}) {
  return (
    <aside
      data-testid="central-filter-sidebar"
      className="relative hidden min-h-0 shrink-0 border-r border-border/80 bg-muted/15 md:flex md:flex-col"
      style={{ width: filterSidebarWidth }}
    >
      <div className="scrollbar-subtle min-h-0 flex-1 space-y-5 overflow-y-auto px-3 py-4 pr-2">
        <FilterSection
          icon={<FolderOpen className="size-3.5" />}
          title={t("central.repositories")}
        >
          <RepositoryFilterButton
            active={repositoryFilter === "all"}
            count={skillsCount}
            label={t("central.allRepositories")}
            sourceKind="all"
            testId="repository-filter-all"
            onClick={() => setRepositoryFilter("all")}
          />
          {repositories.map((repository) => {
            const sourceKind = isGithubRepository(repository) ? "github" : "local";
            const isActive =
              (repository.is_unknown && repositoryFilter === "unassigned") ||
              repositoryFilter === repository.id;

            return (
              <RepositoryFilterButton
                key={repository.id}
                active={isActive}
                count={getRepositorySkillCount(repository)}
                label={repository.name}
                sourceKind={sourceKind}
                testId={`repository-filter-${repository.id}`}
                deleteLabel={t("central.deleteRepositoryLabel", {
                  name: repository.name,
                })}
                deleteDisabled={isDeleting}
                isDeleteLoading={
                  isRepositoryDeletePreviewLoading &&
                  repositoryDeleteTargetId === repository.id
                }
                onDelete={
                  repository.is_unknown
                    ? undefined
                    : () => {
                        onRepositoryDelete(repository);
                      }
                }
                onClick={() =>
                  setRepositoryFilter(repository.is_unknown ? "unassigned" : repository.id)
                }
              />
            );
          })}
        </FilterSection>

        <FilterSection
          icon={<Tag className="size-3.5" />}
          title={t("central.categoryNav")}
        >
          <TagFilterButton
            active={tagFilter === "all"}
            count={skillsCount}
            label={t("central.allTags")}
            tone="all"
            testId="tag-filter-all"
            onClick={() => setTagFilter("all")}
          />
          <TagFilterButton
            active={tagFilter === "uncategorized"}
            count={uncategorizedCount}
            label={t("central.uncategorizedOnly")}
            tone="uncategorized"
            testId="tag-filter-uncategorized"
            onClick={() => setTagFilter("uncategorized")}
          />
          <TagFilterButton
            active={tagFilter === "updates"}
            count={updateAvailableSkillCount}
            label={t("central.updatesAvailableOnly")}
            tone="updates"
            testId="tag-filter-updates"
            onClick={() => setTagFilter("updates")}
          />
          <TagFilterButton
            active={tagFilter === "ai-review"}
            count={aiTagReviews.length}
            label={t("central.aiReviewOnly")}
            tone="ai"
            testId="tag-filter-ai-review"
            onClick={() => {
              setTagFilter("ai-review");
              setCategorizeTab("review");
            }}
          />
          {tagCounts.map(({ tag, count }) => (
            <TagFilterButton
              key={tag.id}
              active={tagFilter === tag.id}
              count={count}
              label={tag.name}
              tone="tag"
              testId={`tag-filter-${tag.id}`}
              onClick={() => setTagFilter(tag.id)}
            />
          ))}
        </FilterSection>
      </div>
      <div
        role="separator"
        aria-label={t("central.resizeFilterSidebar")}
        aria-orientation="vertical"
        tabIndex={0}
        onPointerDown={startFilterSidebarResize}
        onKeyDown={handleFilterSidebarResizeKeyDown}
        className="absolute right-0 top-0 z-20 h-full w-1.5 translate-x-1/2 cursor-col-resize rounded-full bg-transparent transition-colors hover:bg-primary/30 focus:outline-none focus-visible:bg-primary/40"
      />
    </aside>
  );
}
