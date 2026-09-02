import { invoke, listen, isTauriRuntime, type UnlistenFn } from "@/lib/ipc";
import type { GitHubImportProgressPayload } from "@/types/githubImport";
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

/**
 * Release a preview snapshot the renderer will no longer use.
 *
 * Best effort: a consumed or expired token is already gone on the backend, so a
 * rejection here must not surface as a user-facing failure.
 */
async function discardGitHubRepoPreviewSnapshot(previewId?: string | null) {
  if (!previewId || !isTauriRuntime()) {
    return;
  }
  await Promise.resolve(
    invoke("discard_github_repo_preview_snapshot", { previewId }),
  ).catch(() => undefined);
}

export {
  cleanupGitHubImportAiSummaryListener,
  createInitialGitHubImportState,
  createMarketplaceBaseState,
  discardGitHubRepoPreviewSnapshot,
  setupGitHubImportEventListeners,
};

export function rememberGitHubImportAiUnlistener(
  sourcePath: string,
  unlisten: UnlistenFn,
) {
  githubImportAiUnlisteners.set(sourcePath, unlisten);
}

export function teardownGitHubImportProgressListener() {
  if (unlistenGitHubImportProgress) {
    unlistenGitHubImportProgress();
    unlistenGitHubImportProgress = null;
  }
}
