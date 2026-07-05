import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import type {
  RuntimeLogClearRequest,
  RuntimeLogFile,
  RuntimeLogLevel,
  RuntimeLogReadRequest,
  RuntimeLogReadResult,
} from "@/types";

const DEFAULT_RUNTIME_LIMIT = 200;

interface RuntimeLogFilter {
  query?: string;
  level?: RuntimeLogLevel | "";
  source?: string;
}

interface RuntimeLogState {
  files: RuntimeLogFile[];
  selectedFileName: string | null;
  readResult: RuntimeLogReadResult | null;
  filter: RuntimeLogFilter;
  isLoadingFiles: boolean;
  isLoadingLines: boolean;
  isClearing: boolean;
  isExporting: boolean;
  error: string | null;

  loadFiles: () => Promise<RuntimeLogFile[]>;
  selectFile: (fileName: string) => Promise<RuntimeLogReadResult>;
  readFile: (
    request?: Partial<RuntimeLogReadRequest>,
  ) => Promise<RuntimeLogReadResult>;
  setFilter: (partial: Partial<RuntimeLogFilter>) => void;
  clearFilters: () => void;
  clearLogs: (request?: RuntimeLogClearRequest) => Promise<number>;
  exportLog: (fileName?: string) => Promise<string>;
  clearError: () => void;
}

function normalizeText(value?: string): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function normalizeReadRequest(
  selectedFileName: string | null,
  filter: RuntimeLogFilter,
  request: Partial<RuntimeLogReadRequest> = {},
): RuntimeLogReadRequest {
  const fileName = request.fileName ?? selectedFileName;
  if (!fileName) {
    throw new Error("No runtime log file is available.");
  }

  return {
    fileName,
    query: normalizeText(request.query ?? filter.query),
    level: (request.level ?? filter.level) || undefined,
    source: normalizeText(request.source ?? filter.source),
    limit: request.limit ?? DEFAULT_RUNTIME_LIMIT,
    offset: request.offset ?? 0,
    tail: request.tail ?? true,
  };
}

export const useRuntimeLogStore = create<RuntimeLogState>((set, get) => ({
  files: [],
  selectedFileName: null,
  readResult: null,
  filter: {},
  isLoadingFiles: false,
  isLoadingLines: false,
  isClearing: false,
  isExporting: false,
  error: null,

  loadFiles: async () => {
    set({ isLoadingFiles: true, error: null });
    try {
      const files = await invoke("list_runtime_log_files");
      const selectedFileName = files.some(
        (file) => file.fileName === get().selectedFileName,
      )
        ? get().selectedFileName
        : (files[0]?.fileName ?? null);
      set({ files, selectedFileName, isLoadingFiles: false });
      return files;
    } catch (err) {
      set({ error: String(err), isLoadingFiles: false });
      throw err;
    }
  },

  selectFile: async (fileName) => {
    set({ selectedFileName: fileName });
    return get().readFile({ fileName, offset: 0, tail: true });
  },

  readFile: async (request) => {
    const nextRequest = normalizeReadRequest(
      get().selectedFileName,
      get().filter,
      request,
    );
    set({
      isLoadingLines: true,
      error: null,
      selectedFileName: nextRequest.fileName,
    });
    try {
      const result = await invoke("read_runtime_log_file", {
        request: nextRequest,
      });
      set({ readResult: result, isLoadingLines: false });
      return result;
    } catch (err) {
      set({ error: String(err), isLoadingLines: false });
      throw err;
    }
  },

  setFilter: (partial) => {
    set((state) => ({
      filter: {
        ...state.filter,
        ...partial,
      },
    }));
  },

  clearFilters: () => {
    set({ filter: {} });
  },

  clearLogs: async (request) => {
    const currentFileName = get().selectedFileName;
    const targetRequest: RuntimeLogClearRequest =
      request ??
      (currentFileName ? { fileName: currentFileName } : { all: true });
    set({ isClearing: true, error: null });
    try {
      const deleted = await invoke("clear_runtime_logs", {
        request: targetRequest,
      });
      const files = await get().loadFiles();
      if (files.length > 0) {
        await get().readFile({
          fileName: files[0].fileName,
          offset: 0,
          tail: true,
        });
      } else {
        set({ selectedFileName: null, readResult: null });
      }
      set({ isClearing: false });
      return deleted;
    } catch (err) {
      set({ error: String(err), isClearing: false });
      throw err;
    }
  },

  exportLog: async (fileName) => {
    const targetFileName = fileName ?? get().selectedFileName;
    if (!targetFileName) {
      throw new Error("No runtime log file is selected.");
    }

    set({ isExporting: true, error: null });
    try {
      const payload = await invoke("export_runtime_log_file", {
        fileName: targetFileName,
      });
      set({ isExporting: false });
      return payload;
    } catch (err) {
      set({ error: String(err), isExporting: false });
      throw err;
    }
  },

  clearError: () => set({ error: null }),
}));
