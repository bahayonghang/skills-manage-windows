import { UnlistenFn } from "@tauri-apps/api/event";
import { invoke, listen, isTauriRuntime } from "@/lib/ipc";
import { GitHubImportProgressPayload } from "@/types";
import {
  createInitialGitHubImportState,
  createMarketplaceBaseState,
} from "./marketplaceStore.shared";
import type { MarketplaceState } from "./marketplaceStore.types";

let unlistenGitHubImportProgress: UnlistenFn | null = null;
const githubImportAiUnlisteners = new Map<string, UnlistenFn>();

function cleanupGitHubImportAiSummaryListener(sourcePath?: string) {
  if (sourcePath) {
    githubImportAiUnlisteners.get(sourcePath)?.();
    githubImportAiUnlisteners.delete(sourcePath);
    return;
  }

  for (const unlisten of githubImportAiUnlisteners.values()) {
    unlisten();
  }
  githubImportAiUnlisteners.clear();
}

async function setupGitHubImportEventListeners(
  set: (
    fn:
      | Partial<MarketplaceState>
      | ((s: MarketplaceState) => Partial<MarketplaceState>),
  ) => void,
  getGeneration: () => number,
  generation: number,
) {
  if (unlistenGitHubImportProgress) {
    unlistenGitHubImportProgress();
    unlistenGitHubImportProgress = null;
  }

  unlistenGitHubImportProgress = await listen<GitHubImportProgressPayload>(
    "github-import:progress",
    (event) => {
      if (generation !== getGeneration()) {
        return;
      }
      set((state) => ({
        githubImport: {
          ...state.githubImport,
          importProgress: event.payload,
        },
      }));
    },
  );
}

async function discardGitHubRepoPreviewWorkspace(
  previewWorkspaceId?: string | null,
) {
  if (!previewWorkspaceId || !isTauriRuntime()) {
    return;
  }
  await Promise.resolve(
    invoke("discard_github_repo_preview_workspace", {
      workspaceId: previewWorkspaceId,
    }),
  ).catch(() => undefined);
}

export {
  cleanupGitHubImportAiSummaryListener,
  createInitialGitHubImportState,
  createMarketplaceBaseState,
  discardGitHubRepoPreviewWorkspace,
  setupGitHubImportEventListeners,
};

export function rememberGitHubImportAiUnlistener(sourcePath: string, unlisten: UnlistenFn) {
  githubImportAiUnlisteners.set(sourcePath, unlisten);
}

export function teardownGitHubImportProgressListener() {
  if (unlistenGitHubImportProgress) {
    unlistenGitHubImportProgress();
    unlistenGitHubImportProgress = null;
  }
}
