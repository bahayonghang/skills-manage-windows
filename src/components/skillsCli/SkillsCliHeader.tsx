import type { Ref } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import { formatBackendError } from "@/lib/backendError";
import { cn } from "@/lib/utils";
import type { SkillsCliCounts } from "@/pages/skillsCliViewModel";
import type { SkillsCliDoctorReport } from "@/types";

export interface SkillsCliHeaderProps {
  counts: SkillsCliCounts;
  doctor: SkillsCliDoctorReport | null;
  runtimeError: string | null;
  isLoading: boolean;
  isRefreshing: boolean;
  isCheckingUpdates?: boolean;
  installAvailable: boolean;
  mutationLockReason?: string;
  onRefresh: () => void;
  onCheckUpdates: () => void;
  onCancelUpdate?: () => void;
  onOpenInstall: () => void;
  installButtonRef?: Ref<HTMLButtonElement>;
}

export function SkillsCliHeader({
  counts,
  doctor,
  runtimeError,
  isLoading,
  isRefreshing,
  isCheckingUpdates = false,
  installAvailable,
  mutationLockReason,
  onRefresh,
  onCheckUpdates,
  onCancelUpdate,
  onOpenInstall,
  installButtonRef,
}: SkillsCliHeaderProps) {
  const { t } = useTranslation();
  const runtimeLabel = isLoading && !doctor && !runtimeError
    ? t("skillsCli.doctorChecking")
    : doctor
      ? t("skillsCli.doctorOk", {
          version: doctor.nodeVersion,
          spec: doctor.npmSpec,
        })
      : runtimeError
        ? formatBackendError(runtimeError, t)
        : t("skillsCli.doctorUnknown");
  const installDisabled =
    !installAvailable || runtimeError !== null || Boolean(mutationLockReason);
  const updatesDisabled = Boolean(mutationLockReason);

  return (
    <header className="border-b border-border px-6 py-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-xl font-semibold">{t("skillsCli.title")}</h1>
          <p className="mt-1 text-sm text-muted-foreground">{t("skillsCli.subtitle")}</p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={onRefresh}
            disabled={isLoading || isRefreshing}
            aria-label={t("skillsCli.refresh")}
          >
            <RefreshCw className={cn("size-4", isRefreshing && "animate-spin")} />
            {isRefreshing ? t("skillsCli.refreshing") : t("skillsCli.refresh")}
          </Button>
          <Button
            type="button"
            variant="outline"
            onClick={onCheckUpdates}
            disabled={updatesDisabled}
            title={mutationLockReason}
            aria-label={t("skillsCli.updates.checkUpdates")}
            data-testid="skills-cli-check-updates"
          >
            {isCheckingUpdates
              ? t("skillsCli.updates.checking")
              : t("skillsCli.updates.checkUpdates")}
          </Button>
          {isCheckingUpdates && onCancelUpdate ? (
            <Button
              type="button"
              variant="ghost"
              onClick={onCancelUpdate}
              aria-label={t("skillsCli.updates.cancelCheck")}
              data-testid="skills-cli-cancel-update"
            >
              {t("skillsCli.updates.cancelCheck")}
            </Button>
          ) : null}
          <Button
            ref={installButtonRef}
            type="button"
            onClick={onOpenInstall}
            disabled={installDisabled}
            title={mutationLockReason}
            aria-label={t("skillsCli.installSkills")}
          >
            {t("skillsCli.installSkills")}
          </Button>
        </div>
      </div>
      <dl
        className="mt-3 flex flex-wrap gap-4 text-sm"
        data-testid="skills-cli-counts"
      >
        <Count label={t("skillsCli.countInstalled")} value={counts.installed} />
        <Count label={t("skillsCli.countLinked")} value={counts.linked} />
        <Count label={t("skillsCli.countUnlinked")} value={counts.unlinked} />
        <Count
          label={t("skillsCli.countRepositories")}
          value={counts.repositories}
        />
      </dl>
      <p
        className={cn(
          "mt-2 text-xs",
          runtimeError ? "text-destructive-text" : "text-muted-foreground",
        )}
        data-testid="skills-cli-doctor"
        aria-label={t("skillsCli.runtimeStatus")}
      >
        {runtimeLabel}
      </p>
    </header>
  );
}

function Count({ label, value }: { label: string; value: number }) {
  return (
    <div>
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="font-medium tabular-nums text-foreground">{value}</dd>
    </div>
  );
}
