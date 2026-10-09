import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import { parseBackendError } from "@/lib/backendError";
import { useTargetStore } from "@/stores/targetStore";
import type {
  RuleDetail,
  RuleDraft,
  RuleTool,
  RulesImportPreview,
  RulesImportResult,
  RulesSnapshot,
} from "@/types/rules";

interface RulesState {
  targetId: string | null;
  snapshot: RulesSnapshot | null;
  selectedName: string | null;
  detail: RuleDetail | null;
  drafts: Record<string, RuleDraft>;
  loading: boolean;
  detailLoading: boolean;
  saving: boolean;
  mutating: boolean;
  pendingTools: Partial<Record<RuleTool, boolean>>;
  importPreview: RulesImportPreview | null;
  importResult: RulesImportResult | null;
  errorCode: string | null;
  requiresReload: boolean;
  load: () => Promise<void>;
  select: (name: string) => Promise<void>;
  editBody: (body: string) => void;
  discardDraft: () => void;
  save: () => Promise<void>;
  createRule: (name: string, description: string, body: string) => Promise<void>;
  previewImport: () => Promise<void>;
  importRules: (names: string[]) => Promise<void>;
  setTargetEnabled: (tool: RuleTool, enabled: boolean, fingerprint?: string | null) => Promise<void>;
  deleteRule: () => Promise<void>;
  recover: (operationId: string) => Promise<void>;
  clearError: () => void;
}

let listRequest = 0;
let detailRequest = 0;
let previewRequest = 0;
let targetGeneration = 0;

function localTargetId(): string {
  const target = useTargetStore.getState().activeTarget;
  if (target.kind !== "local") throw new Error("rules.local_only:Local rules only");
  return target.id;
}

function isCurrent(targetId: string, generation = targetGeneration): boolean {
  return useTargetStore.getState().activeTarget.id === targetId && generation === targetGeneration;
}

function errorCode(error: unknown): string {
  return parseBackendError(error).code ?? "internal.unexpected";
}

export function isRuleDraftDirty(draft: RuleDraft | undefined): boolean {
  return !!draft && draft.body !== draft.baseBody;
}

export const useRulesStore = create<RulesState>((set, get) => {
  const updateDetail = (detail: RuleDetail) => {
    ++listRequest;
    if (get().selectedName === detail.name) ++detailRequest;
    set((state) => ({
      detail: state.selectedName === detail.name ? detail : state.detail,
      loading: false,
      detailLoading: state.selectedName === detail.name ? false : state.detailLoading,
      snapshot: state.snapshot
        ? {
            ...state.snapshot,
            rules: state.snapshot.rules.some((rule) => rule.name === detail.name)
              ? state.snapshot.rules.map((rule) => rule.name === detail.name ? detail : rule)
              : [...state.snapshot.rules, detail].sort((a, b) => a.name.localeCompare(b.name)),
          }
        : state.snapshot,
    }));
  };

  const refreshAfterMutation = async (targetId: string, generation: number) => {
    if (!isCurrent(targetId, generation)) return;
    set({ requiresReload: true });
    await get().load();
  };

  return {
    targetId: null,
    snapshot: null,
    selectedName: null,
    detail: null,
    drafts: {},
    loading: false,
    detailLoading: false,
    saving: false,
    mutating: false,
    pendingTools: {},
    importPreview: null,
    importResult: null,
    errorCode: null,
    requiresReload: false,

    load: async () => {
      const target = useTargetStore.getState().activeTarget;
      const request = ++listRequest;
      if (target.kind !== "local") {
        ++detailRequest;
        ++previewRequest;
        set({ targetId: target.id, loading: false, detailLoading: false, saving: false, mutating: false, pendingTools: {}, errorCode: null, importPreview: null });
        return;
      }
      const targetId = target.id;
      set({ targetId, loading: true, errorCode: null });
      try {
        const snapshot = await invoke("list_rules", { targetId });
        if (!isCurrent(targetId) || request !== listRequest) return;
        const selectedName = snapshot.rules.some((rule) => rule.name === get().selectedName)
          ? get().selectedName
          : snapshot.rules[0]?.name ?? null;
        set({ snapshot, selectedName, loading: false, requiresReload: false });
        if (selectedName) await get().select(selectedName);
        else set({ detail: null });
      } catch (error) {
        if (isCurrent(targetId) && request === listRequest) set({ loading: false, errorCode: errorCode(error) });
        throw error;
      }
    },

    select: async (name) => {
      const targetId = localTargetId();
      const request = ++detailRequest;
      set({ selectedName: name, detail: null, detailLoading: true, errorCode: null });
      try {
        const detail = await invoke("read_rule", { targetId, name });
        if (!isCurrent(targetId) || request !== detailRequest) return;
        set({ detail, detailLoading: false });
      } catch (error) {
        if (isCurrent(targetId) && request === detailRequest) set({ detailLoading: false, errorCode: errorCode(error) });
        throw error;
      }
    },

    editBody: (body) => {
      const detail = get().detail;
      if (!detail || useTargetStore.getState().activeTarget.kind !== "local") return;
      set((state) => ({
        drafts: {
          ...state.drafts,
          [detail.name]: { ...(state.drafts[detail.name] ?? { baseBody: detail.body, revision: detail.revision }), body },
        },
      }));
    },

    discardDraft: () => {
      const name = get().selectedName;
      if (!name) return;
      set((state) => {
        const drafts = { ...state.drafts };
        delete drafts[name];
        return { drafts };
      });
    },

    save: async () => {
      const targetId = localTargetId();
      const generation = targetGeneration;
      const { detail, drafts, saving } = get();
      const draft = detail ? drafts[detail.name] : undefined;
      if (!detail || !draft || !isRuleDraftDirty(draft) || saving) return;
      set({ saving: true, errorCode: null });
      try {
        const saved = await invoke("save_rule", { targetId, name: detail.name, body: draft.body, expectedRevision: draft.revision });
        if (!isCurrent(targetId, generation)) return;
        updateDetail(saved);
        set((state) => {
          const current = state.drafts[detail.name];
          if (!current) return {};
          const next = { ...state.drafts };
          if (current.body === draft.body) delete next[detail.name];
          else next[detail.name] = { body: current.body, baseBody: saved.body, revision: saved.revision };
          return { drafts: next };
        });
      } catch (error) {
        if (isCurrent(targetId, generation)) set({ errorCode: errorCode(error) });
        throw error;
      } finally {
        if (isCurrent(targetId, generation)) set({ saving: false });
      }
    },

    createRule: async (name, description, body) => {
      const targetId = localTargetId();
      const generation = targetGeneration;
      set({ mutating: true, errorCode: null });
      try {
        const detail = await invoke("create_rule", { targetId, name, description, body });
        if (!isCurrent(targetId, generation)) return;
        updateDetail(detail);
        ++detailRequest;
        set({ selectedName: detail.name, detail });
        await refreshAfterMutation(targetId, generation);
      } catch (error) {
        if (isCurrent(targetId, generation)) set({ errorCode: errorCode(error) });
        throw error;
      } finally { if (isCurrent(targetId, generation)) set({ mutating: false }); }
    },

    previewImport: async () => {
      const targetId = localTargetId();
      const request = ++previewRequest;
      set({ mutating: true, importPreview: null, importResult: null, errorCode: null });
      try {
        const importPreview = await invoke("preview_rules_import", { targetId });
        if (isCurrent(targetId) && request === previewRequest) set({ importPreview });
      } catch (error) {
        if (isCurrent(targetId) && request === previewRequest) set({ errorCode: errorCode(error) });
        throw error;
      } finally {
        if (request === previewRequest) set({ mutating: false });
      }
    },

    importRules: async (names) => {
      const targetId = localTargetId();
      const generation = targetGeneration;
      const entries = get().importPreview?.entries
        .filter((entry) => names.includes(entry.name) && (entry.status === "new" || entry.status === "same"))
        .map(({ name, sourceFingerprints }) => ({ name, sourceFingerprints })) ?? [];
      if (!entries.length) return;
      set({ mutating: true, errorCode: null });
      try {
        const importResult = await invoke("import_existing_rules", { targetId, entries });
        if (!isCurrent(targetId, generation)) return;
        set({ importResult });
        await refreshAfterMutation(targetId, generation);
      } catch (error) {
        if (isCurrent(targetId, generation)) set({ errorCode: errorCode(error) });
        throw error;
      } finally { if (isCurrent(targetId, generation)) set({ mutating: false }); }
    },

    setTargetEnabled: async (tool, enabled, fingerprint = null) => {
      const targetId = localTargetId();
      const generation = targetGeneration;
      const detail = get().detail;
      if (!detail || get().pendingTools[tool]) return;
      set((state) => ({ pendingTools: { ...state.pendingTools, [tool]: true }, errorCode: null }));
      try {
        const updated = await invoke("set_rule_target_enabled", { targetId, name: detail.name, tool, enabled, expectedDestinationFingerprint: fingerprint });
        if (isCurrent(targetId, generation)) updateDetail(updated);
      } catch (error) {
        if (isCurrent(targetId, generation)) set({ errorCode: errorCode(error), requiresReload: true });
        throw error;
      } finally {
        if (isCurrent(targetId, generation)) set((state) => ({ pendingTools: { ...state.pendingTools, [tool]: false } }));
      }
    },

    deleteRule: async () => {
      const targetId = localTargetId();
      const generation = targetGeneration;
      const detail = get().detail;
      if (!detail) return;
      set({ mutating: true, errorCode: null });
      try {
        const result = await invoke("delete_rule", { targetId, name: detail.name, expectedRevision: detail.revision });
        if (!isCurrent(targetId, generation)) return;
        if (!result.recoveryRequired) {
          set((state) => {
            const drafts = { ...state.drafts };
            delete drafts[detail.name];
            return { drafts, selectedName: state.selectedName === detail.name ? null : state.selectedName };
          });
        }
        await refreshAfterMutation(targetId, generation);
      } catch (error) {
        if (isCurrent(targetId, generation)) set({ errorCode: errorCode(error), requiresReload: true });
        throw error;
      } finally { if (isCurrent(targetId, generation)) set({ mutating: false }); }
    },

    recover: async (operationId) => {
      const targetId = localTargetId();
      const generation = targetGeneration;
      set({ mutating: true, errorCode: null });
      try {
        await invoke("recover_rule_operation", { targetId, operationId });
        await refreshAfterMutation(targetId, generation);
      } catch (error) {
        if (isCurrent(targetId, generation)) set({ errorCode: errorCode(error) });
        throw error;
      } finally { if (isCurrent(targetId, generation)) set({ mutating: false }); }
    },
    clearError: () => set({ errorCode: null }),
  };
});

useTargetStore.subscribe((state, previous) => {
  if (state.activeTarget.id === previous.activeTarget.id) return;
  ++targetGeneration;
  ++listRequest;
  ++detailRequest;
  ++previewRequest;
  useRulesStore.setState({
    targetId: state.activeTarget.id,
    loading: false,
    detailLoading: false,
    saving: false,
    mutating: false,
    pendingTools: {},
    importPreview: null,
    importResult: null,
    errorCode: null,
  });
});
