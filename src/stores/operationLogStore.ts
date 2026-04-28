import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/tauri";
import {
  OperationLogEntry,
  OperationLogFilter,
  OperationLogPage,
} from "@/types";

const DEFAULT_LIMIT = 100;

const fixtureLogEntries: OperationLogEntry[] = [
  {
    id: "fixture-log-scan",
    createdAt: "2026-04-27T10:00:00Z",
    level: "info",
    targetKind: "local",
    targetId: "local",
    targetLabel: "Local",
    category: "scan",
    action: "scan.all",
    status: "succeeded",
    subjectType: "scan_root",
    subjectId: "all",
    subjectLabel: "All scan directories",
    summary: "Scanned 12 skills across 5 agents",
    detailsJson: JSON.stringify({ totalSkills: 12, agentsScanned: 5 }, null, 2),
    durationMs: 240,
  },
  {
    id: "fixture-log-import",
    createdAt: "2026-04-27T09:30:00Z",
    level: "warn",
    targetKind: "ssh",
    targetId: "ssh-demo",
    targetLabel: "Remote Demo",
    category: "install",
    action: "central.batch_install",
    status: "partial",
    subjectType: "batch",
    subjectId: "central.batch_install",
    subjectLabel: "Central batch install",
    summary: "Installed 3 Central skill target(s), 1 failed",
    errorSummary: "One target platform was unavailable.",
    detailsJson: JSON.stringify({ succeeded: 3, failed: 1 }, null, 2),
    durationMs: 1080,
  },
];

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

  loadLogs: (filter?: OperationLogFilter, reset?: boolean) => Promise<OperationLogPage>;
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

function matchesFixture(entry: OperationLogEntry, filter: OperationLogFilter): boolean {
  const query = filter.query?.toLowerCase();
  if (query) {
    const haystack = [
      entry.summary,
      entry.errorSummary,
      entry.subjectId,
      entry.subjectLabel,
      entry.action,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    if (!haystack.includes(query)) return false;
  }
  if (filter.targetKind && entry.targetKind !== filter.targetKind) return false;
  if (filter.targetId && entry.targetId !== filter.targetId) return false;
  if (filter.level && entry.level !== filter.level) return false;
  if (filter.status && entry.status !== filter.status) return false;
  if (filter.category && entry.category !== filter.category) return false;
  if (filter.action && entry.action !== filter.action) return false;
  if (filter.createdAfter && entry.createdAt < filter.createdAfter) return false;
  if (filter.createdBefore && entry.createdAt > filter.createdBefore) return false;
  return true;
}

function fixturePage(filter: OperationLogFilter): OperationLogPage {
  const normalized = normalizeFilter(filter);
  const matched = fixtureLogEntries.filter((entry) => matchesFixture(entry, normalized));
  const offset = normalized.offset ?? 0;
  const limit = normalized.limit ?? DEFAULT_LIMIT;

  return {
    entries: matched.slice(offset, offset + limit),
    total: matched.length,
    limit,
    offset,
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

  loadLogs: async (filter, reset = true) => {
    const nextFilter = normalizeFilter({
      ...get().filter,
      ...filter,
      offset: reset ? 0 : filter?.offset ?? get().filter.offset,
    });

    set({ isLoading: true, error: null, filter: nextFilter });
    try {
      const page = isTauriRuntime()
        ? await invoke<OperationLogPage>("list_operation_logs", { filter: nextFilter })
        : fixturePage(nextFilter);
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

  loadLogDetail: async (logId) => {
    set({ isLoadingDetail: true, error: null });
    try {
      const entry = isTauriRuntime()
        ? await invoke<OperationLogEntry | null>("get_operation_log", { logId })
        : fixtureLogEntries.find((item) => item.id === logId) ?? null;
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
      const deleted = isTauriRuntime()
        ? await invoke<number>("clear_operation_logs", { filter: targetFilter })
        : fixtureLogEntries.filter((entry) => matchesFixture(entry, targetFilter)).length;
      if (isTauriRuntime()) {
        await get().loadLogs({ ...get().filter, offset: 0 });
      } else {
        set({
          entries: fixturePage(get().filter).entries,
          total: fixturePage(get().filter).total,
        });
      }
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
      const payload = isTauriRuntime()
        ? await invoke<string>("export_operation_logs", { filter: targetFilter })
        : JSON.stringify(
            {
              exportedAt: new Date().toISOString(),
              filter: targetFilter,
              total: fixturePage(targetFilter).total,
              entries: fixturePage(targetFilter).entries,
            },
            null,
            2
          );
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
