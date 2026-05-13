import { create } from "zustand";
import { createMarketplaceBaseState } from "./marketplaceStore.shared";
import { createMarketplaceGitHubImportSlice } from "./marketplaceStore.githubImportSlice";
import { createMarketplaceRegistrySlice } from "./marketplaceStore.registrySlice";
import type { MarketplaceState, MarketplaceStoreContext } from "./marketplaceStore.types";

export type {
  GitHubImportAiSummaryEntry,
  SkillMarkdownEntry,
} from "./marketplaceStore.types";

let marketplaceStoreGeneration = 0;

export const useMarketplaceStore = create<MarketplaceState>((set, get) => {
  const context: MarketplaceStoreContext = {
    set,
    get,
    getGeneration: () => marketplaceStoreGeneration,
    bumpGeneration: () => {
      marketplaceStoreGeneration += 1;
    },
  };

  return {
    ...createMarketplaceBaseState(),
    ...createMarketplaceRegistrySlice(context),
    ...createMarketplaceGitHubImportSlice(context),
  };
});
