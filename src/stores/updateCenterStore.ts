import { create } from "zustand";

import { invoke, isTauriRuntime, listen } from "@/lib/ipc";
import type {
  ActiveRefreshRepository,
  ForceRepositoryMirrorRequest,
  ForceRepositoryMirrorResult,
  ForceSkillUpdateRequest,
  ForceSkillUpdateResult,
  SkillRefreshScope,
  SkillRefreshMode,
  SkillUpdateApplyResult,
  SkillUpdateDecisions,
  SkillRefreshContext,
  SkillUpdateInventoryProgressPayload,
  SkillUpdateInventoryRefreshProgress,
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

const UPDATE_INVENTORY_PROGRESS_EVENT = "central://skill-update-inventory-progress";
let refreshOperationSequence = 0;

function createRefreshOperationId(): string {
  refreshOperationSequence += 1;
  return (
    globalThis.crypto?.randomUUID?.() ??
    `inventory-refresh-${Date.now()}-${refreshOperationSequence}`
  );
}

function clampProgress(value: number, total: number): number {
  const normalized = Math.max(0, value);
  return total > 0 ? Math.min(normalized, total) : 0;
}

function addActiveRepository(
  repositories: ActiveRefreshRepository[],
  repository: ActiveRefreshRepository,
): ActiveRefreshRepository[] {
  const existingIndex = repositories.findIndex((item) => item.key === repository.key);
  if (existingIndex < 0) return [...repositories, repository];
  return repositories.map((item, index) => (index === existingIndex ? repository : item));
}

function removeActiveRepository(
  repositories: ActiveRefreshRepository[],
  repositoryKey: string,
): ActiveRefreshRepository[] {
  return repositories.filter((item) => item.key !== repositoryKey);
}

export function mergeInventoryRefreshProgress(
  current: SkillUpdateInventoryRefreshProgress,
  payload: SkillUpdateInventoryProgressPayload,
): SkillUpdateInventoryRefreshProgress {
  const total = Math.max(0, payload.total);
  const completed = clampProgress(payload.completed, total);
  if (payload.status === "started") {
    return {
      ...current,
      phase: "checking",
      total,
      completed,
      activeRepositories: [],
    };
  }
  if (payload.status === "finalizing") {
    return {
      ...current,
      phase: "finalizing",
      total,
      completed,
      activeRepositories: [],
    };
  }

  const repositoryKey = payload.repositoryKey ?? "";
  const repositoryName = payload.repositoryName ?? "";
  if (!repositoryKey) {
    return { ...current, phase: "checking", total, completed };
  }
  if (payload.status === "repository_started" && repositoryName) {
    return {
      ...current,
      phase: "checking",
      total,
      completed,
      activeRepositories: addActiveRepository(current.activeRepositories, {
        key: repositoryKey,
        name: repositoryName,
      }),
    };
  }
  return {
    ...current,
    phase: "checking",
    total,
    completed,
    activeRepositories: removeActiveRepository(
      current.activeRepositories,
      repositoryKey,
    ),
  };
}

interface UpdateCenterState {
  inventory: SkillUpdateInventory | null;
  isRefreshing: boolean;
  refreshProgress: SkillUpdateInventoryRefreshProgress | null;
  isApplying: boolean;
  isForcing: boolean;
  lastRefreshedAt: string | null;
  isDialogOpen: boolean;
  activeTab: UpdateCenterTab;
  refreshContext: SkillRefreshContext;
  refreshMode: SkillRefreshMode;
  error: string | null;
  refresh(scope: SkillRefreshScope): Promise<SkillUpdateInventory | null>;
  apply(
    decisions: SkillUpdateDecisions,
    scope?: SkillRefreshScope,
  ): Promise<SkillUpdateApplyResult>;
  clear(scope?: SkillRefreshScope): Promise<void>;
  loadInventory(scope?: SkillRefreshScope): Promise<void>;
  scanDuplicates(agentIds?: string[]): Promise<void>;
  scanDeletedPlatformCopies(agentIds?: string[]): Promise<void>;
  forceUpdateSkills(
    request: ForceSkillUpdateRequest,
    scope?: SkillRefreshScope,
  ): Promise<ForceSkillUpdateResult>;
  forceMirrorRepositories(
    request: ForceRepositoryMirrorRequest,
    scope?: SkillRefreshScope,
  ): Promise<ForceRepositoryMirrorResult>;
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
  refreshProgress: null,
  isApplying: false,
  isForcing: false,
  lastRefreshedAt: null,
  isDialogOpen: false,
  activeTab: "updatable",
  refreshContext: { repositoryIds: [], skillIds: [], agentIds: [] },
  refreshMode: "sync",
  error: null,

  async refresh(scope) {
    const operationId = createRefreshOperationId();
    set({
      isRefreshing: true,
      error: null,
      refreshProgress: {
        operationId,
        phase: "preparing",
        total: 0,
        completed: 0,
        activeRepositories: [],
      },
    });
    let unlisten: (() => void) | undefined;
    try {
      if (!isTauriRuntime()) {
        const inventory = emptyInventory();
        set({
          inventory,
          lastRefreshedAt: new Date().toISOString(),
          isRefreshing: false,
          refreshProgress: null,
        });
        return inventory;
      }
      unlisten = await listen<SkillUpdateInventoryProgressPayload>(
        UPDATE_INVENTORY_PROGRESS_EVENT,
        (event) => {
          if (event.payload.operationId !== operationId) return;
          set((state) => {
            const current = state.refreshProgress;
            if (!current || current.operationId !== operationId) return state;
            return {
              refreshProgress: mergeInventoryRefreshProgress(current, event.payload),
            };
          });
        },
      );
      const inventory = await invoke("refresh_skill_update_inventory", {
        scope: { ...scope, cachePolicy: scope.cachePolicy ?? "bypass" },
        operationId,
      });
      set({
        inventory,
        lastRefreshedAt: new Date().toISOString(),
        isRefreshing: false,
      });
      return inventory;
    } catch (err) {
      set({ error: String(err), isRefreshing: false });
      throw err;
    } finally {
      try {
        unlisten?.();
      } catch {
        // Listener disposal must not replace the completed refresh result.
      }
      set((state) =>
        state.refreshProgress?.operationId === operationId
          ? { refreshProgress: null }
          : state,
      );
    }
  },

  async apply(decisions, scope) {
    if (get().isApplying) {
      throw new Error("job.central_update_busy:A Central update job is already running.");
    }
    const jobId = globalThis.crypto?.randomUUID?.()
      ?? `job-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    set({ isApplying: true, error: null });
    try {
      const result = isTauriRuntime()
        ? await invoke<SkillUpdateApplyResult>("apply_skill_update_decisions", {
            jobId,
            decisions,
          })
        : emptyApplyResult();
      await get().loadInventory(scope);
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

  async loadInventory(scope) {
    if (!isTauriRuntime()) {
      set({ inventory: emptyInventory() });
      return;
    }
    const inventory = await invoke("get_skill_update_inventory", { scope: scope ?? null });
    set({ inventory });
  },

  async scanDuplicates(agentIds) {
    if (!isTauriRuntime()) return;
    const platformDuplicates = await invoke(
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
    const deletedPlatformCopies = await invoke(
      "scan_deleted_platform_copies",
      { agentIds: agentIds ?? null },
    );
    set((state) => ({
      inventory: state.inventory
        ? { ...state.inventory, deletedPlatformCopies }
        : { ...emptyInventory(), deletedPlatformCopies },
    }));
  },

  async forceUpdateSkills(request, scope) {
    set({ isForcing: true, error: null });
    try {
      const result = isTauriRuntime()
        ? await invoke("force_update_central_skills", {
            request,
          })
        : { overwritten: [], skipped: [], failed: [] };
      await get().loadInventory(scope);
      set({ isForcing: false });
      return result;
    } catch (err) {
      set({ error: String(err), isForcing: false });
      throw err;
    }
  },

  async forceMirrorRepositories(request, scope) {
    set({ isForcing: true, error: null });
    try {
      const result = isTauriRuntime()
        ? await invoke(
            "force_mirror_central_repositories",
            { request },
          )
        : {
            overwritten: [],
            imported: [],
            deleted: { succeeded: [], failed: [] },
            skipped: [],
            failedRepositories: [],
            failedItems: [],
          };
      await get().loadInventory(scope);
      set({ isForcing: false });
      return result;
    } catch (err) {
      set({ error: String(err), isForcing: false });
      throw err;
    }
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
