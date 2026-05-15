import { invoke, isTauriRuntime } from "@/lib/tauri";
import type { SkillsShFileEntry, SkillsShSkill } from "@/types";
import type { MarketplaceState, MarketplaceStoreContext } from "./marketplaceStore.types";

const DESKTOP_ONLY_ERROR =
  "Desktop-only feature: skills.sh Marketplace is available in the Tauri app.";

export function createMarketplaceSkillsShSlice({
  set,
  getGeneration,
}: MarketplaceStoreContext): Pick<
  MarketplaceState,
  | "searchSkillsSh"
  | "resolveSkillsShUrl"
  | "browseSkillsShDirectory"
  | "readSkillsShFile"
  | "installFromSkillsSh"
> {
  return {
    searchSkillsSh: async (query: string) => {
      const normalizedQuery = query.trim();
      const generation = getGeneration();

      set({
        skillsShQuery: query,
        skillsShError: null,
        isSkillsShLoading: Boolean(normalizedQuery),
        ...(normalizedQuery ? {} : { skillsShResults: [] }),
      });

      if (!normalizedQuery) {
        return [];
      }

      if (!isTauriRuntime()) {
        if (generation === getGeneration()) {
          set({ skillsShError: DESKTOP_ONLY_ERROR, isSkillsShLoading: false });
        }
        throw new Error(DESKTOP_ONLY_ERROR);
      }

      try {
        const results = await invoke<SkillsShSkill[]>("search_skills_sh", {
          query: normalizedQuery,
          limit: 30,
        });
        if (generation === getGeneration()) {
          set({
            skillsShResults: results ?? [],
            isSkillsShLoading: false,
            skillsShError: null,
          });
        }
        return results ?? [];
      } catch (err) {
        if (generation === getGeneration()) {
          set({ skillsShError: String(err), isSkillsShLoading: false });
        }
        throw err;
      }
    },

    resolveSkillsShUrl: async (source: string, skillId: string) => {
      if (!isTauriRuntime()) {
        throw new Error(DESKTOP_ONLY_ERROR);
      }
      return invoke<string>("resolve_skills_sh_url", { source, skillId });
    },

    browseSkillsShDirectory: async (source: string, skillId: string) => {
      if (!isTauriRuntime()) {
        throw new Error(DESKTOP_ONLY_ERROR);
      }
      return invoke<SkillsShFileEntry[]>("browse_skills_sh_directory", {
        source,
        skillId,
      });
    },

    readSkillsShFile: async (source: string, filePath: string) => {
      if (!isTauriRuntime()) {
        throw new Error(DESKTOP_ONLY_ERROR);
      }
      return invoke<string>("read_skills_sh_file", { source, filePath });
    },

    installFromSkillsSh: async (source: string, skillId: string) => {
      const generation = getGeneration();
      const installKey = `skills.sh:${source}:${skillId}`;
      set((state) => ({
        installingIds: new Set(state.installingIds).add(installKey),
      }));

      if (!isTauriRuntime()) {
        set((state) => {
          const installingIds = new Set(state.installingIds);
          installingIds.delete(installKey);
          return { installingIds, skillsShError: DESKTOP_ONLY_ERROR };
        });
        throw new Error(DESKTOP_ONLY_ERROR);
      }

      try {
        const importedSkillId = await invoke<string>("install_from_skills_sh", {
          source,
          skillId,
        });
        if (generation === getGeneration()) {
          set((state) => {
            const installingIds = new Set(state.installingIds);
            installingIds.delete(installKey);
            return {
              installingIds,
              skillsShError: null,
              skillsShResults: state.skillsShResults.map((skill) =>
                skill.source === source && skill.skill_id === skillId
                  ? { ...skill, id: importedSkillId || skill.id }
                  : skill
              ),
            };
          });
        }
        return importedSkillId;
      } catch (err) {
        if (generation === getGeneration()) {
          set((state) => {
            const installingIds = new Set(state.installingIds);
            installingIds.delete(installKey);
            return { installingIds, skillsShError: String(err) };
          });
        }
        throw err;
      }
    },
  };
}
