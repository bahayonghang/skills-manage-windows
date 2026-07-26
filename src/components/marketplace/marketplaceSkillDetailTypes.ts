export interface MarketplaceSkillDetail {
  id: string;
  name: string;
  downloadUrl: string;
  description?: string;
  publisher?: string;
  sourceLabel?: string;
  sourceUrl?: string | null;
  installed?: boolean;
  source?: string;
  skillId?: string;
  remoteKind?: "skills_sh";
  installs?: number;
  stars?: number | null;
  previewInstall?:
    | { sourceKind: "registry"; registrySkillId: string }
    | { sourceKind: "github"; repoUrl: string; sourcePath: string };
}

export type MarketplaceDetailViewMode = "markdown" | "raw";
