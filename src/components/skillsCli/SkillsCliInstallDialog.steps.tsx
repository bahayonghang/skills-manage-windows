import type { ReactNode, RefObject } from "react";
import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  PlatformMultiSelectGrid,
  type PlatformTargetSelection,
} from "@/components/platform/PlatformMultiSelect";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import type { PlatformTarget } from "@/lib/platformTargetGroups";
import { cn } from "@/lib/utils";
import {
  INSTALL_STEPS,
  installStepStatus,
  type InstallDialogSession,
  type InstallStep,
} from "@/pages/skillsCliInstallViewModel";
import type { SkillsCliSourcePreview } from "@/types";

export function SkillsCliInstallStepper({
  step,
}: {
  step: InstallStep;
}): ReactNode {
  const { t } = useTranslation();
  const labels: Record<InstallStep, string> = {
    source: t("skillsCli.install.stepSource"),
    skills: t("skillsCli.install.stepSkills"),
    platforms: t("skillsCli.install.stepPlatforms"),
  };
  return (
    <ol
      className="flex min-w-0 flex-wrap items-center gap-1.5"
      aria-label={t("skillsCli.install.stepperAria")}
    >
      {INSTALL_STEPS.map((item, index) => {
        const status = installStepStatus(step, item);
        return (
          <li
            key={item}
            className="flex min-w-0 items-center gap-1.5"
            data-testid={`skills-cli-install-step-${item}`}
            data-status={status}
            aria-current={status === "current" ? "step" : undefined}
          >
            {index > 0 ? (
              <span
                aria-hidden
                className={cn(
                  "h-px w-4 shrink-0 sm:w-6",
                  status === "pending" ? "bg-border" : "bg-primary/40",
                )}
              />
            ) : null}
            <span
              className={cn(
                "flex min-w-0 items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs",
                status === "current" &&
                  "border-primary/40 bg-primary/10 text-primary",
                status === "completed" &&
                  "border-primary/20 bg-primary/5 text-foreground",
                status === "pending" &&
                  "border-border bg-muted/20 text-muted-foreground",
              )}
            >
              <span
                aria-hidden
                className={cn(
                  "flex size-4 shrink-0 items-center justify-center rounded-full text-ui-micro font-semibold",
                  status === "pending"
                    ? "bg-background text-muted-foreground"
                    : "bg-primary text-primary-foreground",
                )}
              >
                {index + 1}
              </span>
              <span className="truncate font-medium">{labels[item]}</span>
            </span>
          </li>
        );
      })}
    </ol>
  );
}

export function SkillsCliInstallSourceStep({
  session,
  sourceInputRef,
  recentSources,
  isRecentSourcesLoading,
  recentSourcesLoadFailed,
  previewBusy,
  onSourceChange,
  onPreview,
}: {
  session: InstallDialogSession;
  sourceInputRef: RefObject<HTMLInputElement | null>;
  recentSources: readonly string[];
  isRecentSourcesLoading: boolean;
  recentSourcesLoadFailed: boolean;
  previewBusy: boolean;
  onSourceChange: (value: string) => void;
  onPreview: (source: string) => void;
}): ReactNode {
  const { t } = useTranslation();
  return (
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">
        {t("skillsCli.install.sourceHint")}
      </p>
      <form
        className="flex min-w-0 flex-wrap gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          onPreview(session.sourceInput);
        }}
      >
        <label className="sr-only" htmlFor="skills-cli-install-source">
          {t("skillsCli.install.sourceInputAria")}
        </label>
        <Input
          ref={sourceInputRef}
          id="skills-cli-install-source"
          value={session.sourceInput}
          onChange={(event) => onSourceChange(event.target.value)}
          placeholder={t("skillsCli.sourcePlaceholder")}
          aria-label={t("skillsCli.install.sourceInputAria")}
          aria-invalid={session.submitError !== null}
          className="min-w-0 flex-1"
        />
        <Button
          type="submit"
          disabled={!session.sourceInput.trim() || previewBusy}
        >
          {previewBusy ? (
            <>
              <Loader2 className="size-3.5 animate-spin" />
              {t("skillsCli.previewing")}
            </>
          ) : (
            t("skillsCli.preview")
          )}
        </Button>
      </form>
      <div className="space-y-1.5">
        <p className="text-xs font-medium">{t("skillsCli.install.recentHeading")}</p>
        {isRecentSourcesLoading ? (
          <p className="text-xs text-muted-foreground">
            {t("skillsCli.install.recentLoading")}
          </p>
        ) : null}
        {recentSourcesLoadFailed ? (
          <p className="text-xs text-muted-foreground" role="status">
            {t("skillsCli.install.recentLoadWarning")}
          </p>
        ) : null}
        {recentSources.length === 0 &&
        !isRecentSourcesLoading &&
        !recentSourcesLoadFailed ? (
          <p className="text-xs text-muted-foreground">
            {t("skillsCli.install.recentEmpty")}
          </p>
        ) : (
          <div className="flex min-w-0 flex-wrap gap-2">
            {recentSources.map((source) => (
              <Button
                key={source}
                type="button"
                variant="outline"
                size="sm"
                disabled={previewBusy}
                aria-label={t("skillsCli.install.recentSourceAria", { source })}
                onClick={() => onPreview(source)}
              >
                <span className="max-w-[16rem] truncate">{source}</span>
              </Button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export function SkillsCliInstallSkillsStep({
  preview,
  installedNames,
  selectedSkillNames,
  headingRef,
  onToggle,
  onSelectAll,
  onClear,
}: {
  preview: SkillsCliSourcePreview;
  installedNames: ReadonlySet<string>;
  selectedSkillNames: ReadonlySet<string>;
  headingRef: RefObject<HTMLHeadingElement | null>;
  onToggle: (name: string) => void;
  onSelectAll: () => void;
  onClear: () => void;
}): ReactNode {
  const { t } = useTranslation();
  const installedCount = preview.skills.filter((name) =>
    installedNames.has(name),
  ).length;
  return (
    <div className="space-y-3">
      <div className="flex min-w-0 flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <h3
            ref={headingRef}
            tabIndex={-1}
            className="text-sm font-medium outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
          >
            {t("skillsCli.skillsHeading")}
          </h3>
          <p className="mt-1 break-all text-ui-meta text-muted-foreground">
            {preview.source}
          </p>
          <p className="text-xs text-muted-foreground">
            {t("skillsCli.install.skillsCount", {
              found: preview.skills.length,
              installed: installedCount,
            })}
          </p>
        </div>
        <div className="flex gap-2">
          <Button type="button" variant="outline" size="sm" onClick={onSelectAll}>
            {t("skillsCli.install.selectAll")}
          </Button>
          <Button type="button" variant="outline" size="sm" onClick={onClear}>
            {t("skillsCli.install.clearAll")}
          </Button>
        </div>
      </div>
      <div className="grid grid-cols-1 gap-2 @min-[28rem]/install-dialog:grid-cols-2">
        {preview.skills.map((name) => {
          const installed = installedNames.has(name);
          return (
            <label
              key={name}
              className="flex min-w-0 items-center gap-2 text-sm"
            >
              <Checkbox
                checked={selectedSkillNames.has(name)}
                onCheckedChange={() => onToggle(name)}
                aria-label={t("skillsCli.selectSkill", { name })}
              />
              <span className="min-w-0 truncate">{name}</span>
              {installed ? (
                <span className="shrink-0 text-ui-meta text-muted-foreground">
                  {t("skillsCli.install.skillInstalled")}
                </span>
              ) : null}
            </label>
          );
        })}
      </div>
    </div>
  );
}

export function SkillsCliInstallPlatformsStep({
  platformTargets,
  selection,
  platformInstalledCounts,
  commandPreview,
  headingRef,
}: {
  platformTargets: PlatformTarget[];
  selection: PlatformTargetSelection;
  platformInstalledCounts: Readonly<Record<string, number>>;
  commandPreview: string;
  headingRef: RefObject<HTMLHeadingElement | null>;
}): ReactNode {
  const { t } = useTranslation();
  return (
    <div className="space-y-3">
      <h3
        ref={headingRef}
        tabIndex={-1}
        className="text-sm font-medium outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
      >
        {t("skillsCli.platformsHeading")}
      </h3>
      <PlatformMultiSelectGrid
        targets={platformTargets}
        isSelected={selection.isSelected}
        onToggle={selection.toggle}
        showIcon
        emptyMessage={t("skillsCli.install.platformsEmpty")}
        ariaLabel={t("skillsCli.install.platformsAria")}
        renderBadges={(target) => {
          const count = platformInstalledCounts[target.id] ?? 0;
          return (
            <span className="shrink-0 text-ui-meta text-muted-foreground">
              {t("skillsCli.install.platformCount", { count })}
            </span>
          );
        }}
      />
      {commandPreview ? (
        <div className="min-w-0 space-y-1">
          <p className="text-xs font-medium">
            {t("skillsCli.install.commandPreview")}
          </p>
          <pre className="max-w-full overflow-x-auto whitespace-pre-wrap break-all rounded-md border border-border bg-muted/30 p-2 text-ui-meta">
            {commandPreview}
          </pre>
        </div>
      ) : null}
    </div>
  );
}

export function SkillsCliInstallFooter({
  step,
  installLocked,
  continueDisabled,
  installDisabled,
  onBack,
  onContinue,
  onInstall,
}: {
  step: InstallStep;
  installLocked: boolean;
  continueDisabled: boolean;
  installDisabled: boolean;
  onBack: () => void;
  onContinue: () => void;
  onInstall: () => void;
}): ReactNode {
  const { t } = useTranslation();
  return (
    <>
      {step !== "source" ? (
        <Button
          type="button"
          variant="outline"
          disabled={installLocked}
          onClick={onBack}
        >
          {t("skillsCli.install.back")}
        </Button>
      ) : null}
      {step === "skills" ? (
        <Button
          type="button"
          disabled={continueDisabled || installLocked}
          onClick={onContinue}
        >
          {t("skillsCli.install.continue")}
        </Button>
      ) : null}
      {step === "platforms" ? (
        <Button
          type="button"
          disabled={installDisabled || installLocked}
          onClick={onInstall}
        >
          {installLocked ? (
            <>
              <Loader2 className="size-3.5 animate-spin" />
              {t("skillsCli.adding")}
            </>
          ) : (
            t("skillsCli.install.install")
          )}
        </Button>
      ) : null}
    </>
  );
}

