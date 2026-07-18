import { useEffect, useMemo, useRef, useState } from "react";
import { useParams, useNavigate, useLocation } from "react-router-dom";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import {
  Pencil,
  Trash2,
  Download,
  PackagePlus,
  Plus,
  Loader2,
  BookOpen,
  FileInput,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { useCollectionStore } from "@/stores/collectionStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { CollectionEditor } from "@/components/collection/CollectionEditor";
import { SkillPickerDialog } from "@/components/collection/SkillPickerDialog";
import { CollectionInstallDialog } from "@/components/collection/CollectionInstallDialog";
import { InstallDialog } from "@/components/central/InstallDialog";
import { SkillWithLinks } from "@/types";
import { useSkillExplanationSummaries } from "@/hooks/useSkillExplanationSummaries";
import {
  consumeScrollPosition,
  createScrollRestorationState,
} from "@/lib/scrollRestoration";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";
import { getPlatformTargetGroups } from "@/lib/platformTargetGroups";

// Scroll-restoration key shared with `CollectionsListView` so list-level and
// single-collection pages interoperate under the same restoration contract.
function collectionScrollKey(collectionId: string): string {
  return `collection:${collectionId}`;
}

// ─── CollectionView ───────────────────────────────────────────────────────────

export function CollectionView() {
  const { collectionId } = useParams<{ collectionId: string }>();
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation();

  const currentDetail = useCollectionStore((s) => s.currentDetail);
  const isLoadingDetail = useCollectionStore((s) => s.isLoadingDetail);
  const error = useCollectionStore((s) => s.error);
  const loadCollectionDetail = useCollectionStore(
    (s) => s.loadCollectionDetail,
  );
  const removeSkillFromCollection = useCollectionStore(
    (s) => s.removeSkillFromCollection,
  );
  const deleteCollection = useCollectionStore((s) => s.deleteCollection);
  const batchInstallCollection = useCollectionStore(
    (s) => s.batchInstallCollection,
  );
  const exportCollection = useCollectionStore((s) => s.exportCollection);
  const addSkillToCollection = useCollectionStore(
    (s) => s.addSkillToCollection,
  );

  const agents = usePlatformStore((s) => s.agents);
  const categoryVisibility =
    usePlatformStore((s) => s.categoryVisibility) ??
    DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const refreshCounts = usePlatformStore((s) => s.refreshCounts);

  const centralSkills = useCentralSkillsStore((s) => s.skills);
  const loadCentralSkills = useCentralSkillsStore((s) => s.loadCentralSkills);
  const installCentralSkill = useCentralSkillsStore((s) => s.installSkill);
  const visibleCollectionAgents = useMemo(
    () => getPlatformTargetGroups(agents, categoryVisibility),
    [agents, categoryVisibility],
  );
  const visibleCentralAgents = useMemo(
    () => getPlatformTargetGroups(agents, categoryVisibility),
    [agents, categoryVisibility],
  );

  const importCollection = useCollectionStore((s) => s.importCollection);

  // Dialog open states.
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [isNewEditorOpen, setIsNewEditorOpen] = useState(false);
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [isInstallOpen, setIsInstallOpen] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [installTargetSkill, setInstallTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [isSingleInstallOpen, setIsSingleInstallOpen] = useState(false);
  const importInputRef = useRef<HTMLInputElement>(null);
  const skillsContainerRef = useRef<HTMLDivElement | null>(null);

  // Restoration state carried through navigation when returning from a skill
  // detail. The context is already present in the URL (collectionId), but the
  // scroll offset needs to be re-applied after data hydrates.
  //
  // React Router preserves `location.state` across `navigate(-1)` only when
  // the previous history entry was pushed *with* state. Entering
  // /collection/:id from the sidebar or list view has no state, so on
  // back-navigation we also rely on the in-memory scroll map, synthesising a
  // restoration entry keyed on the current collectionId.
  const locationRestorationState = location.state?.scrollRestoration as
    { key?: string; scrollTop?: number } | undefined;
  const restorationState: { key?: string; scrollTop?: number } | undefined =
    locationRestorationState ??
    (collectionId ? { key: collectionScrollKey(collectionId) } : undefined);

  async function handleImportFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const collection = await importCollection(text);
      navigate(`/collection/${collection.id}`);
    } catch (err) {
      toast.error(String(err));
    } finally {
      if (importInputRef.current) importInputRef.current.value = "";
    }
  }

  // Load collection detail on mount and when collectionId changes.
  useEffect(() => {
    if (collectionId) {
      loadCollectionDetail(collectionId);
    }
  }, [collectionId, loadCollectionDetail]);

  // Ensure central skills are loaded so we can resolve SkillWithLinks for InstallDialog.
  useEffect(() => {
    if (centralSkills.length === 0) {
      loadCentralSkills();
    }
  }, [centralSkills.length, loadCentralSkills]);
  const collectionSkillIds = useMemo(
    () => currentDetail?.skills.map((skill) => skill.id) ?? [],
    [currentDetail?.skills],
  );
  const aiSummaries = useSkillExplanationSummaries(collectionSkillIds, "zh");

  // Scroll restoration: once the collection detail for this route's
  // collectionId has finished hydrating, restore the previously recorded
  // skill-list scroll offset. We prefer the in-memory map populated by
  // SkillDetail's back handler, and fall back to the `scrollTop` packed into
  // `location.state` for tests/hosts that don't preserve state through back
  // navigation. After a successful restore we clear the navigation state so
  // that later interactions can't re-apply the stale offset.
  useEffect(() => {
    if (!collectionId) return;
    if (!currentDetail || currentDetail.id !== collectionId) return;
    if (!restorationState?.key) return;
    if (restorationState.key !== collectionScrollKey(collectionId)) return;
    const container = skillsContainerRef.current;
    if (!container) return;

    let scrollTop = consumeScrollPosition(restorationState.key);
    if (scrollTop === null && typeof restorationState.scrollTop === "number") {
      scrollTop = restorationState.scrollTop;
    }
    if (scrollTop === null) return;

    container.scrollTop = scrollTop;
    navigate(location.pathname, { replace: true, state: null });
  }, [
    collectionId,
    currentDetail,
    restorationState?.key,
    restorationState?.scrollTop,
    navigate,
    location.pathname,
  ]);

  function handleInstallSingleSkillClick(skillId: string) {
    const target = centralSkills.find((s) => s.id === skillId);
    if (!target) {
      toast.error(t("central.installError", { error: t("platform.notFound") }));
      return;
    }
    setInstallTargetSkill(target);
    setIsSingleInstallOpen(true);
  }

  async function handleInstallSingleSkill(
    skillId: string,
    agentIds: string[],
    method: string,
    projectPath?: string | null,
  ) {
    try {
      const result = await installCentralSkill(
        skillId,
        agentIds,
        method,
        projectPath,
      );
      await refreshCounts();
      if (result.failed.length > 0) {
        const failedNames = result.failed
          .map((failure) => `${failure.agent_id}: ${failure.error}`)
          .join("; ");
        toast.error(
          t("central.installPartialFail", { platforms: failedNames }),
        );
      }
      return result;
    } catch (err) {
      toast.error(t("central.installError", { error: String(err) }));
      throw err;
    }
  }

  async function handleRemoveSkill(skillId: string) {
    if (!collectionId) return;
    try {
      await removeSkillFromCollection(collectionId, skillId);
    } catch (err) {
      toast.error(t("collection.removeSkillError", { error: String(err) }));
    }
  }

  async function handleDelete() {
    if (!collectionId || !currentDetail) return;
    if (
      !window.confirm(
        t("collection.deleteConfirm", { name: currentDetail.name }),
      )
    ) {
      return;
    }
    setIsDeleting(true);
    setDeleteError(null);
    try {
      await deleteCollection(collectionId);
      navigate("/central");
    } catch (err) {
      setDeleteError(String(err));
      toast.error(t("collection.deleteError", { error: String(err) }));
    } finally {
      setIsDeleting(false);
    }
  }

  async function handleExport() {
    if (!collectionId || !currentDetail) return;
    try {
      const json = await exportCollection(collectionId);
      // Trigger browser download.
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${currentDetail.name.replace(/\s+/g, "-").toLowerCase()}-collection.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      toast.error(t("collection.exportError", { error: String(err) }));
    }
  }

  async function handleAddSkills(skillIds: string[]) {
    if (!collectionId) return;
    try {
      // Add skills sequentially.
      for (const skillId of skillIds) {
        await addSkillToCollection(collectionId, skillId);
      }
    } catch (err) {
      toast.error(t("collection.addSkillError", { error: String(err) }));
    }
  }

  // ── Loading ──────────────────────────────────────────────────────────────

  if (isLoadingDetail) {
    return (
      <div className="flex items-center justify-center h-full gap-3 text-muted-foreground">
        <Loader2 className="size-5 animate-spin" />
        <span className="text-sm">{t("collection.loading")}</span>
      </div>
    );
  }

  // ── Error ────────────────────────────────────────────────────────────────

  if (error && !currentDetail) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-3 text-muted-foreground">
        <p className="text-sm text-destructive">{error}</p>
        <Button variant="outline" size="sm" onClick={() => navigate(-1)}>
          {t("collection.goBack")}
        </Button>
      </div>
    );
  }

  if (!currentDetail) {
    return null;
  }

  // ── Main View ────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b border-border px-6 py-4">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0 flex-1">
            <h1 className="text-xl font-semibold truncate">
              {currentDetail.name}
            </h1>
            {currentDetail.description && (
              <p className="text-sm text-muted-foreground mt-0.5">
                {currentDetail.description}
              </p>
            )}
          </div>

          {/* Action buttons */}
          <div className="flex items-center gap-2 shrink-0">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setIsNewEditorOpen(true)}
            >
              <Plus className="size-3.5" />
              <span>{t("sidebar.newCollectionLabel")}</span>
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => importInputRef.current?.click()}
            >
              <FileInput className="size-3.5" />
              <span>{t("sidebar.importCollection")}</span>
            </Button>
            <div className="w-px h-5 bg-border" />
            <Button
              variant="outline"
              size="sm"
              onClick={() => setIsEditorOpen(true)}
              aria-label={t("collection.editLabel")}
            >
              <Pencil className="size-3.5" />
              <span>{t("collection.edit")}</span>
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleExport}
              aria-label={t("collection.exportLabel")}
            >
              <Download className="size-3.5" />
              <span>{t("collection.export")}</span>
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleDelete}
              disabled={isDeleting}
              aria-label={t("collection.deleteLabel")}
              className="text-destructive hover:text-destructive hover:bg-destructive/10 border-destructive/30"
            >
              {isDeleting ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : (
                <Trash2 className="size-3.5" />
              )}
              <span>{t("collection.delete")}</span>
            </Button>
          </div>
        </div>

        {deleteError && (
          <p className="text-xs text-destructive mt-2" role="alert">
            {deleteError}
          </p>
        )}
      </div>

      {/* Skills section header */}
      <div className="flex items-center justify-between px-6 py-3 border-b border-border">
        <span className="text-sm font-medium text-muted-foreground">
          {t("collection.skills", { count: currentDetail.skills.length })}
        </span>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setIsInstallOpen(true)}
            disabled={currentDetail.skills.length === 0}
            aria-label={t("collection.batchInstallLabel")}
          >
            <PackagePlus className="size-3.5" />
            <span>{t("collection.batchInstall")}</span>
          </Button>
          <Button
            variant="default"
            size="sm"
            onClick={() => setIsPickerOpen(true)}
            aria-label={t("collection.addSkillLabel")}
          >
            <Plus className="size-3.5" />
            <span>{t("collection.addSkill")}</span>
          </Button>
        </div>
      </div>

      {/* Skills list */}
      <div ref={skillsContainerRef} className="flex-1 overflow-auto">
        {currentDetail.skills.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-4 py-20">
            <div className="p-4 rounded-full bg-muted/60">
              <BookOpen className="size-12 text-muted-foreground opacity-60" />
            </div>
            <div className="text-center space-y-1">
              <p className="text-sm font-medium text-muted-foreground">
                {t("collection.noSkillsTitle")}
              </p>
              <p className="text-xs text-muted-foreground">
                {t("collection.noSkillsDesc")}
              </p>
            </div>
            <Button
              variant="default"
              size="sm"
              onClick={() => setIsPickerOpen(true)}
            >
              <Plus className="size-3.5" />
              {t("collection.addFirstSkill")}
            </Button>
          </div>
        ) : (
          <div className="mx-6 my-3 grid grid-cols-2 gap-4">
            {currentDetail.skills.map((skill) => (
              <UnifiedSkillCard
                variant="collection"
                key={skill.id}
                name={skill.name}
                description={skill.description}
                aiSummary={aiSummaries[skill.id]}
                onDetail={() =>
                  navigate(`/skill/${skill.id}`, {
                    state: {
                      collectionContext: {
                        collectionId: currentDetail.id,
                      },
                      scrollRestoration: createScrollRestorationState(
                        collectionScrollKey(currentDetail.id),
                        skillsContainerRef.current?.scrollTop ?? 0,
                      ),
                    },
                  })
                }
                onInstallTo={() => handleInstallSingleSkillClick(skill.id)}
                onRemove={() => handleRemoveSkill(skill.id)}
              />
            ))}
          </div>
        )}
      </div>

      {/* Dialogs */}
      <CollectionEditor
        open={isEditorOpen}
        onOpenChange={setIsEditorOpen}
        collection={{
          id: currentDetail.id,
          name: currentDetail.name,
          description: currentDetail.description,
          created_at: currentDetail.created_at,
          updated_at: currentDetail.updated_at,
        }}
      />

      <SkillPickerDialog
        open={isPickerOpen}
        onOpenChange={setIsPickerOpen}
        existingSkillIds={currentDetail.skills.map((s) => s.id)}
        onAdd={handleAddSkills}
      />

      <CollectionEditor
        open={isNewEditorOpen}
        onOpenChange={setIsNewEditorOpen}
        collection={null}
      />

      <input
        ref={importInputRef}
        type="file"
        accept=".json"
        className="hidden"
        onChange={handleImportFile}
      />

      <CollectionInstallDialog
        open={isInstallOpen}
        onOpenChange={setIsInstallOpen}
        collectionName={currentDetail.name}
        skillCount={currentDetail.skills.length}
        agents={visibleCollectionAgents}
        onInstall={(agentIds) =>
          batchInstallCollection(currentDetail.id, agentIds)
        }
      />

      <InstallDialog
        open={isSingleInstallOpen}
        onOpenChange={setIsSingleInstallOpen}
        skill={installTargetSkill}
        agents={visibleCentralAgents}
        onInstall={handleInstallSingleSkill}
      />
    </div>
  );
}
