import { create } from "zustand";

import { invoke, isTauriRuntime } from "@/lib/tauri";
import type {
  DeletedPlatformCopyGroup,
  PlatformDuplicateGroup,
  SkillRefreshScope,
  SkillRefreshMode,
  SkillUpdateApplyResult,
  SkillUpdateDecisions,
  SkillRefreshContext,
  SkillUpdateInventory,
} from "@/types/skillUpdateInventory";
import { normalizeRefreshContext } from "@/lib/updateCenterRefreshScope";
import { normalizeUpdateCheckMode } from "@/pages/centralUpdateCheckMode";

export interface SkillInventoryFlags {
  /** inventory.updatable 中含此 skill_id（远端有新版本可拉取）。 */
  hasUpdate: boolean;
  /** inventory.remoteMissing 中含此 skill_id（远端已删除该 skill）。 */
  isMissing: boolean;
  /** inventory.remoteAdded 中含此 skillId（远端新增 candidate，未导入中央库）。 */
  isAdded: boolean;
  /** inventory.platformDuplicates 中含此 skill_id（平台目录存在多份冗余副本）。 */
  hasDuplicate: boolean;
  /** inventory.orphans 中含此 skillId（symlink 失效或孤儿副本）。 */
  isOrphan: boolean;
}

const EMPTY_FLAGS: SkillInventoryFlags = {
  hasUpdate: false,
  isMissing: false,
  isAdded: false,
  hasDuplicate: false,
  isOrphan: false,
};

export type UpdateCenterTab =
  | "updatable"
  | "added"
  | "missing"
  | "failed"
  | "duplicates"
  | "deletedPlatformCopies"
  | "orphans";

interface UpdateCenterState {
  inventory: SkillUpdateInventory | null;
  isRefreshing: boolean;
  isApplying: boolean;
  lastRefreshedAt: string | null;
  isDialogOpen: boolean;
  activeTab: UpdateCenterTab;
  refreshContext: SkillRefreshContext;
  refreshMode: SkillRefreshMode;
  error: string | null;
  refresh(scope: SkillRefreshScope): Promise<SkillUpdateInventory | null>;
  apply(decisions: SkillUpdateDecisions): Promise<SkillUpdateApplyResult>;
  clear(scope?: SkillRefreshScope): Promise<void>;
  loadInventory(): Promise<void>;
  scanDuplicates(agentIds?: string[]): Promise<void>;
  scanDeletedPlatformCopies(agentIds?: string[]): Promise<void>;
  openDialog(
    tab?: UpdateCenterTab,
    context?: Partial<SkillRefreshContext> & { mode?: SkillRefreshMode },
  ): void;
  closeDialog(): void;
  setActiveTab(tab: UpdateCenterTab): void;
  setRefreshMode(mode: SkillRefreshMode): void;
}

function emptyInventory(): SkillUpdateInventory {
  return {
    updatable: [],
    remoteAdded: [],
    remoteMissing: [],
    platformDuplicates: [],
    deletedPlatformCopies: [],
    orphans: [],
    failedRepositories: [],
    generatedAt: new Date().toISOString(),
  };
}

function emptyApplyResult(): SkillUpdateApplyResult {
  return {
    updatedSkillIds: [],
    keptMissingSkillIds: [],
    deletedSkillIds: [],
    importedSkillIds: [],
    skippedAdditions: [],
    unskippedAdditions: [],
    removedPlatformDuplicatePaths: [],
    removedDeletedPlatformCopyPaths: [],
    failures: [],
  };
}

export const useUpdateCenterStore = create<UpdateCenterState>((set, get) => ({
  inventory: null,
  isRefreshing: false,
  isApplying: false,
  lastRefreshedAt: null,
  isDialogOpen: false,
  activeTab: "updatable",
  refreshContext: { repositoryIds: [], skillIds: [] },
  refreshMode: "sync",
  error: null,

  async refresh(scope) {
    set({ isRefreshing: true, error: null });
    try {
      if (!isTauriRuntime()) {
        const inventory = emptyInventory();
        set({
          inventory,
          lastRefreshedAt: new Date().toISOString(),
          isRefreshing: false,
        });
        return inventory;
      }
      const inventory = await invoke<SkillUpdateInventory>(
        "refresh_skill_update_inventory",
        { scope },
      );
      set({
        inventory,
        lastRefreshedAt: new Date().toISOString(),
        isRefreshing: false,
      });
      return inventory;
    } catch (err) {
      set({ error: String(err), isRefreshing: false });
      throw err;
    }
  },

  async apply(decisions) {
    set({ isApplying: true, error: null });
    try {
      const result = isTauriRuntime()
        ? await invoke<SkillUpdateApplyResult>("apply_skill_update_decisions", {
            decisions,
          })
        : emptyApplyResult();
      await get().loadInventory();
      set({ isApplying: false });
      return result;
    } catch (err) {
      set({ error: String(err), isApplying: false });
      throw err;
    }
  },

  async clear(scope) {
    if (isTauriRuntime()) {
      await invoke("clear_skill_update_inventory", { scope: scope ?? null });
    }
    set({ inventory: emptyInventory() });
  },

  async loadInventory() {
    if (!isTauriRuntime()) {
      set({ inventory: emptyInventory() });
      return;
    }
    const inventory = await invoke<SkillUpdateInventory>(
      "get_skill_update_inventory",
    );
    set({ inventory });
  },

  async scanDuplicates(agentIds) {
    if (!isTauriRuntime()) return;
    const platformDuplicates = await invoke<PlatformDuplicateGroup[]>(
      "scan_platform_duplicate_skills",
      { agentIds: agentIds ?? null },
    );
    set((state) => ({
      inventory: state.inventory
        ? { ...state.inventory, platformDuplicates }
        : { ...emptyInventory(), platformDuplicates },
    }));
  },

  async scanDeletedPlatformCopies(agentIds) {
    if (!isTauriRuntime()) return;
    const deletedPlatformCopies = await invoke<DeletedPlatformCopyGroup[]>(
      "scan_deleted_platform_copies",
      { agentIds: agentIds ?? null },
    );
    set((state) => ({
      inventory: state.inventory
        ? { ...state.inventory, deletedPlatformCopies }
        : { ...emptyInventory(), deletedPlatformCopies },
    }));
  },

  openDialog(tab, context) {
    set({
      isDialogOpen: true,
      activeTab: tab ?? "updatable",
      refreshContext: normalizeRefreshContext(context),
      refreshMode: normalizeUpdateCheckMode(context?.mode ?? get().refreshMode),
    });
  },
  closeDialog() {
    set({ isDialogOpen: false });
  },
  setActiveTab(tab) {
    set({ activeTab: tab });
  },
  setRefreshMode(mode) {
    set({ refreshMode: normalizeUpdateCheckMode(mode) });
  },
}));

/**
 * 从 store state 派生指定 skill 的 inventory 分类标记。配合
 * `useUpdateCenterStore(state => selectSkillInventoryFlags(state, skillId))` 使用，
 * 没有 inventory 或没命中任何分类时返回全 false。
 */
export function selectSkillInventoryFlags(
  state: UpdateCenterState,
  skillId: string,
): SkillInventoryFlags {
  return selectSkillInventoryFlagsFromInventory(state.inventory, skillId);
}

export function selectSkillInventoryFlagsFromInventory(
  inventory: SkillUpdateInventory | null,
  skillId: string,
): SkillInventoryFlags {
  if (!inventory) return EMPTY_FLAGS;
  return {
    hasUpdate: inventory.updatable.some((entry) => entry.state.skill_id === skillId),
    isMissing: inventory.remoteMissing.some((entry) => entry.state.skill_id === skillId),
    isAdded: inventory.remoteAdded.some((entry) => entry.skillId === skillId),
    hasDuplicate: inventory.platformDuplicates.some((entry) => entry.skillId === skillId),
    isOrphan:
      inventory.orphans.some((entry) => entry.skillId === skillId)
      || (inventory.deletedPlatformCopies ?? []).some((entry) => entry.skillId === skillId),
  };
}
