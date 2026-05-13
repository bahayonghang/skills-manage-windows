import { useCallback, useDeferredValue, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate, useParams } from "react-router-dom";

import { DiscoverShell } from "@/components/discover/DiscoverShell";
import { consumeScrollPosition } from "@/lib/scrollRestoration";
import { useDiscoverBindings } from "@/pages/discoverBindings";
import {
  type DiscoverContext,
  type ScrollRestorationState,
  useDiscoverViewModel,
} from "@/pages/discoverViewModel";
import type { DiscoverMetadata } from "@/components/skill/SkillDetailView";
import type { BatchInstallResult, DiscoveredSkill } from "@/types";

export function DiscoverView() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const { projectPath } = useParams<{ projectPath: string }>();
  const {
    agents,
    categoryVisibility,
    clearSelection,
    currentPath,
    discoveredProjects,
    importToCentral,
    importToPlatform,
    isRemoteTarget,
    isScanning,
    loadDiscoveredSkills,
    loadScanRoots,
    openPathInFileManager,
    projectsFoundSoFar,
    refreshCounts,
    refreshDiscoverCounts,
    scanProgress,
    selectedSkillIds,
    skillsFoundSoFar,
    stopScan,
    toggleSkillSelection,
    totalSkillsFound,
  } = useDiscoverBindings();
  const contentRef = useRef<HTMLDivElement | null>(null);
  const detailButtonRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  const [isConfigOpen, setIsConfigOpen] = useState(false);
  const [importingIds, setImportingIds] = useState<Set<string>>(new Set());
  const [installTargetSkill, setInstallTargetSkill] =
    useState<DiscoveredSkill | null>(null);
  const [isInstallDialogOpen, setIsInstallDialogOpen] = useState(false);
  const [drawerSkillId, setDrawerSkillId] = useState<string | null>(null);
  const [drawerFilePath, setDrawerFilePath] = useState<string | null>(null);
  const [drawerDiscoverMeta, setDrawerDiscoverMeta] =
    useState<DiscoverMetadata | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [projectSearch, setProjectSearch] = useState("");
  const [skillSearch, setSkillSearch] = useState("");
  const deferredSkillSearch = useDeferredValue(skillSearch);

  const restorationState = location.state?.scrollRestoration as
    | ScrollRestorationState
    | undefined;
  const discoverContext = location.state?.discoverContext as
    | DiscoverContext
    | undefined;

  const viewModel = useDiscoverViewModel({
    agents: agents,
    categoryVisibility: categoryVisibility,
    deferredSkillSearch,
    discoveredProjects: discoveredProjects,
    projectPath,
    projectSearch,
    skillSearch,
  });

  useEffect(() => {
    loadDiscoveredSkills();
  }, [loadDiscoveredSkills]);

  useEffect(() => {
    if (discoverContext?.skillSearch !== undefined) {
      setSkillSearch(discoverContext.skillSearch);
    }
  }, [discoverContext?.skillSearch]);

  useEffect(() => {
    if (
      !projectPath &&
      discoveredProjects.length > 0 &&
      !discoverContext?.projectPath
    ) {
      navigate(
        `/discover/${encodeURIComponent(discoveredProjects[0].project_path)}`,
        { replace: true }
      );
    }
  }, [
    projectPath,
    discoveredProjects,
    navigate,
    discoverContext?.projectPath,
  ]);

  useEffect(() => {
    if (!projectPath || !discoverContext?.projectPath) {
      return;
    }

    const decodedPath = decodeURIComponent(projectPath);
    if (decodedPath === discoverContext.projectPath) {
      return;
    }

    navigate(`/discover/${encodeURIComponent(discoverContext.projectPath)}`, {
      replace: true,
      state: location.state,
    });
  }, [projectPath, discoverContext?.projectPath, navigate, location.state]);

  useEffect(() => {
    if (!contentRef.current) return;
    contentRef.current.scrollTop = 0;
  }, [viewModel.normalizedSkillQuery, viewModel.selectedProject?.project_path]);

  useEffect(() => {
    if (
      !viewModel.selectedProject ||
      !restorationState?.key ||
      !contentRef.current
    ) {
      return;
    }

    // Prefer the in-memory map (populated by SkillDetail's back handler on the
    // real list -> detail -> back flow). Fall back to the scroll position that
    // was passed directly via location.state so restoration still works when the
    // list is hydrated with state intact.
    let scrollTop = consumeScrollPosition(restorationState.key);
    if (scrollTop === null && typeof restorationState.scrollTop === "number") {
      scrollTop = restorationState.scrollTop;
    }
    if (scrollTop === null) {
      return;
    }

    contentRef.current.scrollTop = scrollTop;
  }, [
    viewModel.selectedProject,
    viewModel.displayedSkills.length,
    restorationState?.key,
    restorationState?.scrollTop,
  ]);

  const handleInstallToCentral = useCallback(
    async (skillId: string) => {
      setImportingIds((prev) => new Set(prev).add(skillId));
      try {
        await importToCentral(skillId);
        await Promise.all([
          refreshCounts(),
          refreshDiscoverCounts(),
        ]);
        toast.success(t("discover.importSuccess"));
      } catch (err) {
        toast.error(t("discover.importError", { error: String(err) }));
      } finally {
        setImportingIds((prev) => {
          const next = new Set(prev);
          next.delete(skillId);
          return next;
        });
      }
    },
    [
      importToCentral,
      refreshCounts,
      refreshDiscoverCounts,
      t,
    ]
  );

  const handleInstallToPlatform = useCallback((skill: DiscoveredSkill) => {
    setInstallTargetSkill(skill);
    setIsInstallDialogOpen(true);
  }, []);

  const handleBatchInstallCentral = useCallback(async () => {
    for (const id of Array.from(selectedSkillIds)) {
      await handleInstallToCentral(id);
    }
  }, [selectedSkillIds, handleInstallToCentral]);

  const handleInstallFromDialog = useCallback(
    async (
      _skillId: string,
      agentIds: string[],
      method: string
    ): Promise<BatchInstallResult> => {
      if (!installTargetSkill) {
        return { succeeded: [], failed: [] };
      }

      const targetId = installTargetSkill.id;
      setImportingIds((prev) => new Set(prev).add(targetId));
      try {
        const succeeded: string[] = [];
        const failed: BatchInstallResult["failed"] = [];
        for (const agentId of agentIds) {
          try {
            await importToPlatform(
              targetId,
              agentId,
              method === "copy" ? "copy" : "symlink"
            );
            succeeded.push(agentId);
          } catch (err) {
            failed.push({ agent_id: agentId, error: String(err) });
          }
        }

        if (succeeded.length > 0) {
          await Promise.all([
            refreshCounts(),
            refreshDiscoverCounts(),
          ]);
        }
        if (failed.length > 0) {
          const failedNames = failed
            .map((failure) => `${failure.agent_id}: ${failure.error}`)
            .join("; ");
          toast.error(
            t("central.installPartialFail", { platforms: failedNames })
          );
        } else {
          toast.success(t("discover.importSuccess"));
        }
        return { succeeded, failed };
      } catch (err) {
        toast.error(t("discover.importError", { error: String(err) }));
        throw err;
      } finally {
        setImportingIds((prev) => {
          const next = new Set(prev);
          next.delete(targetId);
          return next;
        });
      }
    },
    [
      installTargetSkill,
      importToPlatform,
      refreshCounts,
      refreshDiscoverCounts,
      t,
    ]
  );

  const handleRescan = useCallback(async () => {
    if (isRemoteTarget) {
      toast.error(t("targets.discoverUnsupported"));
      return;
    }
    await loadScanRoots();
    setIsConfigOpen(true);
  }, [isRemoteTarget, loadScanRoots, t]);

  const handleSelectProject = useCallback(
    (projectPathValue: string) => {
      if (projectPath === projectPathValue) return;
      navigate(`/discover/${encodeURIComponent(projectPathValue)}`);
    },
    [navigate, projectPath]
  );

  const setDetailButtonRef = useCallback(
    (skillId: string, node: HTMLButtonElement | null) => {
      detailButtonRefs.current[skillId] = node;
    },
    []
  );

  const handleOpenDrawer = useCallback((skillId: string) => {
    setDrawerSkillId(skillId);
    setDrawerFilePath(null);
    setDrawerDiscoverMeta(null);
    setIsDrawerOpen(true);
  }, []);

  const handleOpenDiscoverDrawer = useCallback((skill: DiscoveredSkill) => {
    setDrawerSkillId(null);
    setDrawerFilePath(skill.file_path);
    setDrawerDiscoverMeta({
      name: skill.name,
      description: skill.description,
      platformName: skill.platform_name,
      projectName: skill.project_name,
      filePath: skill.file_path,
      dirPath: skill.dir_path,
      isAlreadyCentral: skill.is_already_central,
    });
    setIsDrawerOpen(true);
  }, []);

  const handleOpenProjectPath = useCallback(
    async (path: string) => {
      try {
        if (isRemoteTarget) {
          await navigator.clipboard.writeText(path);
          toast.success(t("targets.pathCopied"));
          return;
        }
        await openPathInFileManager?.(path);
      } catch (err) {
        toast.error(t("discover.openPathError", { error: String(err) }));
      }
    },
    [isRemoteTarget, openPathInFileManager, t]
  );

  const handleInstallDialogOpenChange = useCallback((open: boolean) => {
    setIsInstallDialogOpen(open);
    if (!open) setInstallTargetSkill(null);
  }, []);

  const handleDrawerOpenChange = useCallback((open: boolean) => {
    setIsDrawerOpen(open);
    if (!open) {
      setDrawerSkillId(null);
      setDrawerFilePath(null);
      setDrawerDiscoverMeta(null);
    }
  }, []);

  return (
    <DiscoverShell
      clearSelection={clearSelection}
      contentRef={contentRef}
      currentPath={currentPath}
      detailButtonRefs={detailButtonRefs}
      drawerDiscoverMeta={drawerDiscoverMeta}
      drawerFilePath={drawerFilePath}
      drawerSkillId={drawerSkillId}
      isConfigOpen={isConfigOpen}
      isDrawerOpen={isDrawerOpen}
      isInstallDialogOpen={isInstallDialogOpen}
      isRemoteTarget={isRemoteTarget}
      isScanning={isScanning}
      importingIds={importingIds}
      installTargetSkill={installTargetSkill}
      onBatchInstallCentral={handleBatchInstallCentral}
      onConfigOpenChange={setIsConfigOpen}
      onDrawerOpenChange={handleDrawerOpenChange}
      onInstallDialogOpenChange={handleInstallDialogOpenChange}
      onInstallFromDialog={handleInstallFromDialog}
      onInstallToCentral={handleInstallToCentral}
      onInstallToPlatform={handleInstallToPlatform}
      onOpenDiscoverDrawer={handleOpenDiscoverDrawer}
      onOpenDrawer={handleOpenDrawer}
      onOpenProjectPath={handleOpenProjectPath}
      onProjectSearchChange={setProjectSearch}
      onRescan={handleRescan}
      onSelectProject={handleSelectProject}
      onSetDetailButtonRef={setDetailButtonRef}
      onSkillSearchChange={setSkillSearch}
      onStopScan={stopScan}
      onToggleSkillSelection={toggleSkillSelection}
      platformAgents={viewModel.platformAgents}
      projectCount={discoveredProjects.length}
      projectSearch={projectSearch}
      projectsFoundSoFar={projectsFoundSoFar}
      scanProgress={scanProgress}
      selectedSkillIds={selectedSkillIds}
      skillSearch={skillSearch}
      skillsFoundSoFar={skillsFoundSoFar}
      totalSkillsFound={totalSkillsFound}
      viewModel={viewModel}
    />
  );
}
