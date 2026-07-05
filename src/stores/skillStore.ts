import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import {
  BatchUninstallSkillRequest,
  BatchUninstallSkillResult,
  ScannedSkill,
  SkillWithLinks,
} from "@/types";
import { getBatchUninstallRequestActionKey } from "@/lib/platformBatchActions";

// ─── State ────────────────────────────────────────────────────────────────────

interface SkillState {
  skillsByAgent: Record<string, ScannedSkill[]>;
  loadingByAgent: Record<string, boolean>;
  pendingSkillActionKeys: Record<string, boolean>;
  error: string | null;

  // Actions
  getSkillsByAgent: (agentId: string) => Promise<void>;
  uninstallSkillFromAgent: (
    skillId: string,
    agentId: string,
    rowId?: string | null,
  ) => Promise<void>;
  batchUninstallSkillsFromAgent: (
    agentId: string,
    requests: BatchUninstallSkillRequest[],
  ) => Promise<BatchUninstallSkillResult>;
  /** Fetch central skills as a one-shot read (no internal caching). */
  fetchCentralSkillsList: () => Promise<SkillWithLinks[]>;
  resetForTargetChange: () => void;
}

// ─── Store ────────────────────────────────────────────────────────────────────

function skillActionKey(
  agentId: string,
  skillId: string,
  rowId?: string | null,
) {
  return rowId ?? `${agentId}::${skillId}`;
}

let skillStoreGeneration = 0;

export const useSkillStore = create<SkillState>((set) => ({
  skillsByAgent: {},
  loadingByAgent: {},
  pendingSkillActionKeys: {},
  error: null,

  /**
   * Fetch skills for a specific agent by invoking the Tauri backend command.
   * Results are cached per agentId in skillsByAgent.
   */
  getSkillsByAgent: async (agentId: string) => {
    const generation = skillStoreGeneration;
    set((state) => ({
      loadingByAgent: { ...state.loadingByAgent, [agentId]: true },
      error: null,
    }));
    try {
      const skills = await invoke("get_skills_by_agent", {
        agentId,
      });
      set((state) => ({
        skillsByAgent:
          generation === skillStoreGeneration
            ? { ...state.skillsByAgent, [agentId]: skills }
            : state.skillsByAgent,
        loadingByAgent: { ...state.loadingByAgent, [agentId]: false },
      }));
    } catch (err) {
      set((state) => ({
        error: String(err),
        loadingByAgent: { ...state.loadingByAgent, [agentId]: false },
      }));
    }
  },

  uninstallSkillFromAgent: async (
    skillId: string,
    agentId: string,
    rowId?: string | null,
  ) => {
    const generation = skillStoreGeneration;
    const actionKey = skillActionKey(agentId, skillId, rowId);
    set((state) => ({
      pendingSkillActionKeys: {
        ...state.pendingSkillActionKeys,
        [actionKey]: true,
      },
      error: null,
    }));

    try {
      await invoke("uninstall_skill_from_agent", {
        skillId,
        agentId,
        ...(rowId ? { rowId } : {}),
      });
      const skills = await invoke("get_skills_by_agent", {
        agentId,
      });

      set((state) => {
        const next = { ...state.pendingSkillActionKeys };
        delete next[actionKey];
        return {
          skillsByAgent:
            generation === skillStoreGeneration
              ? { ...state.skillsByAgent, [agentId]: skills }
              : state.skillsByAgent,
          pendingSkillActionKeys: next,
        };
      });
    } catch (err) {
      set((state) => {
        const next = { ...state.pendingSkillActionKeys };
        delete next[actionKey];
        return {
          error: String(err),
          pendingSkillActionKeys: next,
        };
      });
      throw err;
    }
  },

  batchUninstallSkillsFromAgent: async (
    agentId: string,
    requests: BatchUninstallSkillRequest[],
  ) => {
    if (requests.length === 0) {
      return { succeeded: [], failed: [] };
    }

    const generation = skillStoreGeneration;
    const actionKeys = requests.map((request) =>
      getBatchUninstallRequestActionKey(agentId, request),
    );
    set((state) => ({
      pendingSkillActionKeys: {
        ...state.pendingSkillActionKeys,
        ...Object.fromEntries(actionKeys.map((key) => [key, true])),
      },
      error: null,
    }));

    try {
      const result = await invoke("batch_uninstall_skills_from_agent", {
        agentId,
        requests,
      });
      const skills = await invoke("get_skills_by_agent", {
        agentId,
      });

      set((state) => {
        const next = { ...state.pendingSkillActionKeys };
        actionKeys.forEach((key) => delete next[key]);
        return {
          skillsByAgent:
            generation === skillStoreGeneration
              ? { ...state.skillsByAgent, [agentId]: skills }
              : state.skillsByAgent,
          pendingSkillActionKeys: next,
        };
      });
      return result;
    } catch (err) {
      set((state) => {
        const next = { ...state.pendingSkillActionKeys };
        actionKeys.forEach((key) => delete next[key]);
        return {
          error: String(err),
          pendingSkillActionKeys: next,
        };
      });
      throw err;
    }
  },

  resetForTargetChange: () => {
    skillStoreGeneration += 1;
    set({
      skillsByAgent: {},
      loadingByAgent: {},
      pendingSkillActionKeys: {},
      error: null,
    });
  },

  fetchCentralSkillsList: async () => {
    return invoke("get_central_skills");
  },
}));
