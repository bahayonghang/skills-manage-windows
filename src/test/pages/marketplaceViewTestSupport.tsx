/* eslint-disable react-refresh/only-export-components */
import { vi } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import type {
  AgentWithStatus,
  GitHubRepoImportResult,
  GitHubRepoPreview,
  MarketplaceSkill,
  SkillRegistry,
  SkillWithLinks,
  SkillsShSkill,
  TargetSummary,
} from "@/types";
import { looksLikeGitHubAuthGuidance } from "@/components/marketplace/githubImportWizardUtils";
import { isRemoteLikeTarget, requiresSshPasswordRepair } from "@/lib/targetKind";

const mockLoadRegistries = vi.fn();
const mockLoadPreviewSkills = vi.fn<() => Promise<MarketplaceSkill[]>>();
const mockGetNormalizedRegistryIdentity = vi.fn<(url: string) => string | null>();
const mockInstallSkill = vi.fn();
const mockPreviewGitHubRepoImport = vi.fn();
const mockImportGitHubRepoSkills = vi.fn();
const mockResetGitHubImport = vi.fn();
const mockSearchSkillsSh = vi.fn<() => Promise<SkillsShSkill[]>>();
const mockInstallFromSkillsSh = vi.fn();
const mockRescan = vi.fn();
const mockLoadCentralSkills = vi.fn();
const mockInstallCentralSkill = vi.fn();
const mockGetSkillsByAgent = vi.fn();

export const platformAgents: AgentWithStatus[] = [
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "~/.claude/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

type StoreState = {
  registries: SkillRegistry[];
  installingIds: Set<string>;
  skillsShResults: SkillsShSkill[];
  skillsShQuery: string;
  isSkillsShLoading: boolean;
  skillsShError: string | null;
  centralSkills: SkillWithLinks[];
  githubImport: {
    isPreviewLoading: boolean;
    isImporting: boolean;
    preview: GitHubRepoPreview | null;
    importResult: GitHubRepoImportResult | null;
    previewedRepoUrl: string | null;
    error: string | null;
  };
};

export const storeState: StoreState = {
  registries: [],
  installingIds: new Set<string>(),
  skillsShResults: [],
  skillsShQuery: "",
  isSkillsShLoading: false,
  skillsShError: null,
  centralSkills: [],
  githubImport: {
    isPreviewLoading: false,
    isImporting: false,
    preview: null,
    importResult: null,
    previewedRepoUrl: null,
    error: null,
  },
};

function normalizeRegistryIdentity(url: string): string | null {
  const trimmed = url.trim();
  if (!trimmed) return null;
  const githubMatch = trimmed.match(
    /^(?:https?:\/\/)?(?:www\.)?github\.com\/([^/\s]+)\/([^/\s#?]+?)(?:\.git)?(?:\/)?$/i,
  );
  if (githubMatch) {
    return `github:${githubMatch[1].toLowerCase()}/${githubMatch[2].toLowerCase()}`;
  }
  return trimmed.toLowerCase();
}

export function makeRegistry(id: string, url: string): SkillRegistry {
  return {
    id,
    name: id,
    source_type: "github",
    url,
    normalized_url: normalizeRegistryIdentity(url),
    is_builtin: true,
    is_enabled: true,
    last_synced: "2026-04-16T00:00:00Z",
    last_attempted_sync: "2026-04-16T00:10:00Z",
    last_sync_status: "success",
    last_sync_error: null,
    cache_updated_at: "2026-04-16T00:00:00Z",
    cache_expires_at: "2026-04-17T00:00:00Z",
    etag: null,
    last_modified: null,
    created_at: "2026-04-15T00:00:00Z",
  };
}

export function makePreview(
  skills: GitHubRepoPreview["skills"],
  previewId = "github-preview-view-support",
): GitHubRepoPreview {
  return {
    repo: {
      owner: "openai",
      repo: "skills",
      branch: "main",
      normalizedUrl: "https://github.com/openai/skills",
    },
    skills,
    previewId,
    resolvedCommitSha: "0f1e2d3c4b5a69788796a5b4c3d2e1f001234567",
    snapshotDigest: `sha256-v1:${"d".repeat(64)}`,
    expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
  };
}

vi.mock("@/components/skill/UnifiedSkillCard", () => ({
  UnifiedSkillCard: ({
    name,
    description,
  onDetail,
  publisher,
  onInstall,
  installLabel,
  }: {
    name: string;
    description?: string;
    onDetail?: () => void;
    publisher?: string;
    onInstall?: () => void;
    installLabel?: string;
  }) => (
    <div>
      <button type="button" onClick={onDetail}>
        {name}
      </button>
      {description ? <div>{description}</div> : null}
      {publisher ? <div>{publisher}</div> : null}
      {onInstall ? (
        <button type="button" onClick={onInstall}>
          {installLabel ?? "安装"}
        </button>
      ) : null}
    </div>
  ),
}));

vi.mock("@/components/central/InstallDialog", () => ({
  InstallDialog: ({
    open,
    skill,
  }: {
    open: boolean;
    skill: SkillWithLinks | null;
  }) => (open ? <div role="dialog">Central install {skill?.name}</div> : null),
}));

vi.mock("@/components/marketplace/GitHubRepoImportWizard", async () => {
  const React = await vi.importActual<typeof import("react")>("react");
  const { useTargetStore } =
    await vi.importActual<typeof import("@/stores/targetStore")>("@/stores/targetStore");

  return {
    GitHubRepoImportWizard: ({
      open,
      repoUrl,
      onRepoUrlChange,
      preview,
      previewError,
      importResult,
      onImport,
      onOpenChange,
    }: {
      open: boolean;
      repoUrl: string;
      onRepoUrlChange: (value: string) => void;
      preview: GitHubRepoPreview | null;
      previewError: string | null;
      importResult: GitHubRepoImportResult | null;
      onImport: (
        selections: Array<{
          sourcePath: string;
          resolution: "skip" | "overwrite" | "rename";
          renamedSkillId?: string | null;
        }>,
      ) => Promise<GitHubRepoImportResult | void> | GitHubRepoImportResult | void;
      onOpenChange: (open: boolean) => void;
    }) => {
      const activeTarget = useTargetStore((state) => state.activeTarget);
      const updateSshTargetPassword = useTargetStore(
        (state) => state.updateSshTargetPassword,
      );
      const [step, setStep] = React.useState<"input" | "preview" | "confirm" | "result">(
        importResult ? "result" : preview ? "preview" : "input",
      );
      const [selectedSkillPath, setSelectedSkillPath] = React.useState<string | null>(
        preview?.skills[0]?.sourcePath ?? null,
      );
      const [selectionState, setSelectionState] = React.useState<
        Record<
          string,
          {
            selected: boolean;
            resolution: "skip" | "overwrite" | "rename";
            renamedSkillId: string | null;
          }
        >
      >({});
      const [isRenameEditing, setIsRenameEditing] = React.useState(false);
      const [renameValue, setRenameValue] = React.useState("");
      const [passwordValue, setPasswordValue] = React.useState("");
      const [statusMessage, setStatusMessage] = React.useState<string | null>(null);
      const [localResult, setLocalResult] = React.useState<GitHubRepoImportResult | null>(
        importResult,
      );

      React.useEffect(() => {
        if (!open) {
          setStep("input");
          setSelectedSkillPath(null);
          setSelectionState({});
          setIsRenameEditing(false);
          setRenameValue("");
          setPasswordValue("");
          setStatusMessage(null);
          setLocalResult(null);
          return;
        }

        if (importResult) {
          setLocalResult(importResult);
          setStep("result");
          return;
        }

        if (preview) {
          setSelectionState(
            Object.fromEntries(
              preview.skills.map((skill) => [
                skill.sourcePath,
                {
                  selected: true,
                  resolution: skill.conflict ? "skip" : "overwrite",
                  renamedSkillId: skill.skillId,
                },
              ]),
            ),
          );
          setSelectedSkillPath((current) =>
            current && preview.skills.some((skill) => skill.sourcePath === current)
              ? current
              : (preview.skills[0]?.sourcePath ?? null),
          );
          setStep("preview");
          return;
        }

        setStep("input");
        setSelectedSkillPath(null);
        setSelectionState({});
      }, [importResult, open, preview]);

      if (!open) return null;

      const previewSkills = preview?.skills ?? [];
      const selectedPreviewSkill =
        previewSkills.find((skill) => skill.sourcePath === selectedSkillPath) ??
        previewSkills[0] ??
        null;
      const selectedSkills = previewSkills.filter(
        (skill) => selectionState[skill.sourcePath]?.selected ?? false,
      );
      const showRemoteWorkspaceHint =
        Boolean(preview?.previewId) && isRemoteLikeTarget(activeTarget);
      const needsSshPasswordRepair =
        step === "confirm" &&
        requiresSshPasswordRepair(activeTarget) &&
        !activeTarget.hasStoredPassword;
      const showSshPasswordRepair = needsSshPasswordRepair || Boolean(statusMessage);
      const footerMode = step === "confirm" ? "confirm" : "preview";

      async function handleImportConfirm() {
        const payload = selectedSkills.map((skill) => {
          const state = selectionState[skill.sourcePath];
          return {
            sourcePath: skill.sourcePath,
            resolution: state?.resolution ?? "overwrite",
            renamedSkillId: state?.renamedSkillId ?? skill.skillId,
          };
        });

        const result = await onImport(payload);
        if (result) {
          setLocalResult(result);
          setStep("result");
        }
      }

      async function handleSavePassword() {
        const result = await updateSshTargetPassword(activeTarget.id, passwordValue);
        if (result.credentialStatus === "session") {
          setStatusMessage("Password available for this session only.");
        } else {
          setStatusMessage(`SSH password saved for ${activeTarget.label}.`);
        }
      }

      return (
        <div role="dialog" aria-label="GitHub import wizard">
          {step === "input" ? (
            <div>
              <label>
                GitHub repository URL
                <input
                  aria-label="GitHub repository URL"
                  value={repoUrl}
                  onChange={(event) => onRepoUrlChange(event.target.value)}
                />
              </label>
              {previewError && looksLikeGitHubAuthGuidance(previewError) ? (
                <div>GitHub Personal Access Token</div>
              ) : null}
            </div>
          ) : null}

          {step === "preview" && preview ? (
            <div data-testid="github-import-preview-workspace">
              <div data-testid="github-import-repo-toolbar">
                selected:{selectedSkills.length}
              </div>
              {showRemoteWorkspaceHint ? (
                <div data-testid="github-import-remote-workspace-hint">
                  Preview workspace is on the active SSH target.
                </div>
              ) : null}
              <div data-testid="github-import-bulk-selection-controls">
                <button
                  type="button"
                  disabled={selectedSkills.length === previewSkills.length}
                  onClick={() => {
                    setSelectionState((current) =>
                      Object.fromEntries(
                        previewSkills.map((skill) => [
                          skill.sourcePath,
                          {
                            ...(current[skill.sourcePath] ?? {
                              resolution: skill.conflict ? "skip" : "overwrite",
                              renamedSkillId: skill.skillId,
                            }),
                            selected: true,
                          },
                        ]),
                      ),
                    );
                  }}
                >
                  Select all
                </button>
                <button
                  type="button"
                  disabled={selectedSkills.length === 0}
                  onClick={() => {
                    setSelectionState((current) =>
                      Object.fromEntries(
                        previewSkills.map((skill) => [
                          skill.sourcePath,
                          {
                            ...(current[skill.sourcePath] ?? {
                              resolution: skill.conflict ? "skip" : "overwrite",
                              renamedSkillId: skill.skillId,
                            }),
                            selected: false,
                          },
                        ]),
                      ),
                    );
                  }}
                >
                  Deselect all
                </button>
              </div>
              <div data-testid="github-import-summary-list">
                {previewSkills.map((skill) => {
                  const currentState = selectionState[skill.sourcePath];
                  return (
                    <div key={skill.sourcePath}>
                      <input
                        type="checkbox"
                        checked={currentState?.selected ?? false}
                        readOnly
                        aria-label={`select ${skill.skillName}`}
                      />
                      <button type="button" onClick={() => setSelectedSkillPath(skill.sourcePath)}>
                        {skill.skillName}
                      </button>
                    </div>
                  );
                })}
              </div>
              <div data-testid="github-import-detail-pane">
                {selectedPreviewSkill ? <div>{selectedPreviewSkill.skillName}</div> : null}
                {selectedPreviewSkill?.conflict ? (
                  isRenameEditing ? (
                    <div>
                      <input
                        placeholder="New skill id"
                        value={renameValue}
                        onChange={(event) => setRenameValue(event.target.value)}
                      />
                      <button
                        type="button"
                        onClick={() => {
                          setSelectionState((current) => ({
                            ...current,
                            [selectedPreviewSkill.sourcePath]: {
                              ...(current[selectedPreviewSkill.sourcePath] ?? {
                                selected: true,
                                resolution: "rename",
                                renamedSkillId: selectedPreviewSkill.skillId,
                              }),
                              resolution: "rename",
                              renamedSkillId: renameValue,
                            },
                          }));
                          setIsRenameEditing(false);
                        }}
                      >
                        Confirm
                      </button>
                    </div>
                  ) : (
                    <button
                      type="button"
                      onClick={() => {
                        setRenameValue(
                          selectionState[selectedPreviewSkill.sourcePath]?.renamedSkillId ??
                            selectedPreviewSkill.skillId,
                        );
                        setIsRenameEditing(true);
                      }}
                    >
                      Rename
                    </button>
                  )
                ) : null}
              </div>
            </div>
          ) : null}

          {step === "confirm" ? (
            <div>
              <div data-testid="github-import-confirm-summary">
                {selectedSkills
                  .map(
                    (skill) =>
                      selectionState[skill.sourcePath]?.renamedSkillId ?? skill.skillId,
                  )
                  .join(",")}
              </div>
              {showSshPasswordRepair ? (
                <div data-testid="github-import-ssh-password-repair">
                  <div>Save the active SSH password</div>
                  <label>
                    SSH password for {activeTarget.label}
                    <input
                      aria-label={`SSH password for ${activeTarget.label}`}
                      value={passwordValue}
                      onChange={(event) => setPasswordValue(event.target.value)}
                    />
                  </label>
                  <button
                    type="button"
                    disabled={!passwordValue.trim()}
                    onClick={() => {
                      void handleSavePassword();
                    }}
                  >
                    Save password
                  </button>
                  {statusMessage ? <div>{statusMessage}</div> : null}
                </div>
              ) : null}
            </div>
          ) : null}

          {step === "result" && (localResult ?? importResult) ? (
            <div data-testid="github-import-result-hub">
              <button type="button">Continue platform setup</button>
              <button type="button">Open Central</button>
              <button
                type="button"
                onClick={() => {
                  setLocalResult(null);
                  setStep(preview ? "preview" : "input");
                }}
              >
                Start another import
              </button>
              {((localResult ?? importResult)?.skippedSkills ?? []).map((skillId) => (
                <div key={skillId}>{skillId}</div>
              ))}
            </div>
          ) : null}

          <div data-testid="github-import-shell-footer" data-footer-mode={footerMode}>
            {step === "preview" ? (
              <button
                type="button"
                disabled={selectedSkills.length === 0}
                onClick={() => setStep("confirm")}
              >
                Review import
              </button>
            ) : null}
            {step === "confirm" ? (
              <button
                type="button"
                disabled={needsSshPasswordRepair || selectedSkills.length === 0}
                onClick={() => {
                  void handleImportConfirm();
                }}
              >
                Import
              </button>
            ) : null}
          </div>

          <button type="button" onClick={() => onOpenChange(false)}>
            Close
          </button>
        </div>
      );
    },
  };
});

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/stores/marketplaceStore", () => ({
  useMarketplaceStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      registries: storeState.registries,
      installingIds: storeState.installingIds,
      skillsShResults: storeState.skillsShResults,
      skillsShQuery: storeState.skillsShQuery,
      isSkillsShLoading: storeState.isSkillsShLoading,
      skillsShError: storeState.skillsShError,
      githubImport: storeState.githubImport,
      loadRegistries: mockLoadRegistries,
      loadPreviewSkills: mockLoadPreviewSkills,
      getNormalizedRegistryIdentity: mockGetNormalizedRegistryIdentity,
      installSkill: mockInstallSkill,
      searchSkillsSh: mockSearchSkillsSh,
      installFromSkillsSh: mockInstallFromSkillsSh,
      previewGitHubRepoImport: mockPreviewGitHubRepoImport,
      importGitHubRepoSkills: mockImportGitHubRepoSkills,
      resetGitHubImport: mockResetGitHubImport,
    }),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      rescan: mockRescan,
      agents: platformAgents,
    }),
}));

vi.mock("@/stores/centralSkillsStore", () => {
  const getState = () => ({
      skills: storeState.centralSkills,
      agents: platformAgents,
      loadCentralSkills: mockLoadCentralSkills,
      installSkill: mockInstallCentralSkill,
    });
  const useCentralSkillsStore = Object.assign(
    (selector: (state: Record<string, unknown>) => unknown) => selector(getState()),
    { getState }
  );
  return { useCentralSkillsStore };
});

vi.mock("@/stores/skillStore", () => ({
  useSkillStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      skillsByAgent: {},
      getSkillsByAgent: mockGetSkillsByAgent,
    }),
}));

import { MarketplaceView as MarketplaceViewComponent } from "@/pages/MarketplaceView";
import { useImportIntentStore } from "@/stores/importIntentStore";
import * as tauriBridgeModule from "@/lib/ipc";
import { useTargetStore as targetStoreHook } from "@/stores/targetStore";

export const MarketplaceView = MarketplaceViewComponent;
export const tauriBridge = tauriBridgeModule;
export const useTargetStore = targetStoreHook;
export const localTarget: TargetSummary = {
  id: "local",
  kind: "local",
  label: "Local",
  isActive: true,
};
export const defaultUpdateSshTargetPassword =
  useTargetStore.getState().updateSshTargetPassword;

export function renderMarketplaceView() {
  return render(
    <MemoryRouter>
      <MarketplaceViewComponent />
    </MemoryRouter>,
  );
}

export function resetMarketplaceViewTestState() {
  useImportIntentStore.getState().resetForTest();
  mockLoadRegistries.mockReset();
  mockLoadPreviewSkills.mockReset();
  mockGetNormalizedRegistryIdentity.mockReset();
  mockInstallSkill.mockReset();
  mockPreviewGitHubRepoImport.mockReset();
  mockImportGitHubRepoSkills.mockReset();
  mockResetGitHubImport.mockReset();
  mockSearchSkillsSh.mockReset();
  mockInstallFromSkillsSh.mockReset();
  mockRescan.mockReset();
  mockLoadCentralSkills.mockReset();
  mockInstallCentralSkill.mockReset();
  mockGetSkillsByAgent.mockReset();

  mockGetNormalizedRegistryIdentity.mockImplementation(normalizeRegistryIdentity);
  mockLoadPreviewSkills.mockResolvedValue([
    {
      id: "openai::knowledge-work-plugin",
      registry_id: "openai",
      name: "Knowledge Work Plugin",
      description: "Useful repo preview content",
      download_url: "https://example.com/openai/knowledge-work-plugin/SKILL.md",
      is_installed: false,
      synced_at: "2026-04-16T00:00:00Z",
      cache_updated_at: "2026-04-16T00:00:00Z",
    },
  ]);

  storeState.registries = [makeRegistry("openai", "https://github.com/openai/skills")];
  storeState.installingIds = new Set<string>();
  storeState.skillsShResults = [
    {
      id: "anthropics/skills/webapp-testing",
      skill_id: "webapp-testing",
      name: "webapp-testing",
      source: "anthropics/skills",
      installs: 68897,
      stars: 1234,
    },
  ];
  storeState.skillsShQuery = "webapp";
  storeState.isSkillsShLoading = false;
  storeState.skillsShError = null;
  storeState.centralSkills = [];
  storeState.githubImport = {
    isPreviewLoading: false,
    isImporting: false,
    preview: null,
    importResult: null,
    previewedRepoUrl: null,
    error: null,
  };

  useTargetStore.setState({
    targets: [localTarget],
    activeTarget: localTarget,
    error: null,
    updateSshTargetPassword: defaultUpdateSshTargetPassword,
  });
}

export function cleanupMarketplaceViewTestState() {
  cleanup();
  vi.restoreAllMocks();
}

export {
  mockGetNormalizedRegistryIdentity,
  mockGetSkillsByAgent,
  mockImportGitHubRepoSkills,
  mockInstallFromSkillsSh,
  mockInstallCentralSkill,
  mockInstallSkill,
  mockLoadCentralSkills,
  mockLoadPreviewSkills,
  mockLoadRegistries,
  mockPreviewGitHubRepoImport,
  mockResetGitHubImport,
  mockRescan,
  mockSearchSkillsSh,
};
