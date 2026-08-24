import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import { backendErrorStateValue } from "@/lib/backendError";
import type {
  SkillsCliAddResult,
  SkillsCliDoctorReport,
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliSourcePreview,
} from "@/types";

const BUSY_ENVELOPE =
  "skills_cli.busy:Another skill operation is using this target.";

function newJobId(): string {
  return (
    globalThis.crypto?.randomUUID?.() ??
    `job-${Date.now()}-${Math.random().toString(16).slice(2)}`
  );
}

interface SkillsCliState {
  skills: SkillsCliGlobalSkill[];
  targets: SkillsCliInstallTarget[];
  preview: SkillsCliSourcePreview | null;
  doctor: SkillsCliDoctorReport | null;
  isLoading: boolean;
  isPreviewing: boolean;
  isMutating: boolean;
  isCancelling: boolean;
  jobId: string | null;
  error: string | null;

  loadAll: () => Promise<void>;
  previewSource: (source: string) => Promise<SkillsCliSourcePreview | null>;
  addGlobal: (input: {
    source: string;
    skillNames: string[];
    skillportAgentIds: string[];
  }) => Promise<SkillsCliAddResult | null>;
  removeGlobal: (skillName: string) => Promise<boolean>;
  cancelJob: () => Promise<void>;
  resetForTargetChange: () => void;
}

const emptyState = {
  skills: [] as SkillsCliGlobalSkill[],
  targets: [] as SkillsCliInstallTarget[],
  preview: null as SkillsCliSourcePreview | null,
  doctor: null as SkillsCliDoctorReport | null,
  isLoading: false,
  isPreviewing: false,
  isMutating: false,
  isCancelling: false,
  jobId: null as string | null,
  error: null as string | null,
};

export const useSkillsCliStore = create<SkillsCliState>((set, get) => ({
  ...emptyState,

  async loadAll() {
    set({ isLoading: true, error: null });
    try {
      const [doctor, skills, targets] = await Promise.all([
        invoke("skills_cli_doctor"),
        invoke("skills_cli_list_global"),
        invoke("skills_cli_install_targets"),
      ]);
      set({
        doctor,
        skills: skills ?? [],
        targets: targets ?? [],
        isLoading: false,
      });
    } catch (error) {
      set({
        error: backendErrorStateValue(error),
        isLoading: false,
      });
    }
  },

  async previewSource(source) {
    set({ isPreviewing: true, error: null, preview: null });
    try {
      const preview = await invoke("skills_cli_preview_source", { source });
      set({ preview, isPreviewing: false });
      return preview;
    } catch (error) {
      set({
        error: backendErrorStateValue(error),
        isPreviewing: false,
        preview: null,
      });
      return null;
    }
  },

  async addGlobal(input) {
    if (input.skillNames.length === 0 || input.skillportAgentIds.length === 0) {
      set({ error: "skills_cli.selection_empty:Select at least one skill and one platform." });
      return null;
    }
    if (get().isMutating || get().isCancelling) {
      throw new Error(BUSY_ENVELOPE);
    }
    const jobId = newJobId();
    set({ isMutating: true, error: null, jobId });
    try {
      const result = await invoke("skills_cli_add_global", {
        jobId,
        source: input.source,
        skillNames: input.skillNames,
        skillportAgentIds: input.skillportAgentIds,
      });
      if (get().jobId !== jobId) {
        return result;
      }
      set({ isMutating: false, jobId: null, preview: null });
      await get().loadAll();
      return result;
    } catch (error) {
      if (get().jobId !== jobId) {
        return null;
      }
      set({
        error: backendErrorStateValue(error),
        isMutating: false,
        jobId: null,
      });
      return null;
    }
  },

  async removeGlobal(skillName) {
    if (get().isMutating || get().isCancelling) {
      throw new Error(BUSY_ENVELOPE);
    }
    const jobId = newJobId();
    set({ isMutating: true, error: null, jobId });
    try {
      await invoke("skills_cli_remove_global", { jobId, skillName });
      if (get().jobId !== jobId) {
        return true;
      }
      set({ isMutating: false, jobId: null });
      await get().loadAll();
      return true;
    } catch (error) {
      if (get().jobId !== jobId) {
        return false;
      }
      set({
        error: backendErrorStateValue(error),
        isMutating: false,
        jobId: null,
      });
      return false;
    }
  },

  async cancelJob() {
    const jobId = get().jobId;
    if (!jobId) {
      return;
    }
    set({ isCancelling: true });
    try {
      await invoke("cancel_skills_cli_job", { jobId });
    } finally {
      if (get().jobId === jobId) {
        set({ isCancelling: false });
      }
    }
  },

  resetForTargetChange() {
    set({ ...emptyState });
  },
}));
