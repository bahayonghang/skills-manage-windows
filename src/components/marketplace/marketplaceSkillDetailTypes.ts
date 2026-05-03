export interface MarketplaceSkillDetail {
  id: string;
  name: string;
  downloadUrl: string;
  description?: string;
  publisher?: string;
  sourceLabel?: string;
  sourceUrl?: string | null;
  installed?: boolean;
}

export type MarketplaceDetailViewMode = "markdown" | "raw";
