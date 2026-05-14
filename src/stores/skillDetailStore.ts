import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { DirectoryTreeEntry, ProjectUsingSkill, SkillDetail, SkillDetailRequest } from "@/types";
import { parseBackendError } from "@/lib/backendError";
import {
  ExplanationErrorInfo,
  setupExplanationStreamListeners,
} from "@/lib/explanationStream";

// ─── State ────────────────────────────────────────────────────────────────────

interface SkillDetailState {
  detail: SkillDetail | null;
  content: string | null;
  isLoading: boolean;
  /** Agent ID currently being installed/uninstalled (null = idle). */
  installingAgentId: string | null;
  error: string | null;
  explanation: string | null;
  isExplanationLoading: boolean;
  isExplanationStreaming: boolean;
  explanationError: string | null;
  explanationErrorInfo: ExplanationErrorInfo | null;
  /** File-mode buffers (used by non-central source skills loaded by path). */
  fileContent: string | null;
  fileIsLoading: boolean;
  fileExplanation: string | null;
  fileIsExplaining: boolean;
  directoryTree: DirectoryTreeEntry[];
  isDirectoryLoading: boolean;
  /** Projects that have this central skill installed (per agent). */
  projectsUsingSkill: ProjectUsingSkill[];
  isLoadingProjectsUsingSkill: boolean;

  // Actions
  loadDetail: (request: SkillDetailRequest | string) => Promise<void>;
  loadCachedExplanation: (skillId: string, lang: string) => Promise<void>;
  generateExplanation: (skillId: string, content: string, lang: string) => Promise<void>;
  refreshExplanation: (skillId: string, content: string, lang: string) => Promise<void>;
  installSkill: (skillId: string, agentId: string) => Promise<void>;
  uninstallSkill: (skillId: string, agentId: string, rowId?: string | null) => Promise<void>;
  refreshInstallations: (skillId: string) => Promise<void>;
  /** Read SKILL.md content directly from a path (file mode). */
  loadFileContent: (path: string) => Promise<void>;
  /** Generate a one-shot AI explanation for ad-hoc file content (no streaming). */
  explainFileContent: (content: string) => Promise<void>;
  /** Open the host file manager at `path`. No-op outside the Tauri runtime. */
  openInFileManager: (path: string) => Promise<void>;
  loadDirectoryTree: (path: string) => Promise<void>;
  /** Load the "this skill is installed in N projects" reverse view. */
  loadProjectsUsingSkill: (skillId: string) => Promise<void>;
  /** Reset only the file-mode buffers (used when SkillDetailView toggles modes). */
  resetFileMode: () => void;
  cleanupExplanationListeners: () => void;
  reset: () => void;
}

// ─── Event listeners (managed outside store) ──────────────────────────────────

let unlistenChunk: UnlistenFn | null = null;
let unlistenComplete: UnlistenFn | null = null;
let unlistenError: UnlistenFn | null = null;
let activeExplanationRequestId = 0;
let activeDetailRequest: SkillDetailRequest | null = null;

function normalizeDetailRequest(request: SkillDetailRequest | string): SkillDetailRequest {
  return typeof request === "string" ? { skillId: request } : request;
}

function buildDetailInvokeArgs(request: SkillDetailRequest) {
  return {
    skillId: request.skillId,
    ...(request.agentId ? { agentId: request.agentId } : {}),
    ...(request.rowId ? { rowId: request.rowId } : {}),
  };
}

function resolveDetailRequest(
  request: SkillDetailRequest,
  detail: SkillDetail
): SkillDetailRequest {
  const resolvedRowId =
    request.rowId ??
    (detail.row_id && detail.row_id !== request.skillId ? detail.row_id : undefined);

  return {
    skillId: request.skillId,
    ...(request.agentId ? { agentId: request.agentId } : {}),
    ...(resolvedRowId ? { rowId: resolvedRowId } : {}),
  };
}

function setActiveDetailRequestFromDetail(
  request: SkillDetailRequest,
  detail: SkillDetail
): SkillDetailRequest {
  const resolvedRequest = resolveDetailRequest(request, detail);
  activeDetailRequest = resolvedRequest;
  return resolvedRequest;
}

function getActiveDetailRequest(skillId: string): SkillDetailRequest {
  if (activeDetailRequest?.skillId === skillId) {
    return activeDetailRequest;
  }
  return { skillId };
}

function nextExplanationRequestId() {
  activeExplanationRequestId += 1;
  return activeExplanationRequestId;
}

function cleanupExplanationListeners() {
  if (unlistenChunk) { unlistenChunk(); unlistenChunk = null; }
  if (unlistenComplete) { unlistenComplete(); unlistenComplete = null; }
  if (unlistenError) { unlistenError(); unlistenError = null; }
}

function startExplanationRequest(set: (fn: Partial<SkillDetailState>) => void) {
  cleanupExplanationListeners();
  set({
    explanation: null,
    isExplanationLoading: true,
    isExplanationStreaming: false,
    explanationError: null,
    explanationErrorInfo: null,
  });
  return nextExplanationRequestId();
}

function failExplanationRequest(
  requestId: number,
  error: unknown,
  set: (fn: Partial<SkillDetailState>) => void
) {
  if (requestId !== activeExplanationRequestId) {
    return;
  }
  cleanupExplanationListeners();
  const parsed = parseBackendError(error);
  set({
    explanation: null,
    explanationError: parsed.message,
    explanationErrorInfo: parsed.code
      ? {
          code: parsed.code,
          message: parsed.message,
          details: parsed.details ?? parsed.message,
          kind: "response",
          retryable: false,
          fallbackTried: false,
        }
      : null,
    isExplanationLoading: false,
    isExplanationStreaming: false,
  });
}

async function setupExplanationListeners(
  skillId: string,
  requestId: number,
  set: (fn: Partial<SkillDetailState> | ((s: SkillDetailState) => Partial<SkillDetailState>)) => void
) {
  cleanupExplanationListeners();
  activeExplanationRequestId = requestId;
  const stopListening = await setupExplanationStreamListeners(skillId, {
    onChunk: (chunkText) => {
      if (requestId !== activeExplanationRequestId) return;
      set((state) => ({
        explanation: `${state.explanation ?? ""}${chunkText}`,
        isExplanationLoading: false,
        isExplanationStreaming: true,
      }));
    },
    onComplete: (payload) => {
      if (requestId !== activeExplanationRequestId) return;
      set((state) => {
        const nextExplanation = payload.explanation ?? state.explanation;
        const hasExplanation = Boolean(nextExplanation?.trim());
        return {
          explanation: hasExplanation ? nextExplanation : null,
          explanationError: hasExplanation ? payload.error ?? null : "AI explanation returned no content.",
          explanationErrorInfo: null,
          isExplanationLoading: false,
          isExplanationStreaming: false,
        };
      });
      cleanupExplanationListeners();
    },
    onError: (payload) => {
      if (requestId !== activeExplanationRequestId) return;
      set({
        explanation: null,
        explanationError: payload.error ?? "Unknown explanation error",
        explanationErrorInfo: payload.error_info ?? null,
        isExplanationLoading: false,
        isExplanationStreaming: false,
      });
      cleanupExplanationListeners();
    },
  });
  unlistenChunk = stopListening;
  unlistenComplete = () => {};
  unlistenError = () => {};
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const useSkillDetailStore = create<SkillDetailState>((set) => ({
  detail: null,
  content: null,
  isLoading: false,
  installingAgentId: null,
  error: null,
  explanation: null,
  isExplanationLoading: false,
  isExplanationStreaming: false,
  explanationError: null,
  explanationErrorInfo: null,
  fileContent: null,
  fileIsLoading: false,
  fileExplanation: null,
  fileIsExplaining: false,
  directoryTree: [],
  isDirectoryLoading: false,
  projectsUsingSkill: [],
  isLoadingProjectsUsingSkill: false,

  /**
   * Load skill detail metadata, then read raw SKILL.md content from the
   * resolved detail row's file path so duplicate/source-aware rows keep the
   * selected content source.
   */
  loadDetail: async (request: SkillDetailRequest | string) => {
    const detailRequest = normalizeDetailRequest(request);
    activeDetailRequest = detailRequest;
    set({ isLoading: true, error: null });
    if (!isTauriRuntime()) {
      set({
        detail: null,
        content: null,
        isLoading: false,
        error: null,
      });
      return;
    }
    try {
      const detail = await invoke<SkillDetail>(
        "get_skill_detail",
        buildDetailInvokeArgs(detailRequest)
      );
      setActiveDetailRequestFromDetail(detailRequest, detail);
      const content = await invoke<string>("read_file_by_path", {
        path: detail.file_path,
      });
      set({ detail, content, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  loadCachedExplanation: async (skillId: string, lang: string) => {
    const requestId = startExplanationRequest(set);
    if (!isTauriRuntime()) {
      set({
        explanation: null,
        isExplanationLoading: false,
        isExplanationStreaming: false,
        explanationError: null,
        explanationErrorInfo: null,
      });
      return;
    }
    try {
      const explanation = await invoke<string | null>("get_skill_explanation", { skillId, lang });
      if (requestId !== activeExplanationRequestId) return;
      set({
        explanation,
        isExplanationLoading: false,
        isExplanationStreaming: false,
        explanationError: null,
        explanationErrorInfo: null,
      });
    } catch (err) {
      failExplanationRequest(requestId, err, set);
    }
  },

  generateExplanation: async (skillId: string, content: string, lang: string) => {
    const requestId = startExplanationRequest(set);
    if (!isTauriRuntime()) {
      set({
        explanation: null,
        isExplanationLoading: false,
        isExplanationStreaming: false,
        explanationError: "AI explanation requires the Tauri desktop runtime.",
        explanationErrorInfo: null,
      });
      return;
    }
    try {
      await setupExplanationListeners(skillId, requestId, set);
      await invoke("explain_skill_stream", { skillId, content, lang });
    } catch (err) {
      failExplanationRequest(requestId, err, set);
    }
  },

  refreshExplanation: async (skillId: string, content: string, lang: string) => {
    const requestId = startExplanationRequest(set);
    if (!isTauriRuntime()) {
      set({
        explanation: null,
        isExplanationLoading: false,
        isExplanationStreaming: false,
        explanationError: "AI explanation requires the Tauri desktop runtime.",
        explanationErrorInfo: null,
      });
      return;
    }
    try {
      await setupExplanationListeners(skillId, requestId, set);
      await invoke("refresh_skill_explanation", { skillId, content, lang });
    } catch (err) {
      failExplanationRequest(requestId, err, set);
    }
  },

  /**
   * Install the skill to the given agent via the backend default method.
   * Reloads detail afterward so installation status updates.
   */
  installSkill: async (skillId: string, agentId: string) => {
    set({ installingAgentId: agentId, error: null });
    if (!isTauriRuntime()) {
      set({
        installingAgentId: null,
        error: "Installing skills requires the Tauri desktop runtime.",
      });
      return;
    }
    try {
      await invoke("install_skill_to_agent", {
        skillId,
        agentId,
        method: "auto",
      });
      // Reload detail so the installations list reflects the new install.
      const detailRequest = getActiveDetailRequest(skillId);
      const detail = await invoke<SkillDetail>(
        "get_skill_detail",
        buildDetailInvokeArgs(detailRequest)
      );
      setActiveDetailRequestFromDetail(detailRequest, detail);
      set({ detail, installingAgentId: null });
    } catch (err) {
      set({ error: String(err), installingAgentId: null });
    }
  },

  /**
   * Remove the skill installation from the given agent.
   * Reloads detail afterward so installation status updates.
   */
  uninstallSkill: async (skillId: string, agentId: string, rowId?: string | null) => {
    set({ installingAgentId: agentId, error: null });
    if (!isTauriRuntime()) {
      set({
        installingAgentId: null,
        error: "Uninstalling skills requires the Tauri desktop runtime.",
      });
      return;
    }
    try {
      await invoke("uninstall_skill_from_agent", {
        skillId,
        agentId,
        ...(rowId ? { rowId } : {}),
      });
      // Reload detail so the installations list reflects the removal.
      const detailRequest = getActiveDetailRequest(skillId);
      const detail = await invoke<SkillDetail>(
        "get_skill_detail",
        buildDetailInvokeArgs(detailRequest)
      );
      setActiveDetailRequestFromDetail(detailRequest, detail);
      set({ detail, installingAgentId: null });
    } catch (err) {
      set({ error: String(err), installingAgentId: null });
    }
  },

  refreshInstallations: async (skillId: string) => {
    if (!isTauriRuntime()) {
      return;
    }
    try {
      const detailRequest = getActiveDetailRequest(skillId);
      const detail = await invoke<SkillDetail>(
        "get_skill_detail",
        buildDetailInvokeArgs(detailRequest)
      );
      setActiveDetailRequestFromDetail(detailRequest, detail);
      set((state) => ({
        detail,
        content: state.content,
        isLoading: state.isLoading,
      }));
    } catch (err) {
      set({ error: String(err) });
    }
  },

  cleanupExplanationListeners,

  loadFileContent: async (path: string) => {
    if (!path) {
      set({ fileContent: null, fileIsLoading: false });
      return;
    }
    set({ fileIsLoading: true });
    if (!isTauriRuntime()) {
      set({ fileContent: null, fileIsLoading: false });
      return;
    }
    try {
      const text = await invoke<string>("read_file_by_path", { path });
      set({ fileContent: text, fileIsLoading: false });
    } catch {
      set({ fileContent: null, fileIsLoading: false });
    }
  },

  loadDirectoryTree: async (path: string) => {
    if (!path) {
      set({ directoryTree: [], isDirectoryLoading: false });
      return;
    }
    set({ isDirectoryLoading: true });
    if (!isTauriRuntime()) {
      set({ directoryTree: [], isDirectoryLoading: false });
      return;
    }
    try {
      const directoryTree = await invoke<DirectoryTreeEntry[]>("list_directory_tree", { path });
      set({ directoryTree: directoryTree ?? [], isDirectoryLoading: false });
    } catch {
      set({ directoryTree: [], isDirectoryLoading: false });
    }
  },

  explainFileContent: async (content: string) => {
    if (!content) {
      set({ fileExplanation: null, fileIsExplaining: false });
      return;
    }
    set({ fileIsExplaining: true, fileExplanation: null });
    if (!isTauriRuntime()) {
      set({
        fileIsExplaining: false,
        fileExplanation: "AI explanation requires the Tauri desktop runtime.",
      });
      return;
    }
    try {
      const result = await invoke<string>("explain_skill", { content });
      set({ fileExplanation: result, fileIsExplaining: false });
    } catch (err) {
      const parsed = parseBackendError(err);
      set({
        fileExplanation: `Error: ${parsed.message}`,
        fileIsExplaining: false,
      });
    }
  },

  openInFileManager: async (path: string) => {
    if (!path || !isTauriRuntime()) {
      return;
    }
    try {
      await invoke("open_in_file_manager", { path });
    } catch {
      // silently ignore — opener failure should not surface as a hard error
    }
  },

  loadProjectsUsingSkill: async (skillId: string) => {
    if (!skillId) {
      set({ projectsUsingSkill: [], isLoadingProjectsUsingSkill: false });
      return;
    }
    if (!isTauriRuntime()) {
      set({ projectsUsingSkill: [], isLoadingProjectsUsingSkill: false });
      return;
    }
    set({ isLoadingProjectsUsingSkill: true });
    try {
      const rows = await invoke<ProjectUsingSkill[]>("list_projects_using_skill", {
        skillId,
      });
      set({ projectsUsingSkill: rows, isLoadingProjectsUsingSkill: false });
    } catch {
      set({ projectsUsingSkill: [], isLoadingProjectsUsingSkill: false });
    }
  },

  resetFileMode: () => {
    set({
      fileContent: null,
      fileIsLoading: false,
      fileExplanation: null,
      fileIsExplaining: false,
      directoryTree: [],
      isDirectoryLoading: false,
    });
  },

  /**
   * Reset the store to its initial state (called when leaving the detail page).
   */
  reset: () => {
    cleanupExplanationListeners();
    activeDetailRequest = null;
    set({
      detail: null,
      content: null,
      isLoading: false,
      installingAgentId: null,
      error: null,
      explanation: null,
      isExplanationLoading: false,
      isExplanationStreaming: false,
      explanationError: null,
      explanationErrorInfo: null,
      fileContent: null,
      fileIsLoading: false,
      fileExplanation: null,
      fileIsExplaining: false,
      directoryTree: [],
      isDirectoryLoading: false,
      projectsUsingSkill: [],
      isLoadingProjectsUsingSkill: false,
    });
  },
}));
