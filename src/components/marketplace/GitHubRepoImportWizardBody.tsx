import {
  ArrowRight,
  PartyPopper,
  RefreshCw,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type {
  GitHubRepoImportResult,
  GitHubRepoPreview,
  GitHubSkillPreview,
} from "@/types";
import { Button } from "@/components/ui/button";
import type { SelectionState } from "@/components/marketplace/githubImportWizardUtils";

interface DecisionCounts {
  write: number;
  overwrite: number;
  rename: number;
  skip: number;
}

interface GitHubRepoImportConfirmSummaryProps {
  selectedSkills: GitHubRepoPreview["skills"];
  selectionState: Record<string, SelectionState>;
  decisionCounts: DecisionCounts;
  skippedPreviewSkills: GitHubRepoPreview["skills"];
  overwriteSelections: GitHubRepoPreview["skills"];
  renamedSelections: GitHubRepoPreview["skills"];
  skippedConflictSelections: GitHubRepoPreview["skills"];
  blockingConflict: GitHubSkillPreview | undefined;
}

export function GitHubRepoImportConfirmSummary({
  selectedSkills,
  selectionState,
  decisionCounts,
  skippedPreviewSkills,
  overwriteSelections,
  renamedSelections,
  skippedConflictSelections,
  blockingConflict,
}: GitHubRepoImportConfirmSummaryProps) {
  const { t } = useTranslation();

  return (
    <div
      className="flex h-full min-h-0 flex-col overflow-hidden"
      data-testid="github-import-confirm-summary"
    >
      <div className="min-h-0 flex-1 overflow-y-auto space-y-5 pr-1">
        <div className="rounded-xl border border-border/70 bg-card/80 p-5">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="space-y-1">
              <div className="text-sm font-semibold">
                {t("marketplace.confirmImportTitle")}
              </div>
              <div className="text-sm text-muted-foreground">
                {t("marketplace.confirmImportDesc", {
                  count: selectedSkills.length,
                })}
              </div>
            </div>
          </div>

          <div
            className="mt-4 grid grid-cols-2 gap-2.5 md:grid-cols-4"
            data-testid="github-import-confirm-stats"
          >
            {(
              [
                [
                  "write",
                  t("marketplace.githubImportDecision.write"),
                  decisionCounts.write,
                ],
                [
                  "overwrite",
                  t("marketplace.githubImportDecision.overwrite"),
                  decisionCounts.overwrite,
                ],
                [
                  "rename",
                  t("marketplace.githubImportDecision.rename"),
                  decisionCounts.rename,
                ],
                [
                  "skip",
                  t("marketplace.githubImportDecision.skip"),
                  decisionCounts.skip,
                ],
              ] as const
            ).map(([key, label, value]) => (
              <div
                key={key}
                className="rounded-xl border border-border/70 bg-muted/20 px-3 py-2.5"
              >
                <div className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
                  {label}
                </div>
                <div className="mt-1 text-2xl font-semibold leading-tight">
                  {value}
                </div>
              </div>
            ))}
          </div>
          {skippedPreviewSkills.length > 0 ? (
            <div className="mt-3 text-xs text-muted-foreground">
              {t("marketplace.githubImportDecisionHintUnselected", {
                count: skippedPreviewSkills.length,
              })}
            </div>
          ) : null}
        </div>

        <div className="grid gap-4 xl:grid-cols-[minmax(0,1.25fr)_minmax(0,0.95fr)]">
          <div className="space-y-4">
            <div className="rounded-xl border border-border/70 bg-card/80 p-4">
              <div className="text-sm font-semibold">
                {t("marketplace.githubImportReadyListTitle")}
              </div>
              <div className="mt-1 text-xs text-muted-foreground">
                {t("marketplace.githubImportReadyListDesc")}
              </div>
              <ul className="mt-4 space-y-2 text-sm">
                {selectedSkills.map((skill) => {
                  const state = selectionState[skill.sourcePath];
                  const resolution = state?.resolution ?? "overwrite";
                  return (
                    <li
                      key={skill.sourcePath}
                      className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border/60 bg-background/80 px-3 py-2"
                    >
                      <div className="min-w-0">
                        <div className="font-medium">{skill.skillName}</div>
                        <div className="mt-1 text-[11px] text-muted-foreground">
                          {skill.sourcePath}
                        </div>
                      </div>
                      <div className="text-right text-xs text-muted-foreground">
                        <div>
                          {t(`marketplace.duplicateResolution.${resolution}`)}
                        </div>
                        {resolution === "rename" && state?.renamedSkillId ? (
                          <div className="mt-1 font-medium text-foreground">
                            → {state.renamedSkillId}
                          </div>
                        ) : null}
                      </div>
                    </li>
                  );
                })}
              </ul>
            </div>
          </div>

          <div className="space-y-4">
            <div className="rounded-xl border border-border/70 bg-card/80 p-4">
              <div className="text-sm font-semibold">
                {t("marketplace.githubImportConflictSummaryTitle")}
              </div>
              <div className="mt-3 space-y-3 text-sm">
                <div>
                  <div className="font-medium">
                    {t("marketplace.githubImportDecision.overwrite")}
                  </div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {overwriteSelections.length > 0
                      ? overwriteSelections.map((skill) => skill.skillName).join(", ")
                      : t("marketplace.githubImportDecisionNone")}
                  </div>
                </div>
                <div>
                  <div className="font-medium">
                    {t("marketplace.githubImportDecision.rename")}
                  </div>
                  <div className="mt-1 space-y-1 text-xs text-muted-foreground">
                    {renamedSelections.length > 0 ? (
                      renamedSelections.map((skill) => {
                        const renamedSkillId =
                          selectionState[skill.sourcePath]?.renamedSkillId;
                        return (
                          <div key={skill.sourcePath}>
                            {skill.skillName} → {renamedSkillId}
                          </div>
                        );
                      })
                    ) : (
                      <div>{t("marketplace.githubImportDecisionNone")}</div>
                    )}
                  </div>
                </div>
                <div>
                  <div className="font-medium">
                    {t("marketplace.githubImportDecision.skip")}
                  </div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {skippedConflictSelections.length > 0 ||
                    skippedPreviewSkills.length > 0
                      ? [
                          ...skippedConflictSelections.map(
                            (skill) => skill.skillName,
                          ),
                          ...skippedPreviewSkills.map((skill) => skill.skillName),
                        ].join(", ")
                      : t("marketplace.githubImportDecisionNone")}
                  </div>
                </div>
              </div>
            </div>

            {blockingConflict ? (
              <div className="rounded-xl border border-destructive/30 bg-destructive/5 p-4 text-sm text-destructive">
                {t("marketplace.resolveConflictsBeforeImport")}
              </div>
            ) : (
              <div className="rounded-xl border border-primary/20 bg-primary/5 p-4 text-sm text-muted-foreground">
                {t("marketplace.githubImportConfirmCalmHint")}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

interface GitHubRepoImportResultHubProps {
  importResult: GitHubRepoImportResult;
  canInstallImportedSkills: boolean;
  onInstallImported: (skillId: string) => void;
  onOpenCentral: () => void;
  onStartAnotherImport: () => void;
}

export function GitHubRepoImportResultHub({
  importResult,
  canInstallImportedSkills,
  onInstallImported,
  onOpenCentral,
  onStartAnotherImport,
}: GitHubRepoImportResultHubProps) {
  const { t } = useTranslation();

  return (
    <div
      className="flex h-full min-h-0 flex-col overflow-hidden"
      data-testid="github-import-result-hub"
    >
      <div className="min-h-0 flex-1 overflow-y-auto space-y-5 pr-1">
        <div className="rounded-xl border border-emerald-500/30 bg-emerald-500/5 p-5">
          <div className="flex items-start gap-3">
            <div className="rounded-full bg-emerald-500/10 p-2 text-emerald-700 dark:text-emerald-300">
              <PartyPopper className="size-5" />
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-2 text-emerald-700 dark:text-emerald-300">
                <div className="text-base font-semibold">
                  {t("marketplace.githubImportSuccessTitle")}
                </div>
                <span className="rounded-full bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium">
                  {importResult.repo.owner}/{importResult.repo.repo}
                </span>
              </div>
              <div className="mt-2 text-sm text-muted-foreground">
                {t("marketplace.githubImportSuccessDesc", {
                  count: importResult.importedSkills.length,
                })}
              </div>
            </div>
          </div>
        </div>

        <div className="grid gap-4 md:grid-cols-3">
          <ResultMetricCard
            label={t("marketplace.githubImportDecision.import")}
            value={importResult.importedSkills.length}
          />
          <ResultMetricCard
            label={t("marketplace.githubImportDecision.skip")}
            value={importResult.skippedSkills.length}
          />
          <ResultMetricCard
            label={t("marketplace.githubImportResultInstalledReady")}
            value={importResult.importedSkills.length}
          />
        </div>

        <div className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,0.95fr)]">
          <div className="rounded-xl border border-border/70 bg-card/80 p-4">
            <div className="text-sm font-semibold">
              {t("marketplace.githubImportResultImportedTitle")}
            </div>
            <ul className="mt-4 space-y-2 text-sm">
              {importResult.importedSkills.map((skill) => (
                <li
                  key={`${skill.sourcePath}-${skill.importedSkillId}`}
                  className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border/60 bg-background/80 px-3 py-2"
                >
                  <div className="min-w-0">
                    <div className="font-medium">{skill.skillName}</div>
                    <code className="mt-1 inline-flex rounded bg-muted px-2 py-1 text-[11px] text-muted-foreground">
                      {skill.importedSkillId}
                    </code>
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-xs text-muted-foreground">
                      {t(`marketplace.duplicateResolution.${skill.resolution}`)}
                    </span>
                    {canInstallImportedSkills ? (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => onInstallImported(skill.importedSkillId)}
                      >
                        <span>
                          {t("marketplace.githubImportInstallImportedSkill")}
                        </span>
                      </Button>
                    ) : null}
                  </div>
                </li>
              ))}
            </ul>
          </div>

          <div className="space-y-4">
            <div className="rounded-xl border border-border/70 bg-card/80 p-4">
              <div className="text-sm font-semibold">
                {t("marketplace.githubImportResultNextTitle")}
              </div>
              <div className="mt-1 text-xs text-muted-foreground">
                {t("marketplace.githubImportResultNextDesc")}
              </div>
              <div className="mt-4 flex flex-col gap-2">
                {importResult.importedSkills.length > 0 && canInstallImportedSkills ? (
                  <Button
                    className="justify-between"
                    onClick={() =>
                      onInstallImported(importResult.importedSkills[0].importedSkillId)
                    }
                  >
                    <span>{t("marketplace.githubImportResultActionInstall")}</span>
                    <ArrowRight className="size-4" />
                  </Button>
                ) : null}
                <Button
                  variant="outline"
                  className="justify-between"
                  onClick={onOpenCentral}
                >
                  <span>{t("marketplace.githubImportResultActionCentral")}</span>
                  <ArrowRight className="size-4" />
                </Button>
                <Button
                  variant="ghost"
                  className="justify-between"
                  onClick={onStartAnotherImport}
                >
                  <span>{t("marketplace.githubImportResultActionRestart")}</span>
                  <RefreshCw className="size-4" />
                </Button>
              </div>
            </div>

            <div className="rounded-xl border border-border/70 bg-card/80 p-4">
              <div className="text-sm font-semibold">
                {t("marketplace.githubImportResultSkippedTitle")}
              </div>
              <div className="mt-3 text-xs text-muted-foreground">
                {importResult.skippedSkills.length > 0
                  ? importResult.skippedSkills.join(", ")
                  : t("marketplace.githubImportResultSkippedNone")}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function ResultMetricCard({
  label,
  value,
}: {
  label: string;
  value: number;
}) {
  return (
    <div className="rounded-xl border border-border/70 bg-card/80 px-4 py-3">
      <div className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
        {label}
      </div>
      <div className="mt-2 text-2xl font-semibold">{value}</div>
    </div>
  );
}
