import type { Dispatch, SetStateAction } from "react";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import type { PlatformCategoryKey } from "@/lib/platformVisibility";
import type {
  AgentWithStatus,
  CreateSshTargetRequest,
  GitHubPatTestResult,
  SshTargetTestResult,
  TargetSummary,
  TestSshTargetRequest,
  UpdateSshTargetRequest,
} from "@/types";
import type { SshTargetFormState } from "@/components/settings/RemoteTargetsSettingsSection";
import {
  EMPTY_SSH_TARGET_FORM,
  sshTargetPayload,
  targetToSshTargetForm,
} from "@/pages/settingsViewModel";
import { createPlatformManagementActions } from "@/pages/platformManagementActions";

export type StatusMessage = {
  type: "success" | "error";
  text: string;
  detail?: string | null;
} | null;

type StateSetter<T> = Dispatch<SetStateAction<T>>;

export function createSettingsViewActions({
  t,
  githubPatInput,
  sshTargetForm,
  sshTargetEditForm,
  sshTargetPasswordUpdates,
  editingPlatform,
  selectedMarketplaceRegistryId,
  addScanDirectory,
  removeScanDirectory,
  toggleScanDirectory,
  addCustomAgent,
  updateCustomAgent,
  removeCustomAgent,
  saveGitHubPat,
  clearGitHubPat,
  testGitHubPat,
  rescan,
  refreshCounts,
  loadCentralSkills,
  refreshDiscoverCounts,
  loadMarketplaceRegistries,
  loadMarketplaceSkills,
  createSshTarget,
  updateSshTarget,
  testSshTarget,
  updateSshTargetPassword,
  deleteTarget,
  switchTarget,
  setCategoryVisibility,
  setAgentEnabled,
  setEditingPlatform,
  setPlatformError,
  setIsPlatformDialogOpen,
  setRemovingAgent,
  setScanDirError,
  setRemovingDir,
  setGitHubPatInput,
  setGitHubPatMessage,
  setTargetMessage,
  setSshTargetForm,
  setEditingTargetId,
  setSshTargetEditForm,
  setSshTargetPasswordUpdates,
}: {
  t: TFunction;
  githubPatInput: string;
  sshTargetForm: SshTargetFormState;
  sshTargetEditForm: SshTargetFormState;
  sshTargetPasswordUpdates: Record<string, string>;
  editingPlatform: AgentWithStatus | null;
  selectedMarketplaceRegistryId: string | null;
  addScanDirectory: (path: string, label?: string) => Promise<unknown>;
  removeScanDirectory: (path: string) => Promise<void>;
  toggleScanDirectory: (path: string, active: boolean) => Promise<void>;
  addCustomAgent: (config: {
    display_name: string;
    global_skills_dir: string;
    category: string;
  }) => Promise<unknown>;
  updateCustomAgent: (
    agentId: string,
    config: {
      display_name: string;
      global_skills_dir: string;
      category: string;
    }
  ) => Promise<unknown>;
  removeCustomAgent: (agentId: string) => Promise<void>;
  saveGitHubPat: (value: string) => Promise<void>;
  clearGitHubPat: () => Promise<void>;
  testGitHubPat: () => Promise<GitHubPatTestResult>;
  rescan: () => Promise<void>;
  refreshCounts: () => Promise<void>;
  loadCentralSkills: () => Promise<void>;
  refreshDiscoverCounts: () => Promise<void>;
  loadMarketplaceRegistries: () => Promise<void>;
  loadMarketplaceSkills: (registryId: string) => Promise<void>;
  createSshTarget: (request: CreateSshTargetRequest) => Promise<TargetSummary>;
  updateSshTarget: (request: UpdateSshTargetRequest) => Promise<TargetSummary>;
  testSshTarget: (request: TestSshTargetRequest) => Promise<SshTargetTestResult>;
  updateSshTargetPassword: (targetId: string, password: string) => Promise<SshTargetTestResult>;
  deleteTarget: (targetId: string) => Promise<void>;
  switchTarget: (targetId: string) => Promise<TargetSummary>;
  setCategoryVisibility: (category: PlatformCategoryKey, visible: boolean) => Promise<void>;
  setAgentEnabled: (agentId: string, enabled: boolean) => Promise<void>;
  setEditingPlatform: StateSetter<AgentWithStatus | null>;
  setPlatformError: StateSetter<string | null>;
  setIsPlatformDialogOpen: StateSetter<boolean>;
  setRemovingAgent: StateSetter<string | null>;
  setScanDirError: StateSetter<string | null>;
  setRemovingDir: StateSetter<string | null>;
  setGitHubPatInput: StateSetter<string>;
  setGitHubPatMessage: StateSetter<StatusMessage>;
  setTargetMessage: StateSetter<StatusMessage>;
  setSshTargetForm: StateSetter<SshTargetFormState>;
  setEditingTargetId: StateSetter<string | null>;
  setSshTargetEditForm: StateSetter<SshTargetFormState>;
  setSshTargetPasswordUpdates: StateSetter<Record<string, string>>;
}) {
  async function refreshAfterTargetChange() {
    await rescan();
    await Promise.allSettled([
      loadCentralSkills(),
      refreshDiscoverCounts(),
      loadMarketplaceRegistries().then(() => {
        if (selectedMarketplaceRegistryId) {
          return loadMarketplaceSkills(selectedMarketplaceRegistryId);
        }

        return undefined;
      }),
    ]);
  }

  function updateSshTargetFormField(
    field: keyof SshTargetFormState,
    value: string
  ) {
    setSshTargetForm((current) => ({ ...current, [field]: value }));
  }

  function updateSshTargetEditFormField(
    field: keyof SshTargetFormState,
    value: string
  ) {
    setSshTargetEditForm((current) => ({ ...current, [field]: value }));
  }

  function updateExistingTargetPassword(targetId: string, value: string) {
    setSshTargetPasswordUpdates((current) => ({ ...current, [targetId]: value }));
  }

  function handleStartEditTarget(target: TargetSummary) {
    setTargetMessage(null);
    setEditingTargetId(target.id);
    setSshTargetEditForm(targetToSshTargetForm(target));
  }

  function handleCancelEditTarget() {
    setEditingTargetId(null);
    setSshTargetEditForm(EMPTY_SSH_TARGET_FORM);
  }

  async function handleCreateSshTarget() {
    setTargetMessage(null);
    try {
      const target = await createSshTarget(sshTargetPayload(sshTargetForm, true));
      setSshTargetForm(EMPTY_SSH_TARGET_FORM);
      setTargetMessage({
        type: "success",
        text: t("targets.created", { label: target.label }),
      });
      toast.success(t("targets.created", { label: target.label }));
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleTestNewSshTarget() {
    setTargetMessage(null);
    try {
      const result = await testSshTarget(sshTargetPayload(sshTargetForm, true));
      const text = result.ok
        ? t("targets.testSucceeded", {
            home: result.remoteHome ?? "",
            os: result.remoteOs ?? "",
          })
        : result.message;
      setTargetMessage({ type: result.ok ? "success" : "error", text });
      if (result.ok) {
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleTestExistingTarget(targetId: string) {
    setTargetMessage(null);
    try {
      const password = sshTargetPasswordUpdates[targetId];
      const result = await testSshTarget({
        id: targetId,
        password: password?.trim() ? password : null,
      });
      const text = result.ok
        ? t("targets.testSucceeded", {
            home: result.remoteHome ?? "",
            os: result.remoteOs ?? "",
          })
        : result.message;
      setTargetMessage({ type: result.ok ? "success" : "error", text });
      if (result.ok) {
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleUpdateSshTarget(target: TargetSummary) {
    setTargetMessage(null);
    try {
      const updatedTarget = await updateSshTarget({
        id: target.id,
        ...sshTargetPayload(sshTargetEditForm, false),
      });
      setEditingTargetId(null);
      setSshTargetEditForm(EMPTY_SSH_TARGET_FORM);
      updateExistingTargetPassword(target.id, "");
      await refreshAfterTargetChange();
      const text = t("targets.updated", { label: updatedTarget.label });
      setTargetMessage({ type: "success", text });
      toast.success(text);
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleUpdateTargetPassword(target: TargetSummary) {
    const password = sshTargetPasswordUpdates[target.id] ?? "";
    if (!password.trim()) {
      const text = t("targets.passwordRequired");
      setTargetMessage({ type: "error", text });
      toast.error(text);
      return;
    }

    setTargetMessage(null);
    try {
      const result = await updateSshTargetPassword(target.id, password);
      const text = result.ok
        ? result.credentialStatus === "session"
          ? t("targets.passwordUpdatedSession", { label: target.label })
          : t("targets.passwordUpdated", { label: target.label })
        : result.message;
      setTargetMessage({ type: result.ok ? "success" : "error", text });
      if (result.ok) {
        updateExistingTargetPassword(target.id, "");
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleSwitchTarget(targetId: string) {
    setTargetMessage(null);
    try {
      const target = await switchTarget(targetId);
      await refreshAfterTargetChange();
      toast.success(t("targets.switched", { label: target.label }));
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleDeleteTarget(targetId: string) {
    setTargetMessage(null);
    try {
      await deleteTarget(targetId);
      await refreshAfterTargetChange();
      toast.success(t("targets.deleted"));
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleAddDirectory(path: string) {
    setScanDirError(null);
    try {
      await addScanDirectory(path);
      await refreshCounts();
      toast.success(t("addDir.add") + " ✓");
    } catch (err) {
      setScanDirError(String(err));
      toast.error(String(err));
      throw err;
    }
  }

  async function handleRemoveDirectory(path: string) {
    setRemovingDir(path);
    setScanDirError(null);
    try {
      await removeScanDirectory(path);
      await refreshCounts();
      toast.success(t("common.delete") + " ✓");
    } catch (err) {
      setScanDirError(String(err));
      toast.error(String(err));
    } finally {
      setRemovingDir(null);
    }
  }

  async function handleToggleDirectory(path: string, active: boolean) {
    setScanDirError(null);
    try {
      await toggleScanDirectory(path, active);
    } catch (err) {
      setScanDirError(String(err));
      toast.error(String(err));
    }
  }

  const {
    handleOpenAddPlatform,
    handleOpenEditPlatform,
    handleAddPlatform,
    handleEditPlatform,
    handleRemovePlatform,
    handleToggleCategory,
    handleTogglePlatformVisibility,
  } = createPlatformManagementActions({
    t,
    editingPlatform,
    addCustomAgent,
    updateCustomAgent,
    removeCustomAgent,
    refreshAfterPlatformChange: rescan,
    setCategoryVisibility,
    setAgentEnabled,
    setEditingPlatform,
    setPlatformError,
    setIsPlatformDialogOpen,
    setRemovingAgent,
  });

  async function handleSaveGitHubPat() {
    setGitHubPatMessage(null);
    try {
      await saveGitHubPat(githubPatInput);
      setGitHubPatMessage({
        type: "success",
        text: t("settings.githubPatSaved"),
      });
      toast.success(t("settings.githubPatSaved"));
    } catch {
      const text = t("settings.githubPatSaveFailed");
      setGitHubPatMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleClearGitHubPat() {
    setGitHubPatMessage(null);
    try {
      await clearGitHubPat();
      setGitHubPatInput("");
      setGitHubPatMessage({
        type: "success",
        text: t("settings.githubPatCleared"),
      });
      toast.success(t("settings.githubPatCleared"));
    } catch {
      const text = t("settings.githubPatClearFailed");
      setGitHubPatMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleTestGitHubPat() {
    setGitHubPatMessage(null);
    try {
      const result = await testGitHubPat();
      const text = t(result.messageKey, {
        defaultValue: t(
          result.ok ? "settings.githubPatTestSuccess" : "settings.githubPatTestFailure"
        ),
        status: result.status ?? "",
      });
      setGitHubPatMessage({
        type: result.ok ? "success" : "error",
        text,
        detail: result.ok ? null : `HTTP ${result.status ?? "-"}`,
      });
      if (result.ok) {
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch {
      const text = t("settings.githubPatTestFailed");
      setGitHubPatMessage({ type: "error", text });
      toast.error(text);
    }
  }

  return {
    updateSshTargetFormField,
    updateSshTargetEditFormField,
    updateExistingTargetPassword,
    handleStartEditTarget,
    handleCancelEditTarget,
    handleCreateSshTarget,
    handleTestNewSshTarget,
    handleTestExistingTarget,
    handleUpdateSshTarget,
    handleUpdateTargetPassword,
    handleSwitchTarget,
    handleDeleteTarget,
    handleAddDirectory,
    handleRemoveDirectory,
    handleToggleDirectory,
    handleOpenAddPlatform,
    handleOpenEditPlatform,
    handleAddPlatform,
    handleEditPlatform,
    handleRemovePlatform,
    handleToggleCategory,
    handleTogglePlatformVisibility,
    handleSaveGitHubPat,
    handleClearGitHubPat,
    handleTestGitHubPat,
  };
}
