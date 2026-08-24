import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Terminal } from "lucide-react";
import { toast } from "sonner";

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
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { useTargetStore } from "@/stores/targetStore";
import type { SkillsCliGlobalSkill } from "@/types";

function preselectedSkillFromSource(source: string, skills: string[]): string[] {
  const atIndex = source.lastIndexOf("@");
  if (atIndex <= 0) return [...skills];
  const hinted = source.slice(atIndex + 1).trim();
  if (hinted && skills.includes(hinted)) {
    return [hinted];
  }
  return [...skills];
}

export function SkillsCliView() {
  const { t } = useTranslation();
  const activeTarget = useTargetStore((state) => state.activeTarget);
  const isLocal = isLocalTarget(activeTarget);

  const skills = useSkillsCliStore((state) => state.skills);
  const targets = useSkillsCliStore((state) => state.targets);
  const preview = useSkillsCliStore((state) => state.preview);
  const doctor = useSkillsCliStore((state) => state.doctor);
  const isLoading = useSkillsCliStore((state) => state.isLoading);
  const isPreviewing = useSkillsCliStore((state) => state.isPreviewing);
  const isMutating = useSkillsCliStore((state) => state.isMutating);
  const jobId = useSkillsCliStore((state) => state.jobId);
  const error = useSkillsCliStore((state) => state.error);
  const loadAll = useSkillsCliStore((state) => state.loadAll);
  const previewSource = useSkillsCliStore((state) => state.previewSource);
  const addGlobal = useSkillsCliStore((state) => state.addGlobal);
  const removeGlobal = useSkillsCliStore((state) => state.removeGlobal);
  const cancelJob = useSkillsCliStore((state) => state.cancelJob);

  const [source, setSource] = useState("");
  const [selectedSkillNames, setSelectedSkillNames] = useState<string[]>([]);
  const [selectedPlatformIds, setSelectedPlatformIds] = useState<string[]>([]);
  const [uninstallTarget, setUninstallTarget] =
    useState<SkillsCliGlobalSkill | null>(null);

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

  const visibleError = useMemo(
    () => (error ? formatBackendError(error, t) : null),
    [error, t],
  );

  if (!isLocal) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-8">
        <Terminal className="size-10 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">{t("skillsCli.localOnly")}</p>
      </div>
    );
  }

  async function handlePreview() {
    const result = await previewSource(source.trim());
    if (!result && useSkillsCliStore.getState().error) {
      toast.error(
        t("skillsCli.previewError", {
          error: formatBackendError(useSkillsCliStore.getState().error, t),
        }),
      );
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
      if (result) {
        toast.success(
          t("skillsCli.addSuccess", {
            count: result.installedSkills,
            platforms: result.targetedPlatforms,
          }),
        );
        setSource("");
        return;
      }
      const latest = useSkillsCliStore.getState().error;
      if (latest) {
        toast.error(
          t("skillsCli.addError", { error: formatBackendError(latest, t) }),
        );
      }
    } catch (err) {
      toast.error(t("skillsCli.addError", { error: formatBackendError(err, t) }));
    }
  }

  async function handleUninstall() {
    if (!uninstallTarget) return;
    const name = uninstallTarget.name;
    const ok = await removeGlobal(name);
    if (ok) {
      setUninstallTarget(null);
      toast.success(t("skillsCli.removeSuccess", { name }));
      return;
    }
    const latest = useSkillsCliStore.getState().error;
    if (latest) {
      toast.error(
        t("skillsCli.removeError", { error: formatBackendError(latest, t) }),
      );
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

  const canAdd =
    selectedSkillNames.length > 0 &&
    selectedPlatformIds.length > 0 &&
    !isMutating &&
    !isPreviewing;

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="border-b border-border px-6 py-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-semibold">{t("skillsCli.title")}</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              {t("skillsCli.subtitle")}
            </p>
          </div>
          <Button
            variant="outline"
            onClick={() => void loadAll()}
            disabled={isLoading || isMutating}
          >
            {t("skillsCli.refresh")}
          </Button>
        </div>
        <p className="mt-2 text-xs text-muted-foreground" data-testid="skills-cli-doctor">
          {isLoading && !doctor
            ? t("skillsCli.doctorChecking")
            : doctor
              ? t("skillsCli.doctorOk", {
                  version: doctor.nodeVersion,
                  spec: doctor.npmSpec,
                })
              : visibleError}
        </p>
      </div>

      <div className="flex-1 overflow-y-auto px-6 py-4 space-y-6">
        {visibleError && (
          <p role="alert" className="text-sm text-destructive-text">
            {visibleError}
          </p>
        )}

        <section className="space-y-3 rounded-lg border border-border p-4">
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
        </section>

        <section className="space-y-3">
          {skills.length === 0 && !isLoading ? (
            <p className="text-sm text-muted-foreground">{t("skillsCli.empty")}</p>
          ) : (
            <div className="grid gap-3 md:grid-cols-2">
              {skills.map((skill) => (
                <UnifiedSkillCard
                  key={skill.name}
                  variant="skillsCli"
                  name={skill.name}
                  description={
                    skill.path
                      ? t("skillsCli.path", { path: skill.path })
                      : undefined
                  }
                  path={skill.path}
                  agents={skill.agents}
                  source={skill.source}
                  onUninstall={() => setUninstallTarget(skill)}
                  uninstallLabel={t("skillsCli.uninstall", { name: skill.name })}
                  isLoading={isMutating}
                />
              ))}
            </div>
          )}
        </section>
      </div>

      <Dialog
        open={uninstallTarget !== null}
        onOpenChange={(open) => {
          if (!open) setUninstallTarget(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("skillsCli.uninstallTitle", {
                name: uninstallTarget?.name ?? "",
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
            <Button variant="outline" onClick={() => setUninstallTarget(null)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => void handleUninstall()}
              disabled={isMutating}
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
