import { create } from "zustand";

import { invoke } from "@/lib/ipc";
import {
  nextRecentSources,
  parseRecentSourcesSetting,
  SKILLS_CLI_RECENT_SOURCES_PARSE_ERROR,
  SKILLS_CLI_RECENT_SOURCES_SETTING_KEY,
} from "@/pages/skillsCliInstallViewModel";

export interface SkillsCliRecentSourcesState {
  sources: string[];
  isLoading: boolean;
  error: unknown | null;
  loaded: boolean;
  load: () => Promise<void>;
  push: (source: string) => Promise<void>;
  reset: () => void;
}

const emptyState = {
  sources: [] as string[],
  isLoading: false,
  error: null as unknown | null,
  loaded: false,
};

export const useSkillsCliRecentSourcesStore = create<SkillsCliRecentSourcesState>(
  (set, get) => ({
    ...emptyState,

    async load() {
      set({ isLoading: true, error: null });
      try {
        const raw = await invoke("get_setting", {
          key: SKILLS_CLI_RECENT_SOURCES_SETTING_KEY,
        });
        const parsed = parseRecentSourcesSetting(raw);
        if (!parsed.ok) {
          set({
            sources: [],
            isLoading: false,
            loaded: true,
            error: SKILLS_CLI_RECENT_SOURCES_PARSE_ERROR,
          });
          return;
        }
        set({
          sources: parsed.sources,
          isLoading: false,
          loaded: true,
          error: null,
        });
      } catch (error) {
        set({
          sources: [],
          isLoading: false,
          loaded: true,
          error,
        });
      }
    },

    async push(source) {
      const trimmed = source.trim();
      if (trimmed === "") {
        return;
      }
      const next = nextRecentSources(get().sources, trimmed);
      await invoke("set_setting", {
        key: SKILLS_CLI_RECENT_SOURCES_SETTING_KEY,
        value: JSON.stringify(next),
      });
      set({ sources: next, error: null });
    },

    reset() {
      set({ ...emptyState });
    },
  }),
);
