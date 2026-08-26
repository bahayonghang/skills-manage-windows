import { useEffect, type ReactNode, type RefObject } from "react";
import { useTranslation } from "react-i18next";

import { SkillsCliInstallDialog } from "@/components/skillsCli/SkillsCliInstallDialog";
import { showSkillsCliActionToast } from "@/components/skillsCli/skillsCliActionToast";
import { formatBackendError } from "@/lib/backendError";
import {
  buildPlatformInstalledCounts,
  joinInstallPlatformTargets,
} from "@/pages/skillsCliInstallViewModel";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillsCliRecentSourcesStore } from "@/stores/skillsCliRecentSourcesStore";
import { useSkillsCliStore } from "@/stores/skillsCliStore";

export function SkillsCliInstallSurface({
  open,
  onOpenChange,
  returnFocusRef,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  returnFocusRef: RefObject<HTMLElement | null>;
  contentWidthPx: number | null;
}): ReactNode {
  const { t } = useTranslation();
  const skills = useSkillsCliStore((state) => state.skills);
  const targets = useSkillsCliStore((state) => state.targets);
  const doctor = useSkillsCliStore((state) => state.doctor);
  const canonicalRoot = useSkillsCliStore((state) => state.canonicalRoot);
  const isPreviewing = useSkillsCliStore((state) => state.isPreviewing);
  const isMutating = useSkillsCliStore((state) => state.isMutating);
  const previewSource = useSkillsCliStore((state) => state.previewSource);
  const addGlobal = useSkillsCliStore((state) => state.addGlobal);
  const loadAll = useSkillsCliStore((state) => state.loadAll);
  const agents = usePlatformStore((state) => state.agents);
  const recentSources = useSkillsCliRecentSourcesStore((state) => state.sources);
  const isRecentSourcesLoading = useSkillsCliRecentSourcesStore(
    (state) => state.isLoading,
  );
  const recentError = useSkillsCliRecentSourcesStore((state) => state.error);
  const loadRecent = useSkillsCliRecentSourcesStore((state) => state.load);
  const pushRecent = useSkillsCliRecentSourcesStore((state) => state.push);

  useEffect(() => {
    if (open) {
      void loadRecent();
    }
  }, [open, loadRecent]);

  const installedNames = new Set(skills.map((skill) => skill.name));
  const platformInstalledCounts = buildPlatformInstalledCounts(skills, targets);
  const platformTargets = joinInstallPlatformTargets(targets, agents);

  return (
    <SkillsCliInstallDialog
      open={open}
      onOpenChange={onOpenChange}
      canonicalRoot={canonicalRoot}
      npmSpec={doctor?.npmSpec ?? ""}
      targets={targets}
      platformTargets={platformTargets}
      installedNames={installedNames}
      platformInstalledCounts={platformInstalledCounts}
      recentSources={recentSources}
      isRecentSourcesLoading={isRecentSourcesLoading}
      recentSourcesLoadFailed={recentError !== null}
      isPreviewing={isPreviewing}
      isMutating={isMutating}
      returnFocusRef={returnFocusRef}
      onPreview={async (source) => {
        const result = await previewSource(source);
        if (result) {
          return result;
        }
        const latest = useSkillsCliStore.getState().actionError;
        throw (
          latest ??
          new Error(
            "skills_cli.preview_unparsed:The skill preview could not be parsed.",
          )
        );
      }}
      onInstall={async (input) => {
        const result = await addGlobal(input);
        onOpenChange(false);
        showSkillsCliActionToast({
          semantic: "success",
          message: t("skillsCli.addSuccess", {
            count: result.installedSkills,
            platforms: result.targetedPlatforms,
          }),
        });
        void (async () => {
          try {
            await loadAll();
            const inventoryError = useSkillsCliStore.getState().inventoryError;
            if (inventoryError) {
              showSkillsCliActionToast({
                semantic: "error",
                message: t("skillsCli.inventoryRefreshWarning", {
                  error: formatBackendError(inventoryError, t),
                }),
              });
            }
          } catch (error) {
            showSkillsCliActionToast({
              semantic: "error",
              message: t("skillsCli.inventoryRefreshWarning", {
                error: formatBackendError(error, t),
              }),
            });
          }
        })();
        void (async () => {
          try {
            await pushRecent(input.source);
          } catch (error) {
            showSkillsCliActionToast({
              semantic: "error",
              message: t("skillsCli.install.recentPushWarning", {
                error: formatBackendError(error, t),
              }),
            });
          }
        })();
        return result;
      }}
    />
  );
}
