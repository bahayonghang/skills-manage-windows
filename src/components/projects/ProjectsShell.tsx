import { useMemo } from "react";
import {
  Folder,
  Loader2,
  PackagePlus,
  Pencil,
  Pin,
  PinOff,
  Plus,
  RefreshCw,
  Search,
  Trash2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { cn } from "@/lib/utils";
import type { Project, ProjectSkill } from "@/types";

interface ProjectsShellProps {
  projects: Project[];
  currentProjectId: string | null;
  skills: ProjectSkill[];
  isAddingProject: boolean;
  scanningProjectIds: Set<string>;
  uninstallingKeys: Set<string>;
  projectSearch: string;
  onProjectSearchChange: (value: string) => void;
  onSelectProject: (id: string) => void;
  onAddProject: () => void | Promise<void>;
  onRescanProject: (id: string) => void | Promise<void>;
  onOpenInstallDialog: () => void;
  onUninstallSkill: (skill: ProjectSkill) => void | Promise<void>;
  onTogglePin: (project: Project) => void | Promise<void>;
  onRequestRename: (project: Project) => void;
  onRequestRemove: (project: Project) => void;
}

function midEllipsis(input: string, max = 56): string {
  if (input.length <= max) return input;
  const head = Math.ceil((max - 1) / 2);
  const tail = Math.floor((max - 1) / 2);
  return `${input.slice(0, head)}…${input.slice(input.length - tail)}`;
}

function ProjectList({
  projects,
  currentProjectId,
  scanningProjectIds,
  projectSearch,
  onProjectSearchChange,
  onSelectProject,
  onTogglePin,
  onRequestRename,
  onRequestRemove,
}: Pick<
  ProjectsShellProps,
  | "projects"
  | "currentProjectId"
  | "scanningProjectIds"
  | "projectSearch"
  | "onProjectSearchChange"
  | "onSelectProject"
  | "onTogglePin"
  | "onRequestRename"
  | "onRequestRemove"
>) {
  const { t } = useTranslation();

  const filtered = useMemo(() => {
    const q = projectSearch.trim().toLowerCase();
    const sorted = [...projects].sort((a, b) => {
      if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    if (!q) return sorted;
    return sorted.filter(
      (p) =>
        p.name.toLowerCase().includes(q) || p.path.toLowerCase().includes(q)
    );
  }, [projects, projectSearch]);

  return (
    <div className="w-64 shrink-0 border-r border-border flex flex-col">
      <div className="p-2 border-b border-border">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground pointer-events-none" />
          <Input
            placeholder={t("projects.searchPlaceholder")}
            value={projectSearch}
            onChange={(event) => onProjectSearchChange(event.target.value)}
            aria-label={t("projects.searchPlaceholder")}
            className="pl-7 pr-7 h-7 text-xs bg-muted/40"
          />
          {projectSearch.length > 0 && (
            <button
              type="button"
              onClick={() => onProjectSearchChange("")}
              aria-label={t("projects.clearSearch")}
              title={t("projects.clearSearch")}
              className="absolute right-1 top-1/2 -translate-y-1/2 p-0.5 rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors cursor-pointer"
            >
              <X className="size-3" />
            </button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto py-1 px-1.5">
        {filtered.length === 0 ? (
          <div className="px-3 py-6 text-center">
            <p className="text-xs text-muted-foreground">
              {projectSearch
                ? t("projects.noMatch", { query: projectSearch })
                : t("projects.emptyHint")}
            </p>
          </div>
        ) : (
          filtered.map((project) => {
            const isActive = currentProjectId === project.id;
            const isScanning = scanningProjectIds.has(project.id);
            return (
              <div
                key={project.id}
                className={cn(
                  "group flex items-center gap-1 w-full text-left transition-colors border rounded-md mb-0.5",
                  isActive
                    ? "bg-primary/10 border-primary/60 text-foreground font-medium shadow-sm"
                    : "border-transparent text-muted-foreground hover:border-border hover:bg-muted/40"
                )}
              >
                <button
                  onClick={() => onSelectProject(project.id)}
                  title={project.path}
                  aria-current={isActive ? "true" : undefined}
                  className="flex items-center gap-2 flex-1 min-w-0 px-2.5 py-2 cursor-pointer"
                >
                  <Folder
                    className={cn(
                      "size-3.5 shrink-0",
                      isActive ? "text-primary" : "text-muted-foreground"
                    )}
                  />
                  <span className="text-sm truncate flex-1">{project.name}</span>
                  {project.pinned && (
                    <Pin className="size-3 shrink-0 text-amber-500" />
                  )}
                  {isScanning ? (
                    <Loader2 className="size-3 shrink-0 animate-spin" />
                  ) : (
                    <span className="text-[10px] font-mono tabular-nums text-muted-foreground shrink-0">
                      {project.skillCount}
                    </span>
                  )}
                </button>
                <div className="hidden group-hover:flex items-center gap-0.5 pr-1">
                  <button
                    type="button"
                    onClick={() => onTogglePin(project)}
                    aria-label={
                      project.pinned
                        ? t("projects.menuUnpin")
                        : t("projects.menuPin")
                    }
                    title={
                      project.pinned
                        ? t("projects.menuUnpin")
                        : t("projects.menuPin")
                    }
                    className="p-1 rounded text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
                  >
                    {project.pinned ? (
                      <PinOff className="size-3" />
                    ) : (
                      <Pin className="size-3" />
                    )}
                  </button>
                  <button
                    type="button"
                    onClick={() => onRequestRename(project)}
                    aria-label={t("projects.menuRename")}
                    title={t("projects.menuRename")}
                    className="p-1 rounded text-muted-foreground hover:bg-muted hover:text-foreground cursor-pointer"
                  >
                    <Pencil className="size-3" />
                  </button>
                  <button
                    type="button"
                    onClick={() => onRequestRemove(project)}
                    aria-label={t("projects.menuRemove")}
                    title={t("projects.menuRemove")}
                    className="p-1 rounded text-muted-foreground hover:bg-destructive/10 hover:text-destructive cursor-pointer"
                  >
                    <Trash2 className="size-3" />
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

interface SkillPanelProps {
  project: Project | null;
  skills: ProjectSkill[];
  isScanning: boolean;
  uninstallingKeys: Set<string>;
  onRescan: () => void | Promise<void>;
  onOpenInstallDialog: () => void;
  onUninstallSkill: (skill: ProjectSkill) => void | Promise<void>;
}

function SkillPanel({
  project,
  skills,
  isScanning,
  uninstallingKeys,
  onRescan,
  onOpenInstallDialog,
  onUninstallSkill,
}: SkillPanelProps) {
  const { t } = useTranslation();

  if (!project) {
    return (
      <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
        <Folder className="size-5 mr-2 opacity-40" />
        {t("projects.noSelection")}
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col min-w-0">
      <div className="px-6 py-3 border-b border-border flex items-center gap-3">
        <div className="min-w-0 flex-1">
          <h2 className="text-sm font-semibold truncate">{project.name}</h2>
          <p
            className="text-xs text-muted-foreground truncate"
            title={project.path}
          >
            {midEllipsis(project.path)}
          </p>
        </div>
        <Button
          variant="default"
          size="sm"
          onClick={onOpenInstallDialog}
          disabled={isScanning}
        >
          <PackagePlus className="size-3.5 mr-1" />
          {t("projects.installFromCentral")}
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={onRescan}
          disabled={isScanning}
        >
          {isScanning ? (
            <Loader2 className="size-3.5 mr-1 animate-spin" />
          ) : (
            <RefreshCw className="size-3.5 mr-1" />
          )}
          {t("projects.rescan")}
        </Button>
      </div>

      <div className="flex-1 overflow-auto p-4">
        {isScanning && skills.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-3 py-12">
            <Loader2 className="size-6 animate-spin text-muted-foreground" />
            <p className="text-sm text-muted-foreground">
              {t("projects.scanning")}
            </p>
          </div>
        ) : skills.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-2 py-12">
            <Folder className="size-8 text-muted-foreground opacity-40" />
            <p className="text-sm text-muted-foreground">
              {t("projects.noSkills")}
            </p>
            <p className="text-xs text-muted-foreground text-center max-w-sm">
              {t("projects.noSkillsHint")}
            </p>
            <Button
              variant="outline"
              size="sm"
              onClick={onOpenInstallDialog}
            >
              <PackagePlus className="size-3.5 mr-1" />
              {t("projects.installFromCentral")}
            </Button>
          </div>
        ) : (
          <div className="space-y-2">
            {skills.map((skill) => {
              const key = `${skill.agentId}:${skill.skillId}`;
              const isLoading = uninstallingKeys.has(key);
              const linkSource =
                skill.linkType === "symlink" ? "symlink" : "copy";
              return (
                <UnifiedSkillCard
                  key={key}
                  name={skill.name}
                  description={skill.description ?? undefined}
                  sourceType={linkSource}
                  platformBadge={{
                    id: skill.agentId,
                    name: skill.agentDisplayName,
                  }}
                  onUninstallFromPlatform={() => onUninstallSkill(skill)}
                  uninstallFromLabel={t("projects.uninstallFromAgent", {
                    agent: skill.agentDisplayName,
                  })}
                  isLoading={isLoading}
                />
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

export function ProjectsShell({
  projects,
  currentProjectId,
  skills,
  isAddingProject,
  scanningProjectIds,
  uninstallingKeys,
  projectSearch,
  onProjectSearchChange,
  onSelectProject,
  onAddProject,
  onRescanProject,
  onOpenInstallDialog,
  onUninstallSkill,
  onTogglePin,
  onRequestRename,
  onRequestRemove,
}: ProjectsShellProps) {
  const { t } = useTranslation();
  const currentProject =
    projects.find((p) => p.id === currentProjectId) ?? null;
  const isCurrentScanning = currentProject
    ? scanningProjectIds.has(currentProject.id)
    : false;

  if (projects.length === 0) {
    return (
      <div className="flex flex-col h-full">
        <div className="border-b border-border px-6 py-4 flex items-center justify-between">
          <div>
            <h1 className="text-xl font-semibold">{t("projects.title")}</h1>
            <p className="text-sm text-muted-foreground mt-0.5">
              {t("projects.subtitle")}
            </p>
          </div>
          <Button
            variant="default"
            size="sm"
            onClick={onAddProject}
            disabled={isAddingProject}
          >
            {isAddingProject ? (
              <Loader2 className="size-3.5 mr-1 animate-spin" />
            ) : (
              <Plus className="size-3.5 mr-1" />
            )}
            {t("projects.addProject")}
          </Button>
        </div>
        <div className="flex-1 flex flex-col items-center justify-center gap-4 py-20">
          <div className="p-4 rounded-full bg-muted/60">
            <Folder className="size-12 text-muted-foreground opacity-60" />
          </div>
          <p className="text-sm text-muted-foreground font-medium">
            {t("projects.emptyTitle")}
          </p>
          <p className="text-xs text-muted-foreground text-center max-w-sm">
            {t("projects.emptyHint")}
          </p>
          <Button
            variant="outline"
            size="sm"
            onClick={onAddProject}
            disabled={isAddingProject}
          >
            <Plus className="size-3.5 mr-1" />
            {t("projects.addFirstProject")}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-border px-6 py-4 flex items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold">{t("projects.title")}</h1>
          <p className="text-sm text-muted-foreground mt-0.5">
            {t("projects.foundSummary", { count: projects.length })}
          </p>
        </div>
        <Button
          variant="default"
          size="sm"
          onClick={onAddProject}
          disabled={isAddingProject}
        >
          {isAddingProject ? (
            <Loader2 className="size-3.5 mr-1 animate-spin" />
          ) : (
            <Plus className="size-3.5 mr-1" />
          )}
          {t("projects.addProject")}
        </Button>
      </div>

      <div className="flex flex-1 min-h-0">
        <ProjectList
          projects={projects}
          currentProjectId={currentProjectId}
          scanningProjectIds={scanningProjectIds}
          projectSearch={projectSearch}
          onProjectSearchChange={onProjectSearchChange}
          onSelectProject={onSelectProject}
          onTogglePin={onTogglePin}
          onRequestRename={onRequestRename}
          onRequestRemove={onRequestRemove}
        />
        <SkillPanel
          project={currentProject}
          skills={skills}
          isScanning={isCurrentScanning}
          uninstallingKeys={uninstallingKeys}
          onRescan={() => {
            if (currentProject) {
              return onRescanProject(currentProject.id);
            }
          }}
          onOpenInstallDialog={onOpenInstallDialog}
          onUninstallSkill={onUninstallSkill}
        />
      </div>
    </div>
  );
}
