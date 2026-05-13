import { create } from "zustand";
import { createCentralSkillsInitialState } from "./centralSkillsStore.shared";
import { createCentralInstallSlice } from "./centralSkillsStore.installSlice";
import { createCentralListSlice } from "./centralSkillsStore.listSlice";
import { createCentralMetadataSlice } from "./centralSkillsStore.metadataSlice";
import { createCentralUpdateSlice } from "./centralSkillsStore.updateSlice";
import type { CentralSkillsState, CentralStoreContext } from "./centralSkillsStore.types";

let centralStoreGeneration = 0;

export const useCentralSkillsStore = create<CentralSkillsState>((set, get) => {
  const context: CentralStoreContext = {
    set,
    get,
    getGeneration: () => centralStoreGeneration,
    bumpGeneration: () => {
      centralStoreGeneration += 1;
    },
  };

  return {
    ...createCentralSkillsInitialState(),
    ...createCentralListSlice(context),
    ...createCentralInstallSlice(context),
    ...createCentralMetadataSlice(context),
    ...createCentralUpdateSlice(context),
  };
});
