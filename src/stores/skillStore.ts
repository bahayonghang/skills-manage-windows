import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/tauri";
import { ScannedSkill } from "@/types";

const BROWSER_FIXTURE_SKILLS_BY_AGENT: Record<string, ScannedSkill[]> = {
  "claude-code": [
    {
      id: "fixture-central-skill",
      name: "fixture-central-skill",
      description: "Installed browser validation fixture for platform drawer flows.",
      file_path: "~/.claude/skills/fixture-central-skill/SKILL.md",
      dir_path: "~/.claude/skills/fixture-central-skill",
      link_type: "symlink",
      symlink_target: "~/.agents/skills/fixture-central-skill",
      is_central: true,
    },
  ],
  cursor: [
    {
      id: "fixture-central-skill",
      name: "fixture-central-skill",
      description: "Installed browser validation fixture for platform drawer flows.",
      file_path: "~/.cursor/skills/fixture-central-skill/SKILL.md",
      dir_path: "~/.cursor/skills/fixture-central-skill",
      link_type: "symlink",
      symlink_target: "~/.agents/skills/fixture-central-skill",
      is_central: true,
    },
  ],
};

// ─── State ────────────────────────────────────────────────────────────────────

interface SkillState {
  skillsByAgent: Record<string, ScannedSkill[]>;
  loadingByAgent: Record<string, boolean>;
  pendingSkillActionKeys: Record<string, boolean>;
  error: string | null;

  // Actions
  getSkillsByAgent: (agentId: string) => Promise<void>;
  uninstallSkillFromAgent: (skillId: string, agentId: string) => Promise<void>;
}

// ─── Store ────────────────────────────────────────────────────────────────────

function skillActionKey(agentId: string, skillId: string) {
  return `${agentId}::${skillId}`;
}

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
    set((state) => ({
      loadingByAgent: { ...state.loadingByAgent, [agentId]: true },
      error: null,
    }));
    if (!isTauriRuntime()) {
      set((state) => ({
        skillsByAgent: {
          ...state.skillsByAgent,
          [agentId]: BROWSER_FIXTURE_SKILLS_BY_AGENT[agentId] ?? [],
        },
        loadingByAgent: { ...state.loadingByAgent, [agentId]: false },
      }));
      return;
    }
    try {
      const skills = await invoke<ScannedSkill[]>("get_skills_by_agent", {
        agentId,
      });
      set((state) => ({
        skillsByAgent: { ...state.skillsByAgent, [agentId]: skills },
        loadingByAgent: { ...state.loadingByAgent, [agentId]: false },
      }));
    } catch (err) {
      set((state) => ({
        error: String(err),
        loadingByAgent: { ...state.loadingByAgent, [agentId]: false },
      }));
    }
  },

  uninstallSkillFromAgent: async (skillId: string, agentId: string) => {
    const actionKey = skillActionKey(agentId, skillId);
    set((state) => ({
      pendingSkillActionKeys: {
        ...state.pendingSkillActionKeys,
        [actionKey]: true,
      },
      error: null,
    }));

    if (!isTauriRuntime()) {
      set((state) => {
        const next = { ...state.pendingSkillActionKeys };
        delete next[actionKey];
        return {
          pendingSkillActionKeys: next,
          error: "Uninstalling skills requires the Tauri desktop runtime.",
        };
      });
      return;
    }

    try {
      await invoke("uninstall_skill_from_agent", { skillId, agentId });
      const skills = await invoke<ScannedSkill[]>("get_skills_by_agent", {
        agentId,
      });

      set((state) => {
        const next = { ...state.pendingSkillActionKeys };
        delete next[actionKey];
        return {
          skillsByAgent: { ...state.skillsByAgent, [agentId]: skills },
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
}));
