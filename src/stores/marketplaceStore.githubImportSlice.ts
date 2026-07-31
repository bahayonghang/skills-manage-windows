import { setupExplanationStreamListeners } from "@/lib/explanationStream";
import { backendErrorStateValue } from "@/lib/backendError";
import i18n from "@/i18n";
import { invoke, isTauriRuntime } from "@/lib/ipc";
import { GitHubRepoRef, GitHubSkillImportSelection } from "@/types";
import {
  cleanupGitHubImportAiSummaryListener,
  createInitialGitHubImportState,
  createMarketplaceBaseState,
  discardGitHubRepoPreviewSnapshot,
  rememberGitHubImportAiUnlistener,
  setupGitHubImportEventListeners,
  teardownGitHubImportProgressListener,
} from "./marketplaceStore.githubImportHelpers";
import type {
  MarketplaceState,
  MarketplaceStoreContext,
} from "./marketplaceStore.types";

function normalizeGitHubBranch(branch?: string | null): string | null {
  return branch?.trim() || null;
}

export function createMarketplaceGitHubImportSlice({
  set,
  get,
  getGeneration,
  bumpGeneration,
}: MarketplaceStoreContext): Pick<
  MarketplaceState,
  | "previewGitHubRepoSkills"
  | "previewGitHubRepoImport"
  | "importGitHubRepoSkills"
  | "installGitHubPreviewSkill"
  | "fetchGitHubSkillMarkdown"
  | "generateGitHubImportAiSummary"
  | "resetGitHubImport"
  | "resetForTargetChange"
> {
  return {
    previewGitHubRepoSkills: async (
      repoUrl: string,
      branch?: string | null,
    ) => {
      if (!isTauriRuntime()) {
        throw new Error(
          "Desktop-only feature: GitHub repo preview is available in the Tauri app.",
        );
      }

      const preview = await invoke("preview_github_repo_import", {
        repoUrl,
        branch: normalizeGitHubBranch(branch),
      });
      await discardGitHubRepoPreviewSnapshot(preview.previewId);
      return preview;
    },

    previewGitHubRepoImport: async (
      repoUrl: string,
      branch?: string | null,
    ) => {
      const generation = getGeneration();
      const previewedBranch = normalizeGitHubBranch(branch);
      if (!isTauriRuntime()) {
        const error =
          "Desktop-only feature: GitHub repo preview is available in the Tauri app.";
        set((state) => ({
          githubImport: {
            ...state.githubImport,
            isPreviewLoading: false,
            preview: null,
            importResult: null,
            previewedRepoUrl: repoUrl,
            previewedBranch,
            error,
            importProgress: null,
            importStartedAt: null,
          },
        }));
        throw new Error(error);
      }

      await discardGitHubRepoPreviewSnapshot(
        get().githubImport.preview?.previewId,
      );

      set((state) => ({
        githubImport: {
          ...state.githubImport,
          isPreviewLoading: true,
          preview: null,
          importResult: null,
          previewedRepoUrl: repoUrl,
          previewedBranch,
          error: null,
          importProgress: null,
          importStartedAt: null,
        },
      }));

      try {
        const preview = await invoke("preview_github_repo_import", {
          repoUrl,
          branch: previewedBranch,
        });
        if (generation === getGeneration()) {
          set((state) => ({
            githubImport: {
              ...state.githubImport,
              isPreviewLoading: false,
              preview,
              importResult: null,
              previewedRepoUrl: repoUrl,
              previewedBranch,
              error: null,
              importProgress: null,
              importStartedAt: null,
            },
          }));
        }
        return preview;
      } catch (err) {
        if (generation === getGeneration()) {
          set((state) => ({
            githubImport: {
              ...state.githubImport,
              isPreviewLoading: false,
              preview: null,
              importResult: null,
              previewedRepoUrl: repoUrl,
              previewedBranch,
              error: backendErrorStateValue(err),
              importProgress: null,
              importStartedAt: null,
            },
          }));
        }
        throw err;
      }
    },

    importGitHubRepoSkills: async (
      repoUrl: string,
      selections: GitHubSkillImportSelection[],
      previewId?: string | null,
    ) => {
      const generation = getGeneration();
      if (!isTauriRuntime()) {
        const error =
          "Desktop-only feature: GitHub repo import is available in the Tauri app.";
        set((state) => ({
          githubImport: {
            ...state.githubImport,
            isImporting: false,
            error,
            importProgress: null,
            importStartedAt: null,
          },
        }));
        throw new Error(error);
      }

      const activePreviewId =
        previewId ?? get().githubImport.preview?.previewId ?? null;
      const previewedBranch = get().githubImport.previewedBranch;
      if (!activePreviewId) {
        const error = i18n.t("marketplace.githubImportPreviewRequired");
        set((state) => ({
          githubImport: {
            ...state.githubImport,
            isImporting: false,
            error,
            importProgress: null,
            importStartedAt: null,
          },
        }));
        throw new Error(error);
      }

      set((state) => ({
        githubImport: {
          ...state.githubImport,
          isImporting: true,
          error: null,
          importProgress: {
            phase: "preparing",
            currentSkill: null,
            currentPath: null,
            completedFiles: 0,
            totalFiles: 0,
            completedBytes: 0,
            totalBytes: 0,
          },
          importStartedAt: Date.now(),
        },
      }));

      try {
        await setupGitHubImportEventListeners(set, getGeneration, generation);

        const importResult = await invoke("import_github_repo_skills", {
          previewId: activePreviewId,
          repoUrl,
          branch: previewedBranch,
          selections,
        });
        if (generation === getGeneration()) {
          set((state) => ({
            githubImport: {
              ...state.githubImport,
              isImporting: false,
              importResult,
              error: null,
              importProgress: null,
              importStartedAt: null,
              // The backend consumes the snapshot on success; a further import or
              // Markdown read must start from a fresh preview.
              preview: null,
              skillMarkdown: {},
            },
          }));
        }
        return importResult;
      } catch (err) {
        if (generation === getGeneration()) {
          set((state) => ({
            githubImport: {
              ...state.githubImport,
              isImporting: false,
              error: backendErrorStateValue(err),
              importProgress: null,
              importStartedAt: null,
            },
          }));
        }
        throw err;
      }
    },

    installGitHubPreviewSkill: async (repoUrl: string, sourcePath: string) => {
      const preview = await get().previewGitHubRepoImport(repoUrl);
      const candidate = preview.skills.find(
        (skill) => skill.sourcePath === sourcePath,
      );
      if (!candidate) {
        await discardGitHubRepoPreviewSnapshot(preview.previewId);
        throw new Error(
          i18n.t("marketplace.previewSkillDisappeared", { skill: sourcePath }),
        );
      }

      return get().importGitHubRepoSkills(
        repoUrl,
        [
          {
            sourcePath: candidate.sourcePath,
            resolution: "overwrite",
            renamedSkillId: null,
          },
        ],
        preview.previewId,
      );
    },

    fetchGitHubSkillMarkdown: async (
      repo: GitHubRepoRef,
      sourcePath: string,
    ) => {
      const generation = getGeneration();
      const existing = get().githubImport.skillMarkdown[sourcePath];
      if (existing?.status === "loading" || existing?.status === "ready") {
        return;
      }

      if (!isTauriRuntime()) {
        set((state) => ({
          githubImport: {
            ...state.githubImport,
            skillMarkdown: {
              ...state.githubImport.skillMarkdown,
              [sourcePath]: {
                status: "error",
                error:
                  "Desktop-only feature: GitHub markdown preview is available in the Tauri app.",
              },
            },
          },
        }));
        return;
      }

      // Markdown is only readable from the registered preview snapshot; without
      // a token the user must preview the repository again.
      const previewId = get().githubImport.preview?.previewId ?? null;
      if (!previewId) {
        set((state) => ({
          githubImport: {
            ...state.githubImport,
            skillMarkdown: {
              ...state.githubImport.skillMarkdown,
              [sourcePath]: {
                status: "error",
                error: i18n.t("marketplace.githubImportPreviewRequired"),
              },
            },
          },
        }));
        return;
      }

      set((state) => ({
        githubImport: {
          ...state.githubImport,
          skillMarkdown: {
            ...state.githubImport.skillMarkdown,
            [sourcePath]: { status: "loading" },
          },
        },
      }));

      try {
        const content = await invoke("fetch_github_skill_markdown", {
          previewId,
          repo,
          sourcePath,
        });
        if (generation === getGeneration()) {
          set((state) => ({
            githubImport: {
              ...state.githubImport,
              skillMarkdown: {
                ...state.githubImport.skillMarkdown,
                [sourcePath]: { status: "ready", content },
              },
            },
          }));
        }
      } catch (err) {
        if (generation === getGeneration()) {
          set((state) => ({
            githubImport: {
              ...state.githubImport,
              skillMarkdown: {
                ...state.githubImport.skillMarkdown,
                [sourcePath]: {
                  status: "error",
                  error: backendErrorStateValue(err),
                },
              },
            },
          }));
        }
      }
    },

    generateGitHubImportAiSummary: async (
      sourcePath: string,
      skillName: string,
      content: string,
      lang: string,
      refresh = false,
    ) => {
      const generation = getGeneration();
      const existing = get().githubImport.aiSummaries[sourcePath];
      if (!refresh && (existing?.isLoading || existing?.summary)) {
        return;
      }

      if (!isTauriRuntime()) {
        set((state) => ({
          githubImport: {
            ...state.githubImport,
            aiSummaries: {
              ...state.githubImport.aiSummaries,
              [sourcePath]: {
                summary: null,
                isLoading: false,
                isStreaming: false,
                error: "AI summary requires the Tauri desktop runtime.",
              },
            },
          },
        }));
        return;
      }

      set((state) => ({
        githubImport: {
          ...state.githubImport,
          aiSummaries: {
            ...state.githubImport.aiSummaries,
            [sourcePath]: {
              summary: null,
              isLoading: true,
              isStreaming: false,
              error: null,
            },
          },
        },
      }));

      try {
        cleanupGitHubImportAiSummaryListener(sourcePath);
        const prompt =
          lang === "en"
            ? `Summarize this SKILL.md for import decisions in English. Use 3 short parts: 1) What it does 2) When to import it 3) Dependencies or cautions. Keep it concise.\n\nSkill: ${skillName}\n\n${content}`
            : `请基于下面的 SKILL.md 内容，生成适合导入决策的中文摘要。分成 3 个简短部分：1）做什么 2）什么时候值得导入 3）依赖或注意事项。保持简洁。\n\n技能名：${skillName}\n\n${content}`;

        const command = refresh
          ? "refresh_skill_explanation"
          : "explain_skill_stream";
        const skillId = `github-import:${sourcePath}`;
        const stopListening = await setupExplanationStreamListeners(skillId, {
          onChunk: (chunkText) => {
            if (generation !== getGeneration()) {
              return;
            }
            set((state) => ({
              githubImport: {
                ...state.githubImport,
                aiSummaries: {
                  ...state.githubImport.aiSummaries,
                  [sourcePath]: {
                    summary: `${state.githubImport.aiSummaries[sourcePath]?.summary ?? ""}${chunkText}`,
                    isLoading: false,
                    isStreaming: true,
                    error: null,
                  },
                },
              },
            }));
          },
          onComplete: (payload) => {
            if (generation !== getGeneration()) {
              return;
            }
            cleanupGitHubImportAiSummaryListener(sourcePath);
            set((state) => {
              const currentSummary =
                state.githubImport.aiSummaries[sourcePath]?.summary;
              const nextSummary = payload.explanation ?? currentSummary ?? null;
              const hasSummary = Boolean(nextSummary?.trim());
              return {
                githubImport: {
                  ...state.githubImport,
                  aiSummaries: {
                    ...state.githubImport.aiSummaries,
                    [sourcePath]: {
                      summary: hasSummary ? nextSummary : null,
                      isLoading: false,
                      isStreaming: false,
                      error: hasSummary
                        ? null
                        : "AI summary returned no content.",
                    },
                  },
                },
              };
            });
          },
          onError: (payload) => {
            if (generation !== getGeneration()) {
              return;
            }
            cleanupGitHubImportAiSummaryListener(sourcePath);
            set((state) => ({
              githubImport: {
                ...state.githubImport,
                aiSummaries: {
                  ...state.githubImport.aiSummaries,
                  [sourcePath]: {
                    summary: null,
                    isLoading: false,
                    isStreaming: false,
                    error: payload.error ?? "Unknown explanation error",
                  },
                },
              },
            }));
          },
        });
        rememberGitHubImportAiUnlistener(sourcePath, stopListening);
        await invoke(command, { skillId, content: prompt, lang });
        const summary = await invoke<string | null>("get_skill_explanation", {
          skillId,
          lang,
        }).catch(() => null);
        if (generation === getGeneration()) {
          set((state) => ({
            githubImport: {
              ...state.githubImport,
              aiSummaries: {
                ...state.githubImport.aiSummaries,
                [sourcePath]: {
                  summary:
                    summary ??
                    state.githubImport.aiSummaries[sourcePath]?.summary ??
                    null,
                  isLoading: false,
                  isStreaming: false,
                  error: null,
                },
              },
            },
          }));
        }
      } catch (err) {
        cleanupGitHubImportAiSummaryListener(sourcePath);
        if (generation === getGeneration()) {
          set((state) => ({
            githubImport: {
              ...state.githubImport,
              aiSummaries: {
                ...state.githubImport.aiSummaries,
                [sourcePath]: {
                  summary: null,
                  isLoading: false,
                  isStreaming: false,
                  error: backendErrorStateValue(err),
                },
              },
            },
          }));
        }
      }
    },

    resetGitHubImport: () => {
      void discardGitHubRepoPreviewSnapshot(
        get().githubImport.preview?.previewId,
      );
      teardownGitHubImportProgressListener();
      cleanupGitHubImportAiSummaryListener();
      set({ githubImport: createInitialGitHubImportState() });
    },

    resetForTargetChange: () => {
      bumpGeneration();
      void discardGitHubRepoPreviewSnapshot(
        get().githubImport.preview?.previewId,
      );
      teardownGitHubImportProgressListener();
      cleanupGitHubImportAiSummaryListener();
      set(createMarketplaceBaseState());
    },
  };
}
