import type { RefObject } from "react";
import {
  ArrowUpRight,
  Folder,
  Loader2,
  Radar,
  RefreshCw,
  Search,
  StopCircle,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { InstallDialog } from "@/components/central/InstallDialog";
import { DiscoverConfigDialog } from "@/components/discover/DiscoverConfigDialog";
import { SkillDetailDrawer } from "@/components/skill/SkillDetailDrawer";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { VirtualizedList } from "@/components/ui/virtualized-list";
import { getPathBasename } from "@/lib/path";
import { cn } from "@/lib/utils";
import type { DiscoverViewModel } from "@/pages/discoverViewModel";
import type {
  AgentWithStatus,
  BatchInstallResult,
  DiscoveredProject,
  DiscoveredSkill,
  SkillWithLinks,
} from "@/types";
import type { DiscoverMetadata } from "@/components/skill/SkillDetailView";

interface DiscoverProgressViewProps {
  currentPath: string;
  onStopScan: () => void | Promise<void>;
  projectsFoundSoFar: number;
  scanProgress: number;
  skillsFoundSoFar: number;
}

function DiscoverEmptyState() {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-center justify-center h-full gap-4 py-20">
      <div className="p-4 rounded-full bg-muted/60">
        <Radar className="size-12 text-muted-foreground opacity-60" />
      </div>
      <p className="text-sm text-muted-foreground font-medium">
        {t("discover.noResults")}
      </p>
      <p className="text-xs text-muted-foreground text-center max-w-sm">
        {t("discover.noResultsDesc")}
      </p>
    </div>
  );
}

function DiscoverProgressView({
  currentPath,
  onStopScan,
  projectsFoundSoFar,
  scanProgress,
  skillsFoundSoFar,
}: DiscoverProgressViewProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-4 py-6 max-w-lg mx-auto">
      <div className="flex items-center gap-2 text-sm">
        <Loader2 className="size-4 animate-spin" />
        <span className="font-medium">{t("discover.scanning")}</span>
      </div>

      <div className="w-full bg-muted rounded-full h-2">
        <div
          className="bg-primary h-2 w-full origin-left rounded-full transition-transform duration-300 ease-out"
          style={{ transform: `scaleX(${scanProgress / 100})` }}
        />
      </div>

      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>
          {t("discover.progress", { percent: scanProgress, path: currentPath })}
        </span>
        <span>
          {t("discover.foundSoFar", {
            skills: skillsFoundSoFar,
            projects: projectsFoundSoFar,
          })}
        </span>
      </div>

      <div className="flex justify-center pt-2">
        <Button variant="destructive" size="default" onClick={onStopScan}>
          <StopCircle className="size-4 mr-1.5" />
          {t("discover.stopAndShow")}
        </Button>
      </div>
    </div>
  );
}

interface DiscoverSkillCardProps {
  importingIds: Set<string>;
  onInstallToCentral: (skillId: string) => void | Promise<void>;
  onInstallToPlatform: (skill: DiscoveredSkill) => void;
  onOpenDiscoverDrawer: (skill: DiscoveredSkill) => void;
  onOpenDrawer: (skillId: string) => void;
  onSetDetailButtonRef: (
    skillId: string,
    node: HTMLButtonElement | null
  ) => void;
  onToggleSkillSelection: (skillId: string) => void;
  selectedSkillIds: Set<string>;
  skill: DiscoveredSkill;
  virtualized?: boolean;
}

function getDiscoverDetailKey(skill: DiscoveredSkill): string {
  return skill.is_already_central
    ? getPathBasename(skill.dir_path) ?? skill.id
    : skill.id;
}

function DiscoverSkillCard({
  importingIds,
  onInstallToCentral,
  onInstallToPlatform,
  onOpenDiscoverDrawer,
  onOpenDrawer,
  onSetDetailButtonRef,
  onToggleSkillSelection,
  selectedSkillIds,
  skill,
  virtualized = false,
}: DiscoverSkillCardProps) {
  const detailKey = getDiscoverDetailKey(skill);

  return (
    <UnifiedSkillCard
      name={skill.name}
      description={skill.description}
      checkbox={{
        checked: selectedSkillIds.has(skill.id),
        onChange: () => onToggleSkillSelection(skill.id),
      }}
      isCentral={skill.is_already_central}
      platformBadge={{ id: skill.platform_id, name: skill.platform_name }}
      projectBadge={skill.project_name}
      onDetail={
        skill.is_already_central
          ? () => onOpenDrawer(detailKey)
          : () => onOpenDiscoverDrawer(skill)
      }
      detailButtonRef={(node) => onSetDetailButtonRef(detailKey, node)}
      onInstallToCentral={() => onInstallToCentral(skill.id)}
      onInstallToPlatform={() => onInstallToPlatform(skill)}
      isLoading={importingIds.has(skill.id)}
      className={virtualized ? "h-[120px]" : undefined}
    />
  );
}

interface ProjectListProps {
  filteredProjectList: DiscoveredProject[];
  onProjectSearchChange: (value: string) => void;
  onSelectProject: (projectPath: string) => void;
  projectSearch: string;
  selectedProject: DiscoveredProject | null;
  selectedProjectMatchesFilter: boolean;
}

function ProjectList({
  filteredProjectList,
  onProjectSearchChange,
  onSelectProject,
  projectSearch,
  selectedProject,
  selectedProjectMatchesFilter,
}: ProjectListProps) {
  const { t } = useTranslation();

  return (
    <div className="w-60 shrink-0 border-r border-border flex flex-col">
      <div className="p-2 border-b border-border">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground pointer-events-none" />
          <Input
            placeholder={t("discover.projectSearchPlaceholder")}
            value={projectSearch}
            onChange={(event) => onProjectSearchChange(event.target.value)}
            aria-label={t("discover.projectSearchPlaceholder")}
            className="pl-7 pr-7 h-7 text-xs bg-muted/40"
          />
          {projectSearch.length > 0 && (
            <button
              type="button"
              onClick={() => onProjectSearchChange("")}
              aria-label={t("discover.clearSearch")}
              title={t("discover.clearSearch")}
              className="absolute right-1 top-1/2 -translate-y-1/2 p-0.5 rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors cursor-pointer"
            >
              <X className="size-3" />
            </button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto py-1">
        {filteredProjectList.length === 0 ? (
          <div className="flex flex-col items-center justify-center gap-2 px-3 py-6 text-center">
            <p className="text-xs text-muted-foreground">
              {t("discover.noProjectMatch", { query: projectSearch })}
            </p>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onProjectSearchChange("")}
              className="h-7 text-xs"
            >
              <X className="size-3 mr-1" />
              {t("discover.clearSearch")}
            </Button>
          </div>
        ) : (
          filteredProjectList.map((project) => {
            const isActive =
              selectedProject?.project_path === project.project_path;
            return (
              <button
                key={project.project_path}
                onClick={() => onSelectProject(project.project_path)}
                title={project.project_path}
                aria-current={isActive ? "true" : undefined}
                className={cn(
                  "flex items-center gap-2 w-full px-3 py-2 text-left transition-colors cursor-pointer border rounded-md",
                  isActive
                    ? "bg-primary/10 border-primary/60 text-foreground font-medium shadow-sm"
                    : "border-transparent text-muted-foreground hover:border-border hover:bg-muted/40"
                )}
              >
                <Folder
                  className={cn(
                    "size-3.5 shrink-0",
                    isActive ? "text-primary" : "text-muted-foreground"
                  )}
                />
                <span className="text-sm truncate flex-1">
                  {project.project_name}
                </span>
                <span className="text-[10px] font-mono tabular-nums text-muted-foreground shrink-0">
                  {project.skills.length}
                </span>
              </button>
            );
          })
        )}

        {selectedProject && !selectedProjectMatchesFilter && (
          <div className="mt-2 pt-2 px-2 border-t border-border/60 space-y-1">
            <p className="text-[10px] uppercase tracking-wide text-muted-foreground/70 px-1">
              {t("discover.title")}
            </p>
            <button
              onClick={() => onSelectProject(selectedProject.project_path)}
              title={selectedProject.project_path}
              aria-current="true"
              className="flex items-center gap-2 w-full px-3 py-2 text-left transition-colors cursor-pointer border rounded-md bg-primary/10 border-primary/60 text-foreground font-medium shadow-sm"
            >
              <Folder className="size-3.5 shrink-0 text-primary" />
              <span className="text-sm truncate flex-1">
                {selectedProject.project_name}
              </span>
              <span className="text-[10px] font-mono tabular-nums text-muted-foreground shrink-0">
                {selectedProject.skills.length}
              </span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

interface SkillPanelProps {
  contentRef: RefObject<HTMLDivElement | null>;
  displayedSkills: DiscoveredSkill[];
  importingIds: Set<string>;
  normalizedSkillQuery: string;
  onInstallToCentral: (skillId: string) => void | Promise<void>;
  onInstallToPlatform: (skill: DiscoveredSkill) => void;
  onOpenDiscoverDrawer: (skill: DiscoveredSkill) => void;
  onOpenDrawer: (skillId: string) => void;
  onOpenProjectPath: (projectPath: string) => void | Promise<void>;
  onSetDetailButtonRef: (
    skillId: string,
    node: HTMLButtonElement | null
  ) => void;
  onSkillSearchChange: (value: string) => void;
  onToggleSkillSelection: (skillId: string) => void;
  selectedProject: DiscoveredProject | null;
  selectedSkillIds: Set<string>;
  skillSearch: string;
}

function SkillPanel({
  contentRef,
  displayedSkills,
  importingIds,
  normalizedSkillQuery,
  onInstallToCentral,
  onInstallToPlatform,
  onOpenDiscoverDrawer,
  onOpenDrawer,
  onOpenProjectPath,
  onSetDetailButtonRef,
  onSkillSearchChange,
  onToggleSkillSelection,
  selectedProject,
  selectedSkillIds,
  skillSearch,
}: SkillPanelProps) {
  const { t } = useTranslation();

  if (!selectedProject) {
    return (
      <div className="flex-1 flex flex-col min-w-0">
        <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
          <Radar className="size-5 mr-2 opacity-40" />
          {t("discover.noResults")}
        </div>
      </div>
    );
  }

  const skillCardProps = {
    importingIds,
    onInstallToCentral,
    onInstallToPlatform,
    onOpenDiscoverDrawer,
    onOpenDrawer,
    onSetDetailButtonRef,
    onToggleSkillSelection,
    selectedSkillIds,
  };

  return (
    <div className="flex-1 flex flex-col min-w-0">
      <div className="px-6 py-3 border-b border-border flex items-center gap-3">
        <div className="min-w-0 flex-1">
          <h2 className="text-sm font-semibold truncate">
            {selectedProject.project_name}
          </h2>
          <button
            type="button"
            onClick={() => onOpenProjectPath(selectedProject.project_path)}
            className="text-xs text-muted-foreground truncate hover:text-primary hover:underline cursor-pointer text-left block max-w-full"
            title={t("discover.openInFileManager")}
          >
            {selectedProject.project_path}
          </button>
        </div>
        <div className="relative w-48 shrink-0">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground pointer-events-none" />
          <Input
            placeholder={t("discover.skillSearchPlaceholder")}
            value={skillSearch}
            onChange={(event) => onSkillSearchChange(event.target.value)}
            aria-label={t("discover.skillSearchPlaceholder")}
            className="pl-7 pr-7 h-7 text-xs bg-muted/40"
          />
          {skillSearch.length > 0 && (
            <button
              type="button"
              onClick={() => onSkillSearchChange("")}
              aria-label={t("discover.clearSearch")}
              title={t("discover.clearSearch")}
              className="absolute right-1 top-1/2 -translate-y-1/2 p-0.5 rounded text-muted-foreground hover:text-foreground hover:bg-muted transition-colors cursor-pointer"
            >
              <X className="size-3" />
            </button>
          )}
        </div>
        <span className="text-xs text-muted-foreground shrink-0">
          {t("collection.skills", { count: displayedSkills.length })}
        </span>
      </div>

      <div ref={contentRef} className="flex-1 overflow-auto p-4">
        {displayedSkills.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-3 py-12">
            <Radar className="size-8 text-muted-foreground opacity-40" />
            <p className="text-sm text-muted-foreground">
              {normalizedSkillQuery
                ? t("discover.noMatch", { query: skillSearch })
                : t("discover.noResults")}
            </p>
            {normalizedSkillQuery && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onSkillSearchChange("")}
                className="h-7 text-xs"
              >
                <X className="size-3 mr-1" />
                {t("discover.clearSearch")}
              </Button>
            )}
          </div>
        ) : displayedSkills.length > 80 ? (
          <VirtualizedList
            items={displayedSkills}
            itemHeight={120}
            itemGap={8}
            overscan={6}
            scrollContainerRef={contentRef}
            itemKey={(skill) => skill.id}
            renderItem={(skill) => (
              <DiscoverSkillCard
                {...skillCardProps}
                skill={skill}
                virtualized
              />
            )}
          />
        ) : (
          <div className="space-y-2">
            {displayedSkills.map((skill) => (
              <DiscoverSkillCard
                key={skill.id}
                {...skillCardProps}
                skill={skill}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

interface DiscoverShellProps {
  clearSelection: () => void;
  contentRef: RefObject<HTMLDivElement | null>;
  currentPath: string;
  detailButtonRefs: RefObject<Record<string, HTMLButtonElement | null>>;
  drawerDiscoverMeta: DiscoverMetadata | null;
  drawerFilePath: string | null;
  drawerSkillId: string | null;
  isConfigOpen: boolean;
  isDrawerOpen: boolean;
  isInstallDialogOpen: boolean;
  isRemoteTarget: boolean;
  isScanning: boolean;
  importingIds: Set<string>;
  installTargetSkill: DiscoveredSkill | null;
  onBatchInstallCentral: () => void | Promise<void>;
  onConfigOpenChange: (open: boolean) => void;
  onDrawerOpenChange: (open: boolean) => void;
  onInstallDialogOpenChange: (open: boolean) => void;
  onInstallFromDialog: (
    skillId: string,
    agentIds: string[],
    method: string
  ) => Promise<BatchInstallResult>;
  onInstallToCentral: (skillId: string) => void | Promise<void>;
  onInstallToPlatform: (skill: DiscoveredSkill) => void;
  onOpenDiscoverDrawer: (skill: DiscoveredSkill) => void;
  onOpenDrawer: (skillId: string) => void;
  onOpenProjectPath: (projectPath: string) => void | Promise<void>;
  onProjectSearchChange: (value: string) => void;
  onRescan: () => void | Promise<void>;
  onSelectProject: (projectPath: string) => void;
  onSetDetailButtonRef: (
    skillId: string,
    node: HTMLButtonElement | null
  ) => void;
  onSkillSearchChange: (value: string) => void;
  onStopScan: () => void | Promise<void>;
  onToggleSkillSelection: (skillId: string) => void;
  platformAgents: AgentWithStatus[];
  projectCount: number;
  projectSearch: string;
  projectsFoundSoFar: number;
  scanProgress: number;
  selectedSkillIds: Set<string>;
  skillSearch: string;
  skillsFoundSoFar: number;
  totalSkillsFound: number;
  viewModel: DiscoverViewModel;
}

export function DiscoverShell({
  clearSelection,
  contentRef,
  currentPath,
  detailButtonRefs,
  drawerDiscoverMeta,
  drawerFilePath,
  drawerSkillId,
  isConfigOpen,
  isDrawerOpen,
  isInstallDialogOpen,
  isRemoteTarget,
  isScanning,
  importingIds,
  installTargetSkill,
  onBatchInstallCentral,
  onConfigOpenChange,
  onDrawerOpenChange,
  onInstallDialogOpenChange,
  onInstallFromDialog,
  onInstallToCentral,
  onInstallToPlatform,
  onOpenDiscoverDrawer,
  onOpenDrawer,
  onOpenProjectPath,
  onProjectSearchChange,
  onRescan,
  onSelectProject,
  onSetDetailButtonRef,
  onSkillSearchChange,
  onStopScan,
  onToggleSkillSelection,
  platformAgents,
  projectCount,
  projectSearch,
  projectsFoundSoFar,
  scanProgress,
  selectedSkillIds,
  skillSearch,
  skillsFoundSoFar,
  totalSkillsFound,
  viewModel,
}: DiscoverShellProps) {
  const { t } = useTranslation();

  if (isScanning) {
    return (
      <div className="flex flex-col h-full">
        <div className="border-b border-border px-6 py-4">
          <h1 className="text-xl font-semibold">
            {t("discover.resultsTitle")}
          </h1>
        </div>
        <div className="flex-1 overflow-auto p-6">
          <DiscoverProgressView
            currentPath={currentPath}
            onStopScan={onStopScan}
            projectsFoundSoFar={projectsFoundSoFar}
            scanProgress={scanProgress}
            skillsFoundSoFar={skillsFoundSoFar}
          />
        </div>
        <DiscoverConfigDialog
          open={isConfigOpen}
          onOpenChange={onConfigOpenChange}
        />
      </div>
    );
  }

  if (projectCount === 0) {
    return (
      <div className="flex flex-col h-full">
        <div className="border-b border-border px-6 py-4 flex items-center justify-between">
          <h1 className="text-xl font-semibold">
            {t("discover.resultsTitle")}
          </h1>
          <Button
            variant="outline"
            size="sm"
            disabled={isRemoteTarget}
            title={isRemoteTarget ? t("targets.discoverUnsupported") : undefined}
            onClick={onRescan}
          >
            <RefreshCw className="size-3.5 mr-1" />
            {t("discover.reScan")}
          </Button>
        </div>
        <div className="flex-1">
          <DiscoverEmptyState />
        </div>
        <DiscoverConfigDialog
          open={isConfigOpen}
          onOpenChange={onConfigOpenChange}
        />
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-border px-6 py-4 flex items-center justify-between gap-4">
        <div>
          <h1 className="text-xl font-semibold">
            {t("discover.resultsTitle")}
          </h1>
          <p className="text-sm text-muted-foreground mt-0.5">
            {t("discover.foundSummary", {
              skills: totalSkillsFound,
              projects: projectCount,
            })}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={isRemoteTarget}
          title={isRemoteTarget ? t("targets.discoverUnsupported") : undefined}
          onClick={onRescan}
        >
          <RefreshCw className="size-3.5 mr-1" />
          {t("discover.reScan")}
        </Button>
      </div>

      <div className="flex flex-1 min-h-0">
        <ProjectList
          filteredProjectList={viewModel.filteredProjectList}
          onProjectSearchChange={onProjectSearchChange}
          onSelectProject={onSelectProject}
          projectSearch={projectSearch}
          selectedProject={viewModel.selectedProject}
          selectedProjectMatchesFilter={viewModel.selectedProjectMatchesFilter}
        />
        <SkillPanel
          contentRef={contentRef}
          displayedSkills={viewModel.displayedSkills}
          importingIds={importingIds}
          normalizedSkillQuery={viewModel.normalizedSkillQuery}
          onInstallToCentral={onInstallToCentral}
          onInstallToPlatform={onInstallToPlatform}
          onOpenDiscoverDrawer={onOpenDiscoverDrawer}
          onOpenDrawer={onOpenDrawer}
          onOpenProjectPath={onOpenProjectPath}
          onSetDetailButtonRef={onSetDetailButtonRef}
          onSkillSearchChange={onSkillSearchChange}
          onToggleSkillSelection={onToggleSkillSelection}
          selectedProject={viewModel.selectedProject}
          selectedSkillIds={selectedSkillIds}
          skillSearch={skillSearch}
        />
      </div>

      {selectedSkillIds.size > 0 && (
        <div className="border-t border-border px-6 py-3 flex items-center gap-3 bg-muted/20">
          <span className="text-sm text-muted-foreground">
            {t("discover.selected", { count: selectedSkillIds.size })}
          </span>
          <div className="flex items-center gap-2 ml-auto">
            <Button variant="outline" size="sm" onClick={onBatchInstallCentral}>
              <ArrowUpRight className="size-3.5 mr-1" />
              {t("discover.installSelectedCentral")}
            </Button>
            <Button variant="ghost" size="sm" onClick={clearSelection}>
              {t("discover.deselectAll")}
            </Button>
          </div>
        </div>
      )}

      <DiscoverConfigDialog
        open={isConfigOpen}
        onOpenChange={onConfigOpenChange}
      />

      {installTargetSkill && (
        <InstallDialog
          open={isInstallDialogOpen}
          onOpenChange={onInstallDialogOpenChange}
          skill={{
            id: installTargetSkill.id,
            name: installTargetSkill.name,
            description: installTargetSkill.description,
            file_path: installTargetSkill.file_path,
            is_central: false,
            linked_agents: [],
            shared_root_agents: [],
            scanned_at: new Date().toISOString(),
          } as SkillWithLinks}
          agents={platformAgents}
          onInstall={onInstallFromDialog}
        />
      )}

      <SkillDetailDrawer
        open={isDrawerOpen}
        skillId={drawerSkillId}
        filePath={drawerFilePath}
        discoverMetadata={drawerDiscoverMeta}
        onOpenChange={onDrawerOpenChange}
        returnFocusRef={
          drawerSkillId || drawerFilePath
            ? {
                current: detailButtonRefs.current[drawerSkillId ?? ""] ?? null,
              }
            : undefined
        }
      />
    </div>
  );
}
