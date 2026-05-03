import { useMemo, useState } from "react";
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { Search, XIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogOverlay,
  DialogPortal,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { CustomPlatformsSettingsSection } from "@/components/settings/CustomPlatformsSettingsSection";
import { PlatformDialog } from "@/components/settings/PlatformDialog";
import { PlatformVisibilitySettingsSection } from "@/components/settings/PlatformVisibilitySettingsSection";
import { getNormalizedPlatformVisibilityQuery, getPlatformVisibilityGroups } from "@/pages/settingsViewModel";
import { createSettingsViewActions } from "@/pages/settingsViewActions";
import type { PlatformCategoryVisibility } from "@/lib/platformVisibility";
import type { AgentWithStatus } from "@/types";

interface CentralPlatformManageDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  agents: AgentWithStatus[];
  categoryVisibility: PlatformCategoryVisibility;
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
  setCategoryVisibility: (category: "coding" | "lobster", visible: boolean) => Promise<void>;
  setAgentEnabled: (agentId: string, enabled: boolean) => Promise<void>;
  rescan: () => Promise<void>;
  refreshCounts: () => Promise<void>;
  loadCentralSkills: () => Promise<void>;
  refreshDiscoverCounts: () => Promise<void>;
}

async function noopAsync() {
  return undefined;
}

const DEFAULT_PLATFORM_CATEGORIES: PlatformCategoryVisibility = {
  coding: true,
  lobster: true,
};

export function CentralPlatformManageDrawer({
  open,
  onOpenChange,
  agents,
  categoryVisibility,
  addCustomAgent,
  updateCustomAgent,
  removeCustomAgent,
  setCategoryVisibility,
  setAgentEnabled,
  rescan,
  refreshCounts,
  loadCentralSkills,
  refreshDiscoverCounts,
}: CentralPlatformManageDrawerProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [isPlatformDialogOpen, setIsPlatformDialogOpen] = useState(false);
  const [editingPlatform, setEditingPlatform] = useState<AgentWithStatus | null>(null);
  const [platformError, setPlatformError] = useState<string | null>(null);
  const [removingAgent, setRemovingAgent] = useState<string | null>(null);
  const normalizedQuery = useMemo(() => getNormalizedPlatformVisibilityQuery(query), [query]);
  const safeCategoryVisibility = categoryVisibility ?? DEFAULT_PLATFORM_CATEGORIES;
  const groups = useMemo(
    () =>
      getPlatformVisibilityGroups({
        agents,
        categoryVisibility: safeCategoryVisibility,
        normalizedQuery,
        t,
      }),
    [agents, normalizedQuery, safeCategoryVisibility, t]
  );
  const customAgents = useMemo(() => agents.filter((agent) => !agent.is_builtin), [agents]);
  const titleId = "central-platform-manage-title";

  const {
    handleOpenAddPlatform,
    handleOpenEditPlatform,
    handleAddPlatform,
    handleEditPlatform,
    handleRemovePlatform,
    handleToggleCategory,
    handleTogglePlatformVisibility,
  } = createSettingsViewActions({
    t,
    githubPatInput: "",
    sshTargetForm: {
      label: "",
      host: "",
      username: "",
      port: "22",
      authMethod: "key",
      keyPath: "",
      password: "",
    },
    sshTargetEditForm: {
      label: "",
      host: "",
      username: "",
      port: "22",
      authMethod: "key",
      keyPath: "",
      password: "",
    },
    sshTargetPasswordUpdates: {},
    editingPlatform,
    selectedMarketplaceRegistryId: null,
    addScanDirectory: noopAsync,
    removeScanDirectory: async () => undefined,
    toggleScanDirectory: async () => undefined,
    addCustomAgent,
    updateCustomAgent,
    removeCustomAgent,
    saveGitHubPat: async () => undefined,
    clearGitHubPat: async () => undefined,
    testGitHubPat: async () => ({ ok: true, message: "" }),
    rescan,
    refreshCounts,
    loadCentralSkills,
    refreshDiscoverCounts,
    loadMarketplaceRegistries: async () => undefined,
    loadMarketplaceSkills: async () => undefined,
    createSshTarget: async () => {
      throw new Error("unsupported");
    },
    updateSshTarget: async () => {
      throw new Error("unsupported");
    },
    testSshTarget: async () => ({ ok: true, message: "" }),
    updateSshTargetPassword: async () => ({ ok: true, message: "" }),
    deleteTarget: async () => undefined,
    switchTarget: async () => {
      throw new Error("unsupported");
    },
    setCategoryVisibility,
    setAgentEnabled,
    setEditingPlatform,
    setPlatformError,
    setIsPlatformDialogOpen,
    setRemovingAgent,
    setScanDirError: () => undefined,
    setRemovingDir: () => undefined,
    setGitHubPatInput: () => undefined,
    setGitHubPatMessage: () => undefined,
    setTargetMessage: () => undefined,
    setSshTargetForm: () => undefined,
    setEditingTargetId: () => undefined,
    setSshTargetEditForm: () => undefined,
    setSshTargetPasswordUpdates: () => undefined,
  });

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogPortal keepMounted={false}>
          <DialogOverlay className="bg-black/20" />
          <DialogPrimitive.Popup
            role="dialog"
            aria-modal="true"
            aria-labelledby={titleId}
            className="fixed inset-y-0 right-0 z-50 flex h-full w-screen flex-col bg-background shadow-2xl ring-1 ring-border outline-none sm:w-[min(760px,96vw)]"
          >
            <div className="flex min-h-0 flex-1 flex-col">
              <div className="shrink-0 border-b border-border p-4">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0 space-y-1">
                    <DialogTitle id={titleId}>{t("central.platformManageTitle")}</DialogTitle>
                    <p className="text-sm text-muted-foreground">{t("central.platformManageDesc")}</p>
                  </div>
                  <DialogClose
                    render={
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-sm"
                        aria-label={t("common.close")}
                      />
                    }
                  >
                    <XIcon />
                  </DialogClose>
                </div>
                <div className="relative mt-4">
                  <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    placeholder={t("settings.platformSearchPlaceholder")}
                    className="h-9 bg-muted/40 pl-8"
                  />
                </div>
              </div>

              <div className="min-h-0 flex-1 overflow-y-auto p-4">
                <div className="space-y-4">
                  <PlatformVisibilitySettingsSection
                    groups={groups}
                    isSearchActive={normalizedQuery.length > 0}
                    normalizedQuery={normalizedQuery}
                    query={query}
                    onQueryChange={setQuery}
                    onToggleCategory={(category, visible) => {
                      void handleToggleCategory(category, visible);
                    }}
                    onTogglePlatform={(agentId, enabled) => {
                      void handleTogglePlatformVisibility(agentId, enabled);
                    }}
                  />
                  <CustomPlatformsSettingsSection
                    customAgents={customAgents}
                    platformError={platformError}
                    removingAgent={removingAgent}
                    onAddPlatform={handleOpenAddPlatform}
                    onEditPlatform={handleOpenEditPlatform}
                    onRemovePlatform={(agentId) => {
                      void handleRemovePlatform(agentId);
                    }}
                  />
                </div>
              </div>
            </div>
          </DialogPrimitive.Popup>
        </DialogPortal>
      </Dialog>

      <PlatformDialog
        open={isPlatformDialogOpen}
        onOpenChange={setIsPlatformDialogOpen}
        platform={editingPlatform}
        onAdd={handleAddPlatform}
        onEdit={handleEditPlatform}
      />
    </>
  );
}
