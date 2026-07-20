import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import {
  DailyOperationCount,
  OperationLogEntry,
  OperationLogFilter,
  OperationLogPage,
} from "@/types";

const DEFAULT_LIMIT = 100;

// latest-wins 令牌：旧响应不得覆盖新结果（参照 platformStore refreshToken 模式）。
let dailyCountsToken = 0;

interface OperationLogState {
  entries: OperationLogEntry[];
  total: number;
  filter: OperationLogFilter;
  selectedEntry: OperationLogEntry | null;
  isLoading: boolean;
  isLoadingDetail: boolean;
  isClearing: boolean;
  isExporting: boolean;
  error: string | null;
  /** Dashboard Activity 图表：后端按本地日聚合的每日计数（跨 target 语义，
   *  与日志条目一致，不随 target 切换重置）。 */
  dailyCounts: DailyOperationCount[];
  isDailyCountsLoading: boolean;
  dailyCountsError: string | null;

  loadLogs: (
    filter?: OperationLogFilter,
    reset?: boolean,
  ) => Promise<OperationLogPage>;
  loadDailyCounts: (days: number) => Promise<void>;
  loadMore: () => Promise<OperationLogPage | null>;
  loadLogDetail: (logId: string) => Promise<OperationLogEntry | null>;
  setFilter: (partial: Partial<OperationLogFilter>) => void;
  clearFilters: () => void;
  clearLogs: (filter?: OperationLogFilter) => Promise<number>;
  exportLogs: (filter?: OperationLogFilter) => Promise<string>;
  closeDetail: () => void;
  clearError: () => void;
}

function normalizeFilter(filter: OperationLogFilter): OperationLogFilter {
  const normalized: OperationLogFilter = {};
  for (const [key, value] of Object.entries(filter) as Array<
    [keyof OperationLogFilter, OperationLogFilter[keyof OperationLogFilter]]
  >) {
    if (typeof value === "string") {
      const trimmed = value.trim();
      if (trimmed) {
        normalized[key] = trimmed as never;
      }
      continue;
    }
    if (value !== undefined && value !== null) {
      normalized[key] = value as never;
    }
  }

  return {
    ...normalized,
    limit: normalized.limit ?? DEFAULT_LIMIT,
    offset: normalized.offset ?? 0,
  };
}

export const useOperationLogStore = create<OperationLogState>((set, get) => ({
  entries: [],
  total: 0,
  filter: {
    limit: DEFAULT_LIMIT,
    offset: 0,
  },
  selectedEntry: null,
  isLoading: false,
  isLoadingDetail: false,
  isClearing: false,
  isExporting: false,
  error: null,
  dailyCounts: [],
  isDailyCountsLoading: false,
  dailyCountsError: null,

  loadLogs: async (filter, reset = true) => {
    const nextFilter = normalizeFilter({
      ...get().filter,
      ...filter,
      offset: reset ? 0 : (filter?.offset ?? get().filter.offset),
    });

    set({ isLoading: true, error: null, filter: nextFilter });
    try {
      const page = await invoke("list_operation_logs", {
        filter: nextFilter,
      });
      set({
        entries: reset ? page.entries : [...get().entries, ...page.entries],
        total: page.total,
        filter: {
          ...nextFilter,
          limit: page.limit,
          offset: page.offset,
        },
        isLoading: false,
      });
      return page;
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  loadMore: async () => {
    const state = get();
    if (state.isLoading) return null;
    if (state.total > 0 && state.entries.length >= state.total) return null;
    return get().loadLogs({ offset: state.entries.length }, false);
  },

  loadDailyCounts: async (days) => {
    const currentToken = ++dailyCountsToken;
    set({ isDailyCountsLoading: true, dailyCountsError: null });
    try {
      const dailyCounts = await invoke("get_daily_operation_counts", { days });
      if (currentToken === dailyCountsToken) {
        set({ dailyCounts: dailyCounts ?? [], isDailyCountsLoading: false });
      }
    } catch (err) {
      if (currentToken === dailyCountsToken) {
        set({ dailyCountsError: String(err), isDailyCountsLoading: false });
      }
    }
  },

  loadLogDetail: async (logId) => {    set({ isLoadingDetail: true, error: null });
    try {
      const entry = await invoke("get_operation_log", { logId });
      set({ selectedEntry: entry, isLoadingDetail: false });
      return entry;
    } catch (err) {
      set({ error: String(err), isLoadingDetail: false });
      throw err;
    }
  },

  setFilter: (partial) => {
    set((state) => ({
      filter: normalizeFilter({
        ...state.filter,
        ...partial,
        offset: 0,
      }),
    }));
  },

  clearFilters: () => {
    set({
      filter: {
        limit: DEFAULT_LIMIT,
        offset: 0,
      },
    });
  },

  clearLogs: async (filter) => {
    const targetFilter = normalizeFilter(filter ?? get().filter);
    set({ isClearing: true, error: null });
    try {
      const deleted = await invoke("clear_operation_logs", {
        filter: targetFilter,
      });
      await get().loadLogs({ ...get().filter, offset: 0 });
      set({ isClearing: false });
      return deleted;
    } catch (err) {
      set({ error: String(err), isClearing: false });
      throw err;
    }
  },

  exportLogs: async (filter) => {
    const targetFilter = normalizeFilter(filter ?? get().filter);
    set({ isExporting: true, error: null });
    try {
      const payload = await invoke("export_operation_logs", {
        filter: targetFilter,
      });
      set({ isExporting: false });
      return payload;
    } catch (err) {
      set({ error: String(err), isExporting: false });
      throw err;
    }
  },

  closeDetail: () => set({ selectedEntry: null }),
  clearError: () => set({ error: null }),
}));
