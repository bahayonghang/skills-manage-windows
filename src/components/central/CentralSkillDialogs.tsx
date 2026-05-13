import { lazy, Suspense, type ComponentProps, type RefObject } from "react";
import type { TFunction } from "i18next";

import { BatchDeleteCentralSkillsDialog } from "@/components/central/BatchDeleteCentralSkillsDialog";
import { BatchInstallCentralSkillsDialog } from "@/components/central/BatchInstallCentralSkillsDialog";
import { DeleteCentralSkillDialog } from "@/components/central/DeleteCentralSkillDialog";
import { InstallDialog, type InstallMethod } from "@/components/central/InstallDialog";
import { RemoteMissingSkillsDialog } from "@/components/central/RemoteMissingSkillsDialog";
import { CentralUpdateConfirmDialog } from "@/components/central/CentralUpdateConfirmDialog";
import { SkillDetailDrawer } from "@/components/skill/SkillDetailDrawer";
import type { GitHubRepoImportWizardProps } from "@/components/marketplace/githubImportWizardUtils";
import type { PlatformTarget } from "@/lib/platformTargetGroups";
import type {
  AgentWithStatus,
  BatchInstallResult,
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  BatchDeleteCentralSkillResult,
  CentralBatchInstallResult,
  CentralSkillUpdateState,
  DeleteSkillRepositoryPreview,
  GitHubRepoPreview,
  GitHubRepoImportResult,
  SkillDetail,
  SkillRepositoryWithStats,
  SkillWithLinks,
  SkillportStateImportPreview,
  SkillportStateImportResolution,
  SkillportStateImportResult,
  SkillportStatePortabilityJob,
} from "@/types";

type GitHubImportState = {
  isPreviewLoading: boolean;
  isImporting: boolean;
  preview: GitHubRepoPreview | null;
  importResult: GitHubRepoImportResult | null;
  error: string | null;
};

const CentralStatePortabilityDialog = lazy(async () => {
  const module = await import("@/components/central/CentralStatePortabilityDialog");
  return { default: module.CentralStatePortabilityDialog };
});

const GitHubRepoImportWizard = lazy(async () => {
  const module = await import("@/components/marketplace/GitHubRepoImportWizard");
  return { default: module.GitHubRepoImportWizard };
});

const CentralPlatformManageDrawer = lazy(async () => {
  const module = await import("@/components/central/CentralPlatformManageDrawer");
  return { default: module.CentralPlatformManageDrawer };
});

type PlatformManagementProps = Omit<
  ComponentProps<typeof import("@/components/central/CentralPlatformManageDrawer").CentralPlatformManageDrawer>,
  "open" | "onOpenChange"
>;

export function CentralSkillDialogs({
  agents,
  availableInstallAgents,
  batchDeletePreview,
  batchDeletePreviewError,
  deletePreview,
  deletePreviewError,
  deleteTargetSkill,
  detailButtonRefs,
  drawerSkillId,
  githubImport,
  githubRepoUrl,
  importSkillportState,
  installTargetSkill,
  installableImportedSkills,
  isBatchDeleteDialogOpen,
  isBatchDeletePreviewLoading,
  isBatchInstallDialogOpen,
  isDeleteDialogOpen,
  isDeletePreviewLoading,
  isDeleting,
  isDialogOpen,
  isDrawerOpen,
  isGitHubImportOpen,
  isPlatformManageOpen,
  isInstalling,
  isPortabilityOpen,
  isRemoteMissingDialogOpen,
  isRemoteMissingPreviewLoading,
  isRepositoryDeleteDialogOpen,
  isRepositoryDeletePreviewLoading,
  isResolvingRemoteMissing,
  isUpdatingSkills,
  isUpdateConfirmDialogOpen,
  loadCentralSkills,
  pendingUpdateStates,
  previewSkillportStateImport,
  remoteMissingError,
  remoteMissingPreview,
  remoteMissingStates,
  repositoryDeletePreview,
  repositoryDeletePreviewError,
  repositoryDeleteTarget,
  selectedSkillIds,
  skills,
  setDrawerSkillId,
  setGithubRepoUrl,
  setIsBatchInstallDialogOpen,
  setIsDialogOpen,
  setIsDrawerOpen,
  setIsGitHubImportOpen,
  setIsPlatformManageOpen,
  setIsPortabilityOpen,
  t,
  exportSkillportState,
  portabilityJob,
  cancelSkillportStatePortability,
  platformManagement,
  onAfterImportSuccess,
  onBatchDeleteCentralSkills,
  onBatchDeleteDialogOpenChange,
  onBatchInstallCentralSkills,
  onDeleteCentralSkill,
  onDeleteDialogOpenChange,
  onDeleteSkillRepository,
  onGitHubImport,
  onGitHubPreview,
  onInstall,
  onInstallImportedSkill,
  onRefreshCounts,
  onConfirmUpdateSkills,
  onRemoteMissingDialogOpenChange,
  onUpdateConfirmDialogOpenChange,
  onRepositoryDeleteDialogOpenChange,
  onResetGitHubImport,
  onResolveRemoteMissing,
}: {
  agents: AgentWithStatus[];
  availableInstallAgents: PlatformTarget[];
  batchDeletePreview: BatchDeleteCentralSkillPreviewResult | null;
  batchDeletePreviewError: string | null;
  deletePreview: SkillDetail | null;
  deletePreviewError: string | null;
  deleteTargetSkill: SkillWithLinks | null;
  detailButtonRefs: RefObject<Record<string, HTMLButtonElement | null>>;
  drawerSkillId: string | null;
  githubImport: GitHubImportState;
  githubRepoUrl: string;
  importSkillportState: (
    json: string,
    resolutions: SkillportStateImportResolution[]
  ) => Promise<SkillportStateImportResult>;
  installTargetSkill: SkillWithLinks | null;
  installableImportedSkills: SkillWithLinks[];
  isBatchDeleteDialogOpen: boolean;
  isBatchDeletePreviewLoading: boolean;
  isBatchInstallDialogOpen: boolean;
  isDeleteDialogOpen: boolean;
  isDeletePreviewLoading: boolean;
  isDeleting: boolean;
  isDialogOpen: boolean;
  isDrawerOpen: boolean;
  isGitHubImportOpen: boolean;
  isPlatformManageOpen: boolean;
  isInstalling: boolean;
  isPortabilityOpen: boolean;
  isRemoteMissingDialogOpen: boolean;
  isRemoteMissingPreviewLoading: boolean;
  isRepositoryDeleteDialogOpen: boolean;
  isRepositoryDeletePreviewLoading: boolean;
  isResolvingRemoteMissing: boolean;
  isUpdatingSkills: boolean;
  isUpdateConfirmDialogOpen: boolean;
  loadCentralSkills: () => Promise<void>;
  pendingUpdateStates: CentralSkillUpdateState[];
  previewSkillportStateImport: (json: string) => Promise<SkillportStateImportPreview>;
  remoteMissingError: string | null;
  remoteMissingPreview: BatchDeleteCentralSkillPreviewResult | null;
  remoteMissingStates: CentralSkillUpdateState[];
  repositoryDeletePreview: DeleteSkillRepositoryPreview | null;
  repositoryDeletePreviewError: string | null;
  repositoryDeleteTarget: SkillRepositoryWithStats | null;
  selectedSkillIds: string[];
  skills: SkillWithLinks[];
  setDrawerSkillId: (skillId: string | null) => void;
  setGithubRepoUrl: (url: string) => void;
  setIsBatchInstallDialogOpen: (open: boolean) => void;
  setIsDialogOpen: (open: boolean) => void;
  setIsDrawerOpen: (open: boolean) => void;
  setIsGitHubImportOpen: (open: boolean) => void;
  setIsPlatformManageOpen: (open: boolean) => void;
  setIsPortabilityOpen: (open: boolean) => void;
  t: TFunction;
  exportSkillportState: () => Promise<string>;
  portabilityJob: SkillportStatePortabilityJob;
  cancelSkillportStatePortability: () => Promise<void>;
  platformManagement: PlatformManagementProps;
  onAfterImportSuccess: () => Promise<void>;
  onBatchDeleteCentralSkills: (
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<BatchDeleteCentralSkillResult>;
  onBatchDeleteDialogOpenChange: (open: boolean) => void;
  onBatchInstallCentralSkills: (
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null
  ) => Promise<CentralBatchInstallResult>;
  onDeleteCentralSkill: (skillId: string, removeAgentIds: string[]) => Promise<void>;
  onDeleteDialogOpenChange: (open: boolean) => void;
  onDeleteSkillRepository: (
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<BatchDeleteCentralSkillResult>;
  onGitHubImport: GitHubRepoImportWizardProps["onImport"];
  onGitHubPreview: () => Promise<GitHubRepoPreview | null>;
  onInstall: (
    skillId: string,
    agentIds: string[],
    method: InstallMethod,
    projectPath?: string | null
  ) => Promise<BatchInstallResult>;
  onInstallImportedSkill: (
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null
  ) => Promise<BatchInstallResult>;
  onRefreshCounts: () => Promise<void>;
  onConfirmUpdateSkills: (skillIds: string[]) => Promise<void>;
  onRemoteMissingDialogOpenChange: (open: boolean) => void;
  onUpdateConfirmDialogOpenChange: (open: boolean) => void;
  onRepositoryDeleteDialogOpenChange: (open: boolean) => void;
  onResetGitHubImport: () => void;
  onResolveRemoteMissing: (
    keepSkillIds: string[],
    deleteRequests: BatchDeleteCentralSkillRequest[]
  ) => Promise<void>;
}) {
  return (
    <>
      <InstallDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
        skill={installTargetSkill}
        agents={availableInstallAgents}
        onInstall={onInstall}
      />

      <BatchInstallCentralSkillsDialog
        open={isBatchInstallDialogOpen}
        onOpenChange={setIsBatchInstallDialogOpen}
        skillCount={selectedSkillIds.length}
        agents={availableInstallAgents}
        isInstalling={isInstalling}
        onInstall={onBatchInstallCentralSkills}
      />

      <DeleteCentralSkillDialog
        open={isDeleteDialogOpen}
        onOpenChange={onDeleteDialogOpenChange}
        skill={deleteTargetSkill}
        detail={deletePreview}
        agents={agents}
        isPreviewLoading={isDeletePreviewLoading}
        isDeleting={isDeleting}
        error={deletePreviewError}
        onConfirm={onDeleteCentralSkill}
      />

      <BatchDeleteCentralSkillsDialog
        open={isBatchDeleteDialogOpen}
        onOpenChange={onBatchDeleteDialogOpenChange}
        skillIds={selectedSkillIds}
        preview={batchDeletePreview}
        agents={agents}
        isPreviewLoading={isBatchDeletePreviewLoading}
        isDeleting={isDeleting}
        error={batchDeletePreviewError}
        onConfirm={onBatchDeleteCentralSkills}
      />

      <RemoteMissingSkillsDialog
        open={isRemoteMissingDialogOpen}
        onOpenChange={onRemoteMissingDialogOpenChange}
        states={remoteMissingStates}
        preview={remoteMissingPreview}
        agents={agents}
        isPreviewLoading={isRemoteMissingPreviewLoading}
        isApplying={isResolvingRemoteMissing || isDeleting}
        error={remoteMissingError}
        onConfirm={onResolveRemoteMissing}
      />

      <CentralUpdateConfirmDialog
        open={isUpdateConfirmDialogOpen}
        onOpenChange={onUpdateConfirmDialogOpenChange}
        states={pendingUpdateStates}
        skills={skills}
        isApplying={isUpdatingSkills}
        onConfirm={onConfirmUpdateSkills}
      />

      <BatchDeleteCentralSkillsDialog
        open={isRepositoryDeleteDialogOpen}
        onOpenChange={onRepositoryDeleteDialogOpenChange}
        skillIds={(repositoryDeletePreview?.delete_preview.previews ?? []).map((item) => item.skill_id)}
        preview={repositoryDeletePreview?.delete_preview ?? null}
        agents={agents}
        isPreviewLoading={isRepositoryDeletePreviewLoading}
        isDeleting={isDeleting}
        error={repositoryDeletePreviewError}
        title={t("central.deleteRepositoryTitle", {
          name: repositoryDeleteTarget?.name ?? "",
        })}
        description={t("central.deleteRepositoryDesc", {
          name: repositoryDeleteTarget?.name ?? "",
          count: repositoryDeletePreview?.delete_preview.previews.length ?? 0,
        })}
        dangerTitle={t("central.deleteRepositoryCentralRequired", {
          count: repositoryDeletePreview?.delete_preview.previews.length ?? 0,
        })}
        confirmLabel={t("central.confirmDeleteRepository")}
        confirmTestId="confirm-delete-skill-repository"
        onConfirm={onDeleteSkillRepository}
      />

      <SkillDetailDrawer
        open={isDrawerOpen}
        skillId={drawerSkillId}
        onOpenChange={(open) => {
          setIsDrawerOpen(open);
          if (!open) {
            setDrawerSkillId(null);
          }
        }}
        returnFocusRef={
          drawerSkillId
            ? {
                current: detailButtonRefs.current[drawerSkillId] ?? null,
              }
            : undefined
        }
      />

      <Suspense fallback={null}>
        <CentralPlatformManageDrawer
          open={isPlatformManageOpen}
          onOpenChange={setIsPlatformManageOpen}
          {...platformManagement}
        />
      </Suspense>

      {isGitHubImportOpen && (
        <Suspense
          fallback={
            <div
              role="status"
              className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 text-sm text-muted-foreground"
            >
              <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
                {t("central.loading")}
              </div>
            </div>
          }
        >
          <GitHubRepoImportWizard
            open={isGitHubImportOpen}
            onOpenChange={setIsGitHubImportOpen}
            repoUrl={githubRepoUrl}
            onRepoUrlChange={setGithubRepoUrl}
            preview={githubImport.preview}
            previewError={githubImport.error}
            isPreviewLoading={githubImport.isPreviewLoading}
            isImporting={githubImport.isImporting}
            importResult={githubImport.importResult}
            onPreview={onGitHubPreview}
            onImport={onGitHubImport}
            availableAgents={availableInstallAgents}
            installableSkills={installableImportedSkills}
            onInstallImportedSkill={onInstallImportedSkill}
            onAfterImportSuccess={onAfterImportSuccess}
            onReset={() => {
              onResetGitHubImport();
              setGithubRepoUrl("");
            }}
            launcherLabel={t("central.title")}
          />
        </Suspense>
      )}

      {isPortabilityOpen && (
        <Suspense
          fallback={
            <div
              role="status"
              className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 text-sm text-muted-foreground"
            >
              <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
                {t("central.loading")}
              </div>
            </div>
          }
        >
          <CentralStatePortabilityDialog
            open={isPortabilityOpen}
            onOpenChange={setIsPortabilityOpen}
            exportState={exportSkillportState}
            previewImport={previewSkillportStateImport}
            importState={importSkillportState}
            portabilityJob={portabilityJob}
            onCancelJob={cancelSkillportStatePortability}
            onAfterImport={async () => {
              await Promise.all([onRefreshCounts(), loadCentralSkills()]);
            }}
          />
        </Suspense>
      )}
    </>
  );
}
