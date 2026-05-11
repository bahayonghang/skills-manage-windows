import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillStore } from "@/stores/skillStore";

export function useMarketplaceBindings() {
  const registries = useMarketplaceStore((state) => state.registries);
  const installingIds = useMarketplaceStore((state) => state.installingIds);
  const loadRegistries = useMarketplaceStore((state) => state.loadRegistries);
  const loadPreviewSkills = useMarketplaceStore((state) => state.loadPreviewSkills);
  const installSkill = useMarketplaceStore((state) => state.installSkill);
  const getNormalizedRegistryIdentity = useMarketplaceStore(
    (state) => state.getNormalizedRegistryIdentity
  );
  const previewGitHubRepoSkills = useMarketplaceStore(
    (state) => state.previewGitHubRepoSkills
  );
  const githubImport = useMarketplaceStore((state) => state.githubImport);
  const previewGitHubRepoImport = useMarketplaceStore(
    (state) => state.previewGitHubRepoImport
  );
  const importGitHubRepoSkills = useMarketplaceStore(
    (state) => state.importGitHubRepoSkills
  );
  const resetGitHubImport = useMarketplaceStore((state) => state.resetGitHubImport);

  const rescan = usePlatformStore((state) => state.rescan);
  const platformAgents = usePlatformStore((state) => state.agents);

  const centralSkills = useCentralSkillsStore((state) => state.skills);
  const centralAgents = useCentralSkillsStore((state) => state.agents);
  const loadCentralSkills = useCentralSkillsStore((state) => state.loadCentralSkills);
  const installCentralSkill = useCentralSkillsStore((state) => state.installSkill);

  const skillsByAgent = useSkillStore((state) => state.skillsByAgent);
  const getSkillsByAgent = useSkillStore((state) => state.getSkillsByAgent);

  return {
    registries,
    installingIds,
    loadRegistries,
    loadPreviewSkills,
    installSkill,
    getNormalizedRegistryIdentity,
    previewGitHubRepoSkills,
    githubImport,
    previewGitHubRepoImport,
    importGitHubRepoSkills,
    resetGitHubImport,
    rescan,
    platformAgents,
    centralSkills,
    centralAgents,
    loadCentralSkills,
    installCentralSkill,
    skillsByAgent,
    getSkillsByAgent,
  };
}
