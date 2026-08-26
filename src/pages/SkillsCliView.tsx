import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { Terminal } from "lucide-react";

import { SkillsCliGroupHeader } from "@/components/skillsCli/SkillsCliGroupHeader";
import { SkillsCliHeader } from "@/components/skillsCli/SkillsCliHeader";
import {
  SKILLS_CLI_INSTALL_SURFACE_AVAILABLE,
  SkillsCliInstallMount,
} from "@/components/skillsCli/SkillsCliInstallMount";
import { SkillsCliToolbar } from "@/components/skillsCli/SkillsCliToolbar";
import { showSkillsCliActionToast } from "@/components/skillsCli/skillsCliActionToast";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { formatBackendError } from "@/lib/backendError";
import { isLocalTarget } from "@/lib/targetKind";
import { cn } from "@/lib/utils";
import {
  SKILLS_CLI_CONTENT_CONTAINER_CLASS,
  SKILLS_CLI_GRID_CLASS,
  SKILLS_CLI_SKELETON_COUNT,
  bucketSkillsCli,
  closeSkillsCliSurface,
  deriveSkillsCliCounts,
  deriveSkillsCliLayoutBands,
  enabledTargetIdSet,
  filterSkillsCli,
  openSkillsCliDetail,
  openSkillsCliInstall,
  openSkillsCliUninstall,
  type SkillsCliActiveSurface,
  type SkillsCliGroupBy,
} from "@/pages/skillsCliViewModel";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { useTargetStore } from "@/stores/targetStore";

function preselectedSkillFromSource(source: string, skills: string[]): string[] {
  const atIndex = source.lastIndexOf("@");
  if (atIndex <= 0) return [...skills];
  const hinted = source.slice(atIndex + 1).trim();
  if (hinted && skills.includes(hinted)) {
    return [hinted];
  }
  return [...skills];
}

function bucketLabel(
  t: (key: string, options?: Record<string, unknown>) => string,
  labelKey: string,
  labelValue?: string,
): string {
  return labelValue ? t(labelKey, { name: labelValue }) : t(labelKey);
}

export function SkillsCliView() {
  const { t } = useTranslation();
  const activeTarget = useTargetStore((state) => state.activeTarget);
  const isLocal = isLocalTarget(activeTarget);

  const skills = useSkillsCliStore((state) => state.skills);
  const targets = useSkillsCliStore((state) => state.targets);
  const preview = useSkillsCliStore((state) => state.preview);
  const doctor = useSkillsCliStore((state) => state.doctor);
  const canonicalRoot = useSkillsCliStore((state) => state.canonicalRoot);
  const lockPath = useSkillsCliStore((state) => state.lockPath);
  const isLoading = useSkillsCliStore((state) => state.isLoading);
  const isRefreshing = useSkillsCliStore((state) => state.isRefreshing);
  const isPreviewing = useSkillsCliStore((state) => state.isPreviewing);
  const isMutating = useSkillsCliStore((state) => state.isMutating);
  const jobId = useSkillsCliStore((state) => state.jobId);
  const runtimeError = useSkillsCliStore((state) => state.runtimeError);
  const inventoryError = useSkillsCliStore((state) => state.inventoryError);
  const actionError = useSkillsCliStore((state) => state.actionError);
  const loadAll = useSkillsCliStore((state) => state.loadAll);
  const previewSource = useSkillsCliStore((state) => state.previewSource);
  const addGlobal = useSkillsCliStore((state) => state.addGlobal);
  const removeGlobal = useSkillsCliStore((state) => state.removeGlobal);
  const cancelJob = useSkillsCliStore((state) => state.cancelJob);

  const [source, setSource] = useState("");
  const [selectedSkillNames, setSelectedSkillNames] = useState<string[]>([]);
  const [selectedPlatformIds, setSelectedPlatformIds] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [groupBy, setGroupBy] = useState<SkillsCliGroupBy>("repo");
  const [platformFilter, setPlatformFilter] = useState<string | null>(null);
  const [unlinkedOnly, setUnlinkedOnly] = useState(false);
  const [selectMode, setSelectMode] = useState(false);
  const [collapsedGroupIds, setCollapsedGroupIds] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [selectedCardNames, setSelectedCardNames] = useState<ReadonlySet<string>>(
    () => new Set(),
  );
  const [activeSurface, setActiveSurface] = useState<SkillsCliActiveSurface>(null);
  const [contentWidthPx, setContentWidthPx] = useState<number | null>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const installButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!isLocal) {
      return;
    }
    void loadAll();
  }, [isLocal, loadAll]);

  useEffect(() => {
    setSelectedPlatformIds(
      targets.filter((target) => target.defaultSelected).map((target) => target.id),
    );
  }, [targets]);

  useEffect(() => {
    if (!preview) {
      setSelectedSkillNames([]);
      return;
    }
    setSelectedSkillNames(preselectedSkillFromSource(preview.source, preview.skills));
  }, [preview]);

  useEffect(() => {
    const node = contentRef.current;
    if (!node || typeof ResizeObserver === "undefined") {
      return;
    }
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (typeof width === "number") {
        setContentWidthPx(width);
      }
    });
    observer.observe(node);
    return () => observer.disconnect();
  }, [isLocal]);

  const runtimeBlocked = runtimeError !== null;
  const installOpen = !inventoryError && skills.length === 0 && !isLoading;
  const showInventoryEmpty = skills.length === 0 && !isLoading && !inventoryError;
  const showLoading = isLoading && skills.length === 0 && !inventoryError;
  const enabledIds = useMemo(() => enabledTargetIdSet(targets), [targets]);
  const counts = useMemo(
    () => deriveSkillsCliCounts(skills, enabledIds),
    [skills, enabledIds],
  );
  const filtered = useMemo(
    () =>
      filterSkillsCli(
        skills,
        { query, platformFilter, unlinkedOnly },
        enabledIds,
      ),
    [skills, query, platformFilter, unlinkedOnly, enabledIds],
  );
  const buckets = useMemo(
    () => bucketSkillsCli(filtered, groupBy, targets, enabledIds),
    [filtered, groupBy, targets, enabledIds],
  );
  const showFilteredEmpty =
    skills.length > 0 && filtered.length === 0 && !showLoading;
  const layoutBands = deriveSkillsCliLayoutBands(contentWidthPx);
  const uninstallNames =
    activeSurface?.kind === "uninstall" ? activeSurface.skillNames : [];
  const uninstallTarget =
    uninstallNames.length === 1
      ? skills.find((skill) => skill.name === uninstallNames[0]) ?? null
      : null;

  if (!isLocal) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-8">
        <Terminal className="size-10 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">{t("skillsCli.localOnly")}</p>
      </div>
    );
  }

  function handlePageKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key !== "Escape") {
      return;
    }
    if (activeSurface !== null || event.defaultPrevented) {
      return;
    }
    setSelectMode(false);
    setSelectedCardNames(new Set());
  }

  async function handlePreview() {
    const result = await previewSource(source.trim());
    if (!result) {
      const latest = useSkillsCliStore.getState().actionError;
      if (latest) {
        showSkillsCliActionToast({
          semantic: "error",
          message: t("skillsCli.previewError", {
            error: formatBackendError(latest, t),
          }),
        });
      }
    }
  }

  async function handleAdd() {
    if (selectedSkillNames.length === 0 || selectedPlatformIds.length === 0) {
      return;
    }
    try {
      const result = await addGlobal({
        source: preview?.source ?? source.trim(),
        skillNames: selectedSkillNames,
        skillportAgentIds: selectedPlatformIds,
      });
      showSkillsCliActionToast({
        semantic: "success",
        message: t("skillsCli.addSuccess", {
          count: result.installedSkills,
          platforms: result.targetedPlatforms,
        }),
      });
      setSource("");
      await loadAll();
      const refreshError = useSkillsCliStore.getState().inventoryError;
      if (refreshError) {
        showSkillsCliActionToast({
          semantic: "error",
          message: t("skillsCli.inventoryRefreshWarning", {
            error: formatBackendError(refreshError, t),
          }),
        });
      }
    } catch (err) {
      showSkillsCliActionToast({
        semantic: "error",
        message: t("skillsCli.addError", { error: formatBackendError(err, t) }),
      });
    }
  }

  async function handleUninstall() {
    const name = uninstallNames[0];
    if (!name) return;
    const ok = await removeGlobal(name);
    if (ok) {
      setActiveSurface(closeSkillsCliSurface());
      showSkillsCliActionToast({
        semantic: "destructiveSuccess",
        message: t("skillsCli.removeSuccess", { name }),
      });
      return;
    }
    const latest = useSkillsCliStore.getState().actionError;
    if (latest) {
      showSkillsCliActionToast({
        semantic: "destructiveError",
        message: t("skillsCli.removeError", {
          error: formatBackendError(latest, t),
        }),
      });
    }
  }

  function toggleSkill(name: string) {
    setSelectedSkillNames((current) =>
      current.includes(name)
        ? current.filter((item) => item !== name)
        : [...current, name],
    );
  }

  function togglePlatform(id: string) {
    setSelectedPlatformIds((current) =>
      current.includes(id)
        ? current.filter((item) => item !== id)
        : [...current, id],
    );
  }

  function toggleCollapsed(id: string) {
    setCollapsedGroupIds((current) => {
      const next = new Set(current);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  function toggleCardSelected(name: string) {
    setSelectedCardNames((current) => {
      const next = new Set(current);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  }

  const canAdd =
    selectedSkillNames.length > 0 &&
    selectedPlatformIds.length > 0 &&
    !runtimeBlocked &&
    !isMutating &&
    !isPreviewing;

  return (
    <div
      className="flex h-full min-w-0 flex-col overflow-hidden overflow-x-hidden"
      data-testid="skills-cli-page"
      onKeyDown={handlePageKeyDown}
    >
      <div
        hidden
        data-testid="skills-cli-active-surface"
        data-kind={activeSurface?.kind ?? "none"}
        data-focus={
          activeSurface?.kind === "detail" ? String(activeSurface.focus) : ""
        }
        data-uninstall={uninstallNames.join(",")}
      />
      <div
        hidden
        data-testid="skills-cli-layout-bands"
        data-grid={layoutBands.grid}
        data-drawer={layoutBands.drawer}
        data-width={contentWidthPx == null ? "" : String(contentWidthPx)}
      />
      <SkillsCliHeader
        counts={counts}
        doctor={doctor}
        runtimeError={runtimeError}
        isLoading={isLoading}
        isRefreshing={isRefreshing}
        installAvailable={SKILLS_CLI_INSTALL_SURFACE_AVAILABLE}
        onRefresh={() => void loadAll()}
        onOpenInstall={() => setActiveSurface(openSkillsCliInstall())}
        installButtonRef={installButtonRef}
      />

      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto overflow-x-hidden px-6 py-4">
        <div
          ref={contentRef}
          data-testid="skills-cli-content"
          className={cn(SKILLS_CLI_CONTENT_CONTAINER_CLASS, "space-y-4")}
        >
          <SkillsCliToolbar
            query={query}
            onQueryChange={setQuery}
            groupBy={groupBy}
            onGroupByChange={setGroupBy}
            platformFilter={platformFilter}
            onPlatformFilterChange={setPlatformFilter}
            unlinkedOnly={unlinkedOnly}
            onUnlinkedOnlyChange={setUnlinkedOnly}
            selectMode={selectMode}
            onSelectModeChange={setSelectMode}
            targets={targets}
          />

          {inventoryError && (
            <div
              role="alert"
              data-testid="skills-cli-inventory-error"
              className="flex flex-wrap items-center gap-2 text-sm text-destructive-text"
            >
              <span>{formatBackendError(inventoryError, t)}</span>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => void loadAll()}
              >
                {t("common.retry")}
              </Button>
            </div>
          )}

          <section data-testid="skills-cli-inventory">
            {showLoading ? (
              <div
                className={SKILLS_CLI_GRID_CLASS}
                aria-busy="true"
                data-testid="skills-cli-skeleton"
              >
                {Array.from({ length: SKILLS_CLI_SKELETON_COUNT }, (_, index) => (
                  <div
                    key={index}
                    aria-hidden
                    className="min-h-[76px] animate-pulse rounded-xl bg-muted/60"
                  />
                ))}
              </div>
            ) : showInventoryEmpty ? (
              <p className="text-sm text-muted-foreground">{t("skillsCli.empty")}</p>
            ) : showFilteredEmpty ? (
              <p className="text-sm text-muted-foreground">
                {t("skillsCli.filteredEmpty", { query })}
              </p>
            ) : (
              <div className="space-y-4">
                {buckets.map((bucket) => {
                  const panelId = `skills-cli-group-panel-${bucket.id}`;
                  const expanded = !collapsedGroupIds.has(bucket.id);
                  return (
                    <section key={bucket.id}>
                      <SkillsCliGroupHeader
                        bucket={bucket}
                        label={bucketLabel(t, bucket.labelKey, bucket.labelValue)}
                        expanded={expanded}
                        panelId={panelId}
                        onToggle={() => toggleCollapsed(bucket.id)}
                      />
                      {expanded ? (
                        <div id={panelId} className={SKILLS_CLI_GRID_CLASS}>
                          {bucket.skills.map((skill) => (
                            <UnifiedSkillCard
                              key={skill.name}
                              variant="skillsCli"
                              layout="denseRow"
                              name={skill.name}
                              path={skill.canonicalPath ?? skill.path}
                              placements={skill.placements}
                              checkbox={
                                selectMode
                                  ? {
                                      checked: selectedCardNames.has(skill.name),
                                      onChange: () => toggleCardSelected(skill.name),
                                    }
                                  : undefined
                              }
                              onDetail={() =>
                                setActiveSurface(openSkillsCliDetail(skill.name, null))
                              }
                              onManageLinks={() =>
                                setActiveSurface(
                                  openSkillsCliDetail(skill.name, "links"),
                                )
                              }
                              onUninstall={() =>
                                setActiveSurface(openSkillsCliUninstall([skill.name]))
                              }
                              isLoading={isMutating || runtimeBlocked}
                            />
                          ))}
                        </div>
                      ) : null}
                    </section>
                  );
                })}
              </div>
            )}
          </section>

          <details
            data-testid="skills-cli-install"
            className="rounded-lg border border-border p-4"
            open={installOpen}
          >
            <summary className="cursor-pointer text-sm font-medium">
              {t("skillsCli.installHeading")}
            </summary>
            <div className="mt-3 space-y-3">
              {actionError && (
                <p role="alert" className="text-sm text-destructive-text">
                  {formatBackendError(actionError, t)}
                </p>
              )}
              {runtimeBlocked && (
                <p className="text-sm text-muted-foreground">
                  {t("skillsCli.runtimeBlocked")}
                </p>
              )}
              <label className="text-sm font-medium" htmlFor="skills-cli-source">
                {t("skillsCli.sourceLabel")}
              </label>
              <div className="flex flex-wrap gap-2">
                <Input
                  id="skills-cli-source"
                  value={source}
                  onChange={(event) => setSource(event.target.value)}
                  placeholder={t("skillsCli.sourcePlaceholder")}
                  disabled={isPreviewing || isMutating}
                />
                <Button
                  onClick={() => void handlePreview()}
                  disabled={!source.trim() || isPreviewing || isMutating}
                >
                  {isPreviewing ? t("skillsCli.previewing") : t("skillsCli.preview")}
                </Button>
              </div>

              {preview && (
                <div className="grid gap-4 md:grid-cols-2">
                  <fieldset>
                    <legend className="mb-2 text-sm font-medium">
                      {t("skillsCli.skillsHeading")}
                    </legend>
                    <div className="space-y-1.5">
                      {preview.skills.map((name) => (
                        <label
                          key={name}
                          className="flex items-center gap-2 text-sm"
                        >
                          <input
                            type="checkbox"
                            checked={selectedSkillNames.includes(name)}
                            onChange={() => toggleSkill(name)}
                            aria-label={t("skillsCli.selectSkill", { name })}
                          />
                          {name}
                        </label>
                      ))}
                    </div>
                  </fieldset>
                  <fieldset>
                    <legend className="mb-2 text-sm font-medium">
                      {t("skillsCli.platformsHeading")}
                    </legend>
                    <div className="space-y-1.5">
                      {targets.map((target) => (
                        <label
                          key={target.id}
                          className="flex items-center gap-2 text-sm"
                        >
                          <input
                            type="checkbox"
                            checked={selectedPlatformIds.includes(target.id)}
                            onChange={() => togglePlatform(target.id)}
                            aria-label={t("skillsCli.selectPlatform", {
                              name: target.displayName,
                            })}
                          />
                          {target.displayName}
                        </label>
                      ))}
                    </div>
                  </fieldset>
                </div>
              )}

              <div className="flex gap-2">
                <Button onClick={() => void handleAdd()} disabled={!canAdd}>
                  {isMutating ? t("skillsCli.adding") : t("skillsCli.add")}
                </Button>
                {jobId && (
                  <Button variant="outline" onClick={() => void cancelJob()}>
                    {t("skillsCli.cancel")}
                  </Button>
                )}
              </div>
            </div>
          </details>

          {canonicalRoot && lockPath && (
            <div
              data-testid="skills-cli-paths"
              className="space-y-0.5 text-ui-meta text-muted-foreground"
            >
              <p>{t("skillsCli.pathsCanonical", { root: canonicalRoot })}</p>
              <p>{t("skillsCli.pathsLock", { lock: lockPath })}</p>
              <p>{t("skillsCli.ownershipNote")}</p>
            </div>
          )}
        </div>
      </div>

      <SkillsCliInstallMount
        open={activeSurface?.kind === "install"}
        onOpenChange={(open) => {
          setActiveSurface(
            open ? openSkillsCliInstall() : closeSkillsCliSurface(),
          );
        }}
        returnFocusRef={installButtonRef}
        contentWidthPx={contentWidthPx}
      />

      <Dialog
        open={activeSurface?.kind === "uninstall"}
        onOpenChange={(open) => {
          if (open && runtimeBlocked) {
            return;
          }
          if (!open) setActiveSurface(closeSkillsCliSurface());
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("skillsCli.uninstallTitle", {
                name: uninstallTarget?.name ?? uninstallNames[0] ?? "",
              })}
            </DialogTitle>
            <DialogDescription>
              {t("skillsCli.uninstallDescription")}
            </DialogDescription>
          </DialogHeader>
          {uninstallTarget && (
            <div className="space-y-1 text-sm text-muted-foreground">
              {uninstallTarget.path && (
                <p>{t("skillsCli.path", { path: uninstallTarget.path })}</p>
              )}
              {uninstallTarget.agents.length > 0 && (
                <p>
                  {t("skillsCli.agents", {
                    agents: uninstallTarget.agents.join(", "),
                  })}
                </p>
              )}
              {uninstallTarget.source && (
                <p>{t("skillsCli.source", { source: uninstallTarget.source })}</p>
              )}
            </div>
          )}
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setActiveSurface(closeSkillsCliSurface())}
            >
              {t("common.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => void handleUninstall()}
              disabled={isMutating || runtimeBlocked}
            >
              {isMutating
                ? t("skillsCli.uninstalling")
                : t("skillsCli.uninstallConfirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
