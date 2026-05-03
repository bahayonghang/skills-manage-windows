import { invoke, isTauriRuntime } from "@/lib/tauri";
import { MarketplaceSkill, SkillRegistry } from "@/types";
import { normalizeRegistryIdentity } from "./marketplaceStore.shared";
import type { MarketplaceState, MarketplaceStoreContext } from "./marketplaceStore.types";

export function createMarketplaceRegistrySlice({ set, get, getGeneration }: MarketplaceStoreContext): Pick<MarketplaceState,
  | "getNormalizedRegistryIdentity"
  | "findDuplicateRegistry"
  | "loadRegistries"
  | "selectRegistry"
  | "setSearchQuery"
  | "syncRegistry"
  | "loadSkills"
  | "loadPreviewSkills"
  | "installSkill"
  | "triggerSkillExplanation"
  | "addRegistry"
  | "removeRegistry"
> {
  return {
  getNormalizedRegistryIdentity: (url: string) => normalizeRegistryIdentity(url),

  findDuplicateRegistry: (url: string) => {
    const normalized = get().getNormalizedRegistryIdentity(url);
    if (!normalized) return null;

    return (
      get().registries.find((registry) => {
        const existingIdentity =
          registry.normalized_url ?? get().getNormalizedRegistryIdentity(registry.url);
        return existingIdentity === normalized;
      }) ?? null
    );
  },

  loadRegistries: async () => {
    const generation = getGeneration();
    set({ isLoading: true, error: null });
    try {
      const registries = await invoke<SkillRegistry[]>("list_registries");
      if (generation === getGeneration()) {
        set({ registries: registries ?? [], isLoading: false });
      }
    } catch (err) {
      if (generation === getGeneration()) {
        set({ error: String(err), isLoading: false });
      }
    }
  },

  selectRegistry: (id: string) => {
    set({ selectedRegistryId: id, searchQuery: "" });
    get().loadSkills(id);
  },

  setSearchQuery: (query: string) => {
    set({ searchQuery: query });
    const { selectedRegistryId } = get();
    if (selectedRegistryId) {
      get().loadSkills(selectedRegistryId, query);
    }
  },

  syncRegistry: async (registryId: string, forceRefresh = false) => {
    const generation = getGeneration();
    set({ isSyncing: true, error: null });
    try {
      const command = forceRefresh ? "sync_registry_with_options" : "sync_registry";
      const skills = forceRefresh
        ? await invoke<MarketplaceSkill[]>(command, {
            registryId,
            options: { forceRefresh: true },
          })
        : await invoke<MarketplaceSkill[]>(command, { registryId });
      const registries = await invoke<SkillRegistry[]>("list_registries");
      if (generation === getGeneration()) {
        set({
          skills: skills ?? [],
          registries: registries ?? [],
          isSyncing: false,
        });
      }
    } catch (err) {
      const registries = await invoke<SkillRegistry[]>("list_registries").catch(() => null);
      if (generation === getGeneration()) {
        set({
          error: String(err),
          registries: registries ?? get().registries,
          isSyncing: false,
        });
      }
      throw err;
    }
  },

  loadSkills: async (registryId: string, query?: string) => {
    const generation = getGeneration();
    set({ isLoading: true, error: null });
    try {
      const skills = await invoke<MarketplaceSkill[]>("search_marketplace_skills", {
        registryId,
        query: query || null,
      });
      if (generation === getGeneration()) {
        set({ skills: skills ?? [], isLoading: false });
      }
    } catch (err) {
      if (generation === getGeneration()) {
        set({ error: String(err), isLoading: false });
      }
    }
  },

  loadPreviewSkills: async (registryId: string) => {
    return invoke<MarketplaceSkill[]>("search_marketplace_skills", {
      registryId,
      query: null,
    });
  },

  installSkill: async (skillId: string) => {
    const generation = getGeneration();
    set((s) => ({ installingIds: new Set(s.installingIds).add(skillId) }));
    try {
      await invoke("install_marketplace_skill", { skillId });
      if (generation === getGeneration()) {
        set((s) => ({
          skills: s.skills.map((sk) =>
            sk.id === skillId ? { ...sk, is_installed: true } : sk
          ),
          installingIds: (() => {
            const next = new Set(s.installingIds);
            next.delete(skillId);
            return next;
          })(),
        }));
      }
    } catch (err) {
      if (generation === getGeneration()) {
        set((s) => {
          const next = new Set(s.installingIds);
          next.delete(skillId);
          return { installingIds: next, error: String(err) };
        });
      }
      throw err;
    }
  },

  triggerSkillExplanation: async (skillId: string, content: string, lang: string) => {
    if (!isTauriRuntime()) {
      throw new Error("AI explanation requires the Tauri desktop runtime.");
    }
    await invoke("refresh_skill_explanation", { skillId, content, lang });
  },

  addRegistry: async (name: string, sourceType: string, url: string) => {
    const generation = getGeneration();
    const duplicate = get().findDuplicateRegistry(url);
    if (duplicate) {
      throw new Error(
        `DUPLICATE_REGISTRY:${JSON.stringify({
          id: duplicate.id,
          name: duplicate.name,
          url: duplicate.url,
          isBuiltin: duplicate.is_builtin,
        })}`
      );
    }
    const registry = await invoke<SkillRegistry>("add_registry", { name, sourceType, url });
    const registries = await invoke<SkillRegistry[]>("list_registries");
    if (generation === getGeneration()) {
      set({ registries: registries ?? [] });
    }
    return registry;
  },

  removeRegistry: async (registryId: string) => {
    const generation = getGeneration();
    await invoke("remove_registry", { registryId });
    const registries = await invoke<SkillRegistry[]>("list_registries");
    if (generation === getGeneration()) {
      set((s) => ({
        registries: registries ?? [],
        selectedRegistryId: s.selectedRegistryId === registryId ? null : s.selectedRegistryId,
      }));
    }
  },
  };
}
