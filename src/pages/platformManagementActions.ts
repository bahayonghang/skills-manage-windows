import { toast } from "sonner";
import type { TFunction } from "i18next";

import type { PlatformCategoryKey } from "@/lib/platformVisibility";
import type { AgentWithStatus } from "@/types";

export function createPlatformManagementActions({
  t,
  editingPlatform,
  addCustomAgent,
  updateCustomAgent,
  removeCustomAgent,
  refreshAfterPlatformChange,
  setCategoryVisibility,
  setAgentEnabled,
  setEditingPlatform,
  setPlatformError,
  setIsPlatformDialogOpen,
  setRemovingAgent,
}: {
  t: TFunction;
  editingPlatform: AgentWithStatus | null;
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
  refreshAfterPlatformChange: () => Promise<void>;
  setCategoryVisibility: (category: PlatformCategoryKey, visible: boolean) => Promise<void>;
  setAgentEnabled: (agentId: string, enabled: boolean) => Promise<void>;
  setEditingPlatform: (platform: AgentWithStatus | null) => void;
  setPlatformError: (error: string | null) => void;
  setIsPlatformDialogOpen: (open: boolean) => void;
  setRemovingAgent: (agentId: string | null) => void;
}) {
  function handleOpenAddPlatform() {
    setEditingPlatform(null);
    setPlatformError(null);
    setIsPlatformDialogOpen(true);
  }

  function handleOpenEditPlatform(agent: AgentWithStatus) {
    setEditingPlatform(agent);
    setPlatformError(null);
    setIsPlatformDialogOpen(true);
  }

  async function handleAddPlatform(
    displayName: string,
    globalSkillsDir: string,
    category?: string
  ) {
    setPlatformError(null);
    try {
      await addCustomAgent({
        display_name: displayName,
        global_skills_dir: globalSkillsDir,
        category: category || "coding",
      });
      await refreshAfterPlatformChange();
      toast.success(t("platformDialog.add") + " ✓");
    } catch (err) {
      setPlatformError(String(err));
      toast.error(String(err));
      throw err;
    }
  }

  async function handleEditPlatform(
    displayName: string,
    globalSkillsDir: string,
    category?: string
  ) {
    if (!editingPlatform) return;
    setPlatformError(null);
    try {
      await updateCustomAgent(editingPlatform.id, {
        display_name: displayName,
        global_skills_dir: globalSkillsDir,
        category: category || "coding",
      });
      await refreshAfterPlatformChange();
      toast.success(t("platformDialog.save") + " ✓");
    } catch (err) {
      setPlatformError(String(err));
      toast.error(String(err));
      throw err;
    }
  }

  async function handleRemovePlatform(agentId: string) {
    setRemovingAgent(agentId);
    setPlatformError(null);
    try {
      await removeCustomAgent(agentId);
      await refreshAfterPlatformChange();
      toast.success(t("common.delete") + " ✓");
    } catch (err) {
      setPlatformError(String(err));
      toast.error(String(err));
    } finally {
      setRemovingAgent(null);
    }
  }

  async function handleToggleCategory(category: PlatformCategoryKey, visible: boolean) {
    try {
      await setCategoryVisibility(category, visible);
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function handleTogglePlatformVisibility(agentId: string, enabled: boolean) {
    try {
      await setAgentEnabled(agentId, enabled);
    } catch (err) {
      toast.error(String(err));
    }
  }

  return {
    handleOpenAddPlatform,
    handleOpenEditPlatform,
    handleAddPlatform,
    handleEditPlatform,
    handleRemovePlatform,
    handleToggleCategory,
    handleTogglePlatformVisibility,
  };
}
