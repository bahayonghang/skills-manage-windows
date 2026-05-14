import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import { Blocks, Folder, Search, X } from "lucide-react";
import { toast } from "sonner";

import { InstallDialog } from "@/components/central/InstallDialog";
import { SkillDetailDrawer } from "@/components/skill/SkillDetailDrawer";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { Input } from "@/components/ui/input";
import { VirtualizedList } from "@/components/ui/virtualized-list";
import { invoke } from "@/lib/tauri";
import { getPathBasename } from "@/lib/path";
import { getPlatformTargetGroups } from "@/lib/platformTargetGroups";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";
import { buildSearchText, normalizeSearchQuery } from "@/lib/search";
import { cn } from "@/lib/utils";
import {
  importObsidianSkillToCentral,
  importObsidianSkillToPlatform,
  useObsidianStore,
} from "@/stores/obsidianStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";
import type { BatchInstallResult, ObsidianSkill, SkillWithLinks } from "@/types";

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 py-20">
      <div className="rounded-full bg-muted/60 p-4">
        <Blocks className="size-12 text-muted-foreground opacity-60" />
      </div>
      <p className="text-sm font-medium text-muted-foreground">{message}</p>
    </div>
  );
}

export function ObsidianVaultView() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const { vaultId } = useParams<{ vaultId: string }>();
  const decodedVaultId = vaultId ? decodeURIComponent(vaultId) : "";
  const contentRef = useRef<HTMLDivElement | null>(null);
  const detailButtonRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  const vaults = useObsidianStore((state) => state.vaults);
  const skillsByVault = useObsidianStore((state) => state.skillsByVault);
  const isLoadingVaults = useObsidianStore((state) => state.isLoadingVaults);
  const loadingSkillsByVault = useObsidianStore((state) => state.loadingSkillsByVault);
  const loadVaults = useObsidianStore((state) => state.loadVaults);
  const getVaultSkills = useObsidianStore((state) => state.getVaultSkills);

  const agents = usePlatformStore((state) => state.agents);
  const categoryVisibility =
    usePlatformStore((state) => state.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const refreshCounts = usePlatformStore((state) => state.refreshCounts);
  const activeTarget = useTargetStore((state) => state.activeTarget);
  const isRemoteTarget = activeTarget.kind === "ssh";

  const [searchQuery, setSearchQuery] = useState("");
  const [importingIds, setImportingIds] = useState<Set<string>>(new Set());
  const [installTargetSkill, setInstallTargetSkill] = useState<ObsidianSkill | null>(null);
  const [isInstallDialogOpen, setIsInstallDialogOpen] = useState(false);
  const [drawerSkillId, setDrawerSkillId] = useState<string | null>(null);
  const [drawerInstallTargetSkill, setDrawerInstallTargetSkill] = useState<ObsidianSkill | null>(null);
  const [drawerFilePath, setDrawerFilePath] = useState<string | null>(null);
  const [drawerSourceMeta, setDrawerSourceMeta] = useState<{
    name: string;
    description?: string;
    platformName: string;
    projectName: string;
    filePath: string;
    dirPath: string;
    isAlreadyCentral: boolean;
  } | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);

  useEffect(() => {
    void loadVaults();
  }, [loadVaults]);

  useEffect(() => {
    if (decodedVaultId) {
      void getVaultSkills(decodedVaultId);
    }
  }, [decodedVaultId, getVaultSkills]);

  useEffect(() => {
    if (!decodedVaultId && vaults.length > 0) {
      navigate(`/obsidian/${encodeURIComponent(vaults[0].id)}`, { replace: true });
    }
  }, [decodedVaultId, navigate, vaults]);

  const selectedVault = useMemo(
    () => vaults.find((vault) => vault.id === decodedVaultId) ?? null,
    [decodedVaultId, vaults]
  );
  const selectedSkills = useMemo(
    () => (decodedVaultId ? (skillsByVault[decodedVaultId] ?? []) : []),
    [decodedVaultId, skillsByVault]
  );
  const isLoadingSkills = decodedVaultId ? (loadingSkillsByVault[decodedVaultId] ?? false) : false;
  const normalizedSearchQuery = useMemo(
    () => normalizeSearchQuery(searchQuery),
    [searchQuery]
  );
  const filteredSkills = useMemo(() => {
    if (!normalizedSearchQuery) {
      return selectedSkills;
    }
    return selectedSkills.filter((skill) =>
      buildSearchText([skill.name, skill.description]).includes(normalizedSearchQuery)
    );
  }, [normalizedSearchQuery, selectedSkills]);
  const platformAgents = useMemo(
    () => getPlatformTargetGroups(agents, categoryVisibility),
    [agents, categoryVisibility]
  );

  function setDetailButtonRef(skillId: string, node: HTMLButtonElement | null) {
    detailButtonRefs.current[skillId] = node;
  }

  function openDrawerForSkill(skill: ObsidianSkill) {
    if (skill.is_already_central) {
      setDrawerSkillId(getPathBasename(skill.dir_path) ?? skill.id);
      setDrawerInstallTargetSkill(skill);
      setDrawerFilePath(null);
      setDrawerSourceMeta(null);
    } else {
      setDrawerSkillId(null);
      setDrawerInstallTargetSkill(null);
      setDrawerFilePath(skill.file_path);
      setDrawerSourceMeta({
        name: skill.name,
        description: skill.description,
        platformName: skill.platform_name,
        projectName: skill.project_name,
        filePath: skill.file_path,
        dirPath: skill.dir_path,
        isAlreadyCentral: skill.is_already_central,
      });
    }
    setIsDrawerOpen(true);
  }

  async function handleInstallToCentral(skillId: string) {
    const skill = selectedSkills.find((item) => item.id === skillId);
    if (!skill) {
      toast.error(t("obsidian.importError", { error: `Skill '${skillId}' not found` }));
      return;
    }
    setImportingIds((current) => new Set(current).add(skillId));
    try {
      await importObsidianSkillToCentral(skill);
      await refreshCounts();
      toast.success(t("obsidian.importSuccess"));
    } catch (err) {
      toast.error(t("obsidian.importError", { error: String(err) }));
    } finally {
      setImportingIds((current) => {
        const next = new Set(current);
        next.delete(skillId);
        return next;
      });
    }
  }

  async function handleInstallFromDialog(
    _skillId: string,
    agentIds: string[],
    method: "symlink" | "copy"
  ): Promise<BatchInstallResult> {
    if (!installTargetSkill) {
      return { succeeded: [], failed: [] };
    }

    const succeeded: string[] = [];
    const failed: Array<{ agent_id: string; error: string }> = [];

    setImportingIds((current) => new Set(current).add(installTargetSkill.id));
    try {
      for (const agentId of agentIds) {
        try {
          await importObsidianSkillToPlatform(installTargetSkill, agentId, method);
          succeeded.push(agentId);
        } catch (err) {
          failed.push({ agent_id: agentId, error: String(err) });
        }
      }
      await refreshCounts();
      if (failed.length === 0) {
        toast.success(t("obsidian.importSuccess"));
      }
      return { succeeded, failed };
    } finally {
      setImportingIds((current) => {
        const next = new Set(current);
        next.delete(installTargetSkill.id);
        return next;
      });
    }
  }

  async function handleOpenVaultPath(path: string) {
    if (!path) {
      return;
    }
    try {
      if (isRemoteTarget) {
        await navigator.clipboard.writeText(path);
        toast.success(t("targets.pathCopied"));
        return;
      }
      await invoke("open_in_file_manager", { path });
    } catch (err) {
      toast.error(t("obsidian.openPathError", { error: String(err) }));
    }
  }

  const restorationState = location.state?.scrollRestoration as
    | { scrollTop?: number }
    | undefined;

  useEffect(() => {
    if (typeof restorationState?.scrollTop !== "number") {
      return;
    }
    const frame = requestAnimationFrame(() => {
      if (contentRef.current) {
        contentRef.current.scrollTop = restorationState.scrollTop ?? 0;
      }
    });
    return () => cancelAnimationFrame(frame);
  }, [decodedVaultId, restorationState?.scrollTop]);

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border px-6 py-4">
        <h1 className="text-xl font-semibold">{t("obsidian.title")}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{t("obsidian.desc")}</p>
      </div>

      <div className="flex min-h-0 flex-1">
        <div className="w-72 shrink-0 border-r border-border">
          <div className="border-b border-border px-4 py-3">
            <div className="relative">
              <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={searchQuery}
                onChange={(event) => setSearchQuery(event.target.value)}
                placeholder={t("obsidian.searchPlaceholder")}
                aria-label={t("obsidian.searchPlaceholder")}
                className="pl-9 pr-8"
              />
              {searchQuery ? (
                <button
                  type="button"
                  onClick={() => setSearchQuery("")}
                  aria-label={t("obsidian.clearSearch")}
                  title={t("obsidian.clearSearch")}
                  className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                >
                  <X className="size-3.5" />
                </button>
              ) : null}
            </div>
          </div>
          <div className="overflow-auto p-3">
            {isLoadingVaults ? (
              <div className="flex items-center gap-2 px-2 py-3 text-sm text-muted-foreground">
                <Blocks className="size-4 animate-pulse" />
                {t("obsidian.loadingVaults")}
              </div>
            ) : vaults.length === 0 ? (
              <div className="rounded-lg border border-dashed border-border px-4 py-6 text-sm text-muted-foreground">
                {t("obsidian.empty")}
              </div>
            ) : (
              <div className="space-y-2">
                {vaults.map((vault) => {
                  const isActive = vault.id === decodedVaultId;
                  return (
                    <button
                      key={vault.id}
                      type="button"
                      onClick={() => navigate(`/obsidian/${encodeURIComponent(vault.id)}`)}
                      className={cn(
                        "flex w-full items-center gap-2 rounded-md border px-3 py-2 text-left transition-colors",
                        isActive
                          ? "border-primary/60 bg-primary/10 text-foreground shadow-sm"
                          : "border-border hover:bg-muted/40"
                      )}
                    >
                      <Folder className={cn("size-4 shrink-0", isActive ? "text-primary" : "text-muted-foreground")} />
                      <div className="min-w-0 flex-1">
                        <div className="truncate text-sm font-medium">{vault.name}</div>
                        <div className="truncate text-xs text-muted-foreground">{vault.skill_count}</div>
                      </div>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col">
          {selectedVault ? (
            <>
              <div className="flex items-center gap-3 border-b border-border px-6 py-3">
                <div className="min-w-0 flex-1">
                  <h2 className="truncate text-sm font-semibold">{selectedVault.name}</h2>
                  <button
                    type="button"
                    onClick={() => void handleOpenVaultPath(selectedVault.path)}
                    className="block max-w-full truncate text-left text-xs text-muted-foreground hover:text-primary hover:underline"
                    title={t("obsidian.openInFileManager")}
                  >
                    {selectedVault.path}
                  </button>
                </div>
                <span className="shrink-0 text-xs text-muted-foreground">
                  {t("collection.skills", { count: filteredSkills.length })}
                </span>
              </div>
              <div ref={contentRef} className="flex-1 overflow-auto p-4">
                {isLoadingSkills ? (
                  <EmptyState message={t("obsidian.loadingSkills")} />
                ) : filteredSkills.length === 0 ? (
                  <EmptyState
                    message={
                      normalizedSearchQuery
                        ? t("obsidian.noMatch", { query: searchQuery })
                        : t("obsidian.emptySkills")
                    }
                  />
                ) : filteredSkills.length > 80 ? (
                  <VirtualizedList
                    items={filteredSkills}
                    itemHeight={120}
                    itemGap={8}
                    overscan={6}
                    scrollContainerRef={contentRef}
                    itemKey={(skill) => skill.id}
                    renderItem={(skill) => (
                      <UnifiedSkillCard
                        key={skill.id}
                        name={skill.name}
                        description={skill.description}
                        isCentral={skill.is_already_central}
                        platformBadge={{ id: skill.platform_id, name: skill.platform_name }}
                        projectBadge={skill.project_name}
                        onDetail={() => openDrawerForSkill(skill)}
                        detailButtonRef={(node) =>
                          setDetailButtonRef(
                            skill.is_already_central ? (getPathBasename(skill.dir_path) ?? skill.id) : skill.id,
                            node
                          )
                        }
                        onInstallToCentral={() => void handleInstallToCentral(skill.id)}
                        onInstallToPlatform={() => {
                          setInstallTargetSkill(skill);
                          setIsInstallDialogOpen(true);
                        }}
                        isLoading={importingIds.has(skill.id)}
                        className="h-[120px]"
                      />
                    )}
                  />
                ) : (
                  <div className="space-y-2">
                    {filteredSkills.map((skill) => (
                      <UnifiedSkillCard
                        key={skill.id}
                        name={skill.name}
                        description={skill.description}
                        isCentral={skill.is_already_central}
                        platformBadge={{ id: skill.platform_id, name: skill.platform_name }}
                        projectBadge={skill.project_name}
                        onDetail={() => openDrawerForSkill(skill)}
                        detailButtonRef={(node) =>
                          setDetailButtonRef(
                            skill.is_already_central ? (getPathBasename(skill.dir_path) ?? skill.id) : skill.id,
                            node
                          )
                        }
                        onInstallToCentral={() => void handleInstallToCentral(skill.id)}
                        onInstallToPlatform={() => {
                          setInstallTargetSkill(skill);
                          setIsInstallDialogOpen(true);
                        }}
                        isLoading={importingIds.has(skill.id)}
                      />
                    ))}
                  </div>
                )}
              </div>
            </>
          ) : (
            <EmptyState message={t("obsidian.empty")} />
          )}
        </div>
      </div>

      {installTargetSkill ? (
        <InstallDialog
          open={isInstallDialogOpen}
          onOpenChange={(open) => {
            setIsInstallDialogOpen(open);
            if (!open) {
              setInstallTargetSkill(null);
            }
          }}
          skill={{
            id: installTargetSkill.id,
            name: installTargetSkill.name,
            description: installTargetSkill.description,
            file_path: installTargetSkill.file_path,
            is_central: false,
            linked_agents: [],
            shared_root_agents: [],
            scanned_at: new Date().toISOString(),
          } as SkillWithLinks}
          agents={platformAgents}
          onInstall={handleInstallFromDialog}
        />
      ) : null}

      <SkillDetailDrawer
        open={isDrawerOpen}
        skillId={drawerSkillId}
        filePath={drawerFilePath}
        sourceMetadata={drawerSourceMeta}
        onOpenChange={(open) => {
          setIsDrawerOpen(open);
          if (!open) {
            setDrawerSkillId(null);
            setDrawerInstallTargetSkill(null);
            setDrawerFilePath(null);
            setDrawerSourceMeta(null);
          }
        }}
        returnFocusRef={
          drawerSkillId || drawerFilePath
            ? {
                current: detailButtonRefs.current[drawerSkillId ?? drawerFilePath ?? ""] ?? null,
            }
            : undefined
        }
        onInstallClick={
          drawerInstallTargetSkill
            ? () => {
                setInstallTargetSkill(drawerInstallTargetSkill);
                setIsInstallDialogOpen(true);
              }
            : undefined
        }
      />
    </div>
  );
}
