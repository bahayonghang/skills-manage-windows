import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { useNavigate, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { ProjectInstallDialog } from "@/components/projects/ProjectInstallDialog";
import { ProjectRemoveDialog } from "@/components/projects/ProjectRemoveDialog";
import { ProjectRenameDialog } from "@/components/projects/ProjectRenameDialog";
import { ProjectsShell } from "@/components/projects/ProjectsShell";
import { DiscoverDeprecationBanner } from "@/components/projects/DiscoverDeprecationBanner";
import { usePlatformStore } from "@/stores/platformStore";
import { useProjectsStore } from "@/stores/projectsStore";
import { useSkillStore } from "@/stores/skillStore";
import type { Project, ProjectSkill, SkillWithLinks } from "@/types";

export function ProjectsView() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { projectId } = useParams<{ projectId?: string }>();

  const projects = useProjectsStore((s) => s.projects);
  const skillsByProject = useProjectsStore((s) => s.skillsByProject);
  const scanningProjectIds = useProjectsStore((s) => s.scanningProjectIds);
  const loadProjects = useProjectsStore((s) => s.loadProjects);
  const pickProjectFolder = useProjectsStore((s) => s.pickProjectFolder);
  const addProject = useProjectsStore((s) => s.addProject);
  const rescanProject = useProjectsStore((s) => s.rescanProject);
  const getProjectSkills = useProjectsStore((s) => s.getProjectSkills);
  const installSkillToProject = useProjectsStore(
    (s) => s.installSkillToProject
  );
  const uninstallSkillFromProject = useProjectsStore(
    (s) => s.uninstallSkillFromProject
  );
  const setPinned = useProjectsStore((s) => s.setPinned);
  const renameProjectAction = useProjectsStore((s) => s.renameProject);
  const removeProjectAction = useProjectsStore((s) => s.removeProject);
  const setCurrentProjectId = useProjectsStore((s) => s.setCurrentProjectId);
  const currentProjectId = useProjectsStore((s) => s.currentProjectId);
  const ensureEventListener = useProjectsStore((s) => s.ensureEventListener);
  const fetchCentralSkillsList = useSkillStore((s) => s.fetchCentralSkillsList);
  const agents = usePlatformStore((s) => s.agents);

  const [projectSearch, setProjectSearch] = useState("");
  const [isAddingProject, setIsAddingProject] = useState(false);
  const [isInstallOpen, setIsInstallOpen] = useState(false);
  const [centralSkills, setCentralSkills] = useState<SkillWithLinks[]>([]);
  const [isInstalling, setIsInstalling] = useState(false);
  const [uninstallingKeys, setUninstallingKeys] = useState<Set<string>>(
    new Set()
  );
  const [removeTarget, setRemoveTarget] = useState<Project | null>(null);
  const [isRemoving, setIsRemoving] = useState(false);
  const [renameTarget, setRenameTarget] = useState<Project | null>(null);
  const [isRenaming, setIsRenaming] = useState(false);

  useEffect(() => {
    void loadProjects();
    void ensureEventListener();
  }, [loadProjects, ensureEventListener]);

  useEffect(() => {
    if (projectId && projectId !== currentProjectId) {
      setCurrentProjectId(projectId);
    }
  }, [projectId, currentProjectId, setCurrentProjectId]);

  useEffect(() => {
    if (!currentProjectId) return;
    void getProjectSkills(currentProjectId);
  }, [currentProjectId, getProjectSkills, scanningProjectIds]);

  const currentSkills = useMemo(
    () => (currentProjectId ? skillsByProject[currentProjectId] ?? [] : []),
    [currentProjectId, skillsByProject]
  );

  const currentProject = useMemo(
    () => projects.find((p) => p.id === currentProjectId) ?? null,
    [projects, currentProjectId]
  );

  const handleSelectProject = useCallback(
    (id: string) => {
      navigate(`/projects/${encodeURIComponent(id)}`);
    },
    [navigate]
  );

  const handleAddProject = useCallback(async () => {
    setIsAddingProject(true);
    try {
      const path = await pickProjectFolder();
      if (!path) return;
      const project = await addProject(path);
      if (project) {
        toast.success(t("projects.addSuccess", { name: project.name }));
        navigate(`/projects/${encodeURIComponent(project.id)}`);
      }
    } catch (err) {
      toast.error(t("projects.addError", { error: String(err) }));
    } finally {
      setIsAddingProject(false);
    }
  }, [pickProjectFolder, addProject, navigate, t]);

  const handleRescanProject = useCallback(
    async (id: string) => {
      try {
        await rescanProject(id);
        await getProjectSkills(id);
        toast.success(t("projects.rescanSuccess"));
      } catch (err) {
        toast.error(t("projects.rescanError", { error: String(err) }));
      }
    },
    [rescanProject, getProjectSkills, t]
  );

  const handleOpenInstallDialog = useCallback(async () => {
    if (!currentProject) return;
    try {
      const list = await fetchCentralSkillsList();
      setCentralSkills(list);
      setIsInstallOpen(true);
    } catch (err) {
      toast.error(t("projects.loadCentralError", { error: String(err) }));
    }
  }, [currentProject, fetchCentralSkillsList, t]);

  const handleConfirmInstall = useCallback(
    async (
      skillId: string,
      agentIds: string[],
      method: "symlink" | "copy"
    ) => {
      if (!currentProject) return;
      setIsInstalling(true);
      try {
        const failures: string[] = [];
        for (const agentId of agentIds) {
          try {
            await installSkillToProject(
              currentProject.id,
              skillId,
              agentId,
              method
            );
          } catch (err) {
            failures.push(`${agentId}: ${String(err)}`);
          }
        }
        if (failures.length === 0) {
          toast.success(
            t("projectInstall.installedSummary", {
              skill: skillId,
              count: agentIds.length,
            })
          );
        } else {
          toast.error(
            t("projectInstall.installedPartial", {
              failed: failures.join("; "),
            })
          );
        }
      } finally {
        setIsInstalling(false);
      }
    },
    [currentProject, installSkillToProject, t]
  );

  const handleUninstallSkill = useCallback(
    async (skill: ProjectSkill) => {
      if (!currentProject) return;
      const key = `${skill.agentId}:${skill.skillId}`;
      setUninstallingKeys((prev) => new Set(prev).add(key));
      try {
        await uninstallSkillFromProject(
          currentProject.id,
          skill.skillId,
          skill.agentId
        );
        toast.success(
          t("projects.uninstallSuccess", {
            skill: skill.name,
            agent: skill.agentDisplayName,
          })
        );
      } catch (err) {
        toast.error(
          t("projects.uninstallError", {
            skill: skill.name,
            error: String(err),
          })
        );
      } finally {
        setUninstallingKeys((prev) => {
          const next = new Set(prev);
          next.delete(key);
          return next;
        });
      }
    },
    [currentProject, uninstallSkillFromProject, t]
  );

  const handleTogglePin = useCallback(
    async (project: Project) => {
      try {
        await setPinned(project.id, !project.pinned);
      } catch (err) {
        toast.error(String(err));
      }
    },
    [setPinned]
  );

  const handleRequestRename = useCallback((project: Project) => {
    setRenameTarget(project);
  }, []);

  const handleConfirmRename = useCallback(
    async (name: string) => {
      if (!renameTarget) return;
      setIsRenaming(true);
      try {
        await renameProjectAction(renameTarget.id, name);
        toast.success(t("projects.renameSuccess"));
        setRenameTarget(null);
      } catch (err) {
        toast.error(t("projects.renameError", { error: String(err) }));
      } finally {
        setIsRenaming(false);
      }
    },
    [renameTarget, renameProjectAction, t]
  );

  const handleRequestRemove = useCallback((project: Project) => {
    setRemoveTarget(project);
  }, []);

  const handleConfirmRemove = useCallback(
    async (uninstallSkills: boolean) => {
      if (!removeTarget) return;
      const target = removeTarget;
      setIsRemoving(true);
      try {
        await removeProjectAction(target.id, uninstallSkills);
        toast.success(t("projects.removeSuccess", { name: target.name }));
        setRemoveTarget(null);
        if (currentProjectId === target.id) {
          navigate("/projects");
        }
      } catch (err) {
        toast.error(t("projects.removeError", { error: String(err) }));
      } finally {
        setIsRemoving(false);
      }
    },
    [removeTarget, removeProjectAction, currentProjectId, navigate, t]
  );

  return (
    <div className="flex h-full flex-col min-h-0">
      <DiscoverDeprecationBanner />
      <div className="flex-1 min-h-0 overflow-hidden">
        <ProjectsShell
          projects={projects}
          currentProjectId={currentProjectId}
          skills={currentSkills}
          isAddingProject={isAddingProject}
          scanningProjectIds={scanningProjectIds}
          uninstallingKeys={uninstallingKeys}
          projectSearch={projectSearch}
          onProjectSearchChange={setProjectSearch}
          onSelectProject={handleSelectProject}
          onAddProject={handleAddProject}
          onRescanProject={handleRescanProject}
          onOpenInstallDialog={handleOpenInstallDialog}
          onUninstallSkill={handleUninstallSkill}
          onTogglePin={handleTogglePin}
          onRequestRename={handleRequestRename}
          onRequestRemove={handleRequestRemove}
        />
      </div>
      <ProjectInstallDialog
        open={isInstallOpen}
        onOpenChange={setIsInstallOpen}
        project={currentProject}
        centralSkills={centralSkills}
        agents={agents}
        existingSkills={currentSkills}
        isInstalling={isInstalling}
        onConfirm={handleConfirmInstall}
      />
      <ProjectRenameDialog
        open={renameTarget !== null}
        onOpenChange={(open) => {
          if (!open) setRenameTarget(null);
        }}
        project={renameTarget}
        isRenaming={isRenaming}
        onConfirm={handleConfirmRename}
      />
      <ProjectRemoveDialog
        open={removeTarget !== null}
        onOpenChange={(open) => {
          if (!open) setRemoveTarget(null);
        }}
        project={removeTarget}
        isRemoving={isRemoving}
        onConfirm={handleConfirmRemove}
      />
    </div>
  );
}
