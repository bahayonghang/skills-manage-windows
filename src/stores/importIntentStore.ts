import { useCallback } from "react";
import { create } from "zustand";
import { useMarketplaceStore } from "@/stores/marketplaceStore";

const MAX_PENDING_IMPORT_INTENTS = 8;

export type ImportIntent = {
  kind: "github";
  source?: string;
};

type OpenImportIntentResult = "opened" | "pending" | "duplicate" | "invalid";

interface ImportIntentState {
  githubWizardOpen: boolean;
  githubSource: string;
  githubBranch: string;
  pendingSources: string[];
  openImportIntent: (intent: ImportIntent) => OpenImportIntentResult;
  setGitHubWizardOpen: (open: boolean) => void;
  setGitHubSource: (source: string) => void;
  setGitHubBranch: (branch: string) => void;
  consumePendingIntent: () => void;
  discardPendingIntent: () => void;
  resetGitHubSession: () => void;
  resetForTest: () => void;
}

function validateEventSource(source: string): string | null {
  if (source.length === 0 || source.length > 4096) {
    return null;
  }

  let decoded = source;
  for (let depth = 0; depth < 3; depth += 1) {
    if (/[\u0000-\u001f\u007f-\u009f\\]/u.test(decoded)) return null;
    const path = decoded.split(/[?#]/u, 1)[0].replace(/\\/gu, "/");
    if (path.split("/").some((segment) => segment === "." || segment === "..")) {
      return null;
    }
    try {
      const next = decodeURIComponent(decoded);
      if (next === decoded) break;
      decoded = next;
    } catch {
      return null;
    }
  }

  try {
    const url = new URL(source);
    if (
      url.protocol !== "https:" ||
      url.hostname !== "github.com" ||
      url.username !== "" ||
      url.password !== "" ||
      url.port !== "" ||
      url.search !== "" ||
      url.hash !== ""
    ) {
      return null;
    }
    const segments = url.pathname.split("/").filter(Boolean);
    if (
      segments.length < 2 ||
      segments.some((segment) => segment === "." || segment === "..")
    ) {
      return null;
    }
    return source;
  } catch {
    return null;
  }
}

function hasDirtyGitHubSession(state: ImportIntentState): boolean {
  const githubImport = useMarketplaceStore.getState().githubImport;
  return (
    state.githubSource.trim().length > 0 ||
    state.githubBranch.trim().length > 0 ||
    githubImport.isPreviewLoading ||
    githubImport.isImporting ||
    githubImport.preview !== null ||
    githubImport.importResult !== null ||
    githubImport.error !== null ||
    githubImport.previewedRepoUrl !== null
  );
}

export const useImportIntentStore = create<ImportIntentState>((set, get) => ({
  githubWizardOpen: false,
  githubSource: "",
  githubBranch: "",
  pendingSources: [],

  openImportIntent: (intent) => {
    if (intent.kind !== "github") return "invalid";
    if (intent.source === undefined) {
      set({ githubWizardOpen: true });
      return "opened";
    }

    const source = validateEventSource(intent.source);
    if (!source) return "invalid";

    const state = get();
    if (
      state.githubSource === source ||
      state.pendingSources.includes(source)
    ) {
      return "duplicate";
    }

    if (hasDirtyGitHubSession(state)) {
      const pendingSources = [...state.pendingSources, source];
      set({
        pendingSources: pendingSources.slice(-MAX_PENDING_IMPORT_INTENTS),
      });
      return "pending";
    }

    set({ githubSource: source, githubBranch: "", githubWizardOpen: true });
    return "opened";
  },

  setGitHubWizardOpen: (githubWizardOpen) => set({ githubWizardOpen }),
  setGitHubSource: (githubSource) => set({ githubSource }),
  setGitHubBranch: (githubBranch) => set({ githubBranch }),

  consumePendingIntent: () => {
    const [source, ...pendingSources] = get().pendingSources;
    if (!source || get().githubWizardOpen) return;
    useMarketplaceStore.getState().resetGitHubImport();
    set({
      githubSource: source,
      githubBranch: "",
      githubWizardOpen: true,
      pendingSources,
    });
  },

  discardPendingIntent: () => {
    const [, ...pendingSources] = get().pendingSources;
    set({ pendingSources });
  },

  resetGitHubSession: () => {
    useMarketplaceStore.getState().resetGitHubImport();
    set({ githubSource: "", githubBranch: "" });
  },

  resetForTest: () =>
    set({
      githubWizardOpen: false,
      githubSource: "",
      githubBranch: "",
      pendingSources: [],
    }),
}));

export function openImportIntent(intent: ImportIntent): OpenImportIntentResult {
  return useImportIntentStore.getState().openImportIntent(intent);
}

export function useImportIntentBindings() {
  const isGitHubImportOpen = useImportIntentStore(
    (state) => state.githubWizardOpen,
  );
  const setGitHubWizardOpen = useImportIntentStore(
    (state) => state.setGitHubWizardOpen,
  );
  const githubRepoUrl = useImportIntentStore((state) => state.githubSource);
  const githubBranch = useImportIntentStore((state) => state.githubBranch);
  const setGithubRepoUrl = useImportIntentStore(
    (state) => state.setGitHubSource,
  );
  const setGithubBranch = useImportIntentStore(
    (state) => state.setGitHubBranch,
  );
  const setIsGitHubImportOpen = useCallback(
    (open: boolean) => {
      if (open) openImportIntent({ kind: "github" });
      else setGitHubWizardOpen(false);
    },
    [setGitHubWizardOpen],
  );

  return {
    githubBranch,
    githubRepoUrl,
    isGitHubImportOpen,
    setGithubBranch,
    setGithubRepoUrl,
    setIsGitHubImportOpen,
  };
}

export function parseImportIntentEvent(
  payload: unknown,
): ImportIntent | null {
  if (!payload || typeof payload !== "object") return null;
  const record = payload as Record<string, unknown>;
  if (Object.keys(record).length !== 1) return null;
  const source = record.source;
  if (typeof source !== "string" || !validateEventSource(source)) return null;
  return { kind: "github", source };
}
