import { useDiscoverStore } from "@/stores/discoverStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
} from "@/lib/platformVisibility";

export function useDiscoverBindings() {
  const isScanning = useDiscoverStore((state) => state.isScanning);
  const discoveredProjects = useDiscoverStore((state) => state.discoveredProjects);
  const totalSkillsFound = useDiscoverStore((state) => state.totalSkillsFound);
  const selectedSkillIds = useDiscoverStore((state) => state.selectedSkillIds);
  const scanProgress = useDiscoverStore((state) => state.scanProgress);
  const currentPath = useDiscoverStore((state) => state.currentPath);
  const skillsFoundSoFar = useDiscoverStore((state) => state.skillsFoundSoFar);
  const projectsFoundSoFar = useDiscoverStore((state) => state.projectsFoundSoFar);

  const loadDiscoveredSkills = useDiscoverStore(
    (state) => state.loadDiscoveredSkills
  );
  const refreshDiscoverCounts = useDiscoverStore((state) => state.refreshCounts);
  const importToCentral = useDiscoverStore((state) => state.importToCentral);
  const importToPlatform = useDiscoverStore((state) => state.importToPlatform);
  const openPathInFileManager = useDiscoverStore(
    (state) => state.openPathInFileManager
  );
  const toggleSkillSelection = useDiscoverStore(
    (state) => state.toggleSkillSelection
  );
  const clearSelection = useDiscoverStore((state) => state.clearSelection);
  const loadScanRoots = useDiscoverStore((state) => state.loadScanRoots);
  const stopScan = useDiscoverStore((state) => state.stopScan);

  const agents = usePlatformStore((state) => state.agents);
  const categoryVisibility =
    usePlatformStore((state) => state.categoryVisibility) ??
    DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const refreshCounts = usePlatformStore((state) => state.refreshCounts);

  const activeTarget = useTargetStore((state) => state.activeTarget);
  const isRemoteTarget = activeTarget.kind === "ssh";

  return {
    isScanning,
    discoveredProjects,
    totalSkillsFound,
    selectedSkillIds,
    scanProgress,
    currentPath,
    skillsFoundSoFar,
    projectsFoundSoFar,
    loadDiscoveredSkills,
    refreshDiscoverCounts,
    importToCentral,
    importToPlatform,
    openPathInFileManager,
    toggleSkillSelection,
    clearSelection,
    loadScanRoots,
    stopScan,
    agents,
    categoryVisibility,
    refreshCounts,
    isRemoteTarget,
  };
}
