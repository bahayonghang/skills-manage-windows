import { KeyRound, Loader2, ShieldCheck, Zap } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { GitHubPatState, SecretStorageState } from "@/types";

import { Button } from "@/components/ui/button";
import { SecretValueInput } from "@/components/settings/SecretValueInput";
import { SettingsCollapsibleCard } from "@/components/settings/SettingsCollapsibleCard";

type GitHubPatMessage = {
  type: "success" | "error";
  text: string;
  detail?: string | null;
} | null;

interface GitHubPatSettingsSectionProps {
  githubPatState: GitHubPatState;
  githubPatInput: string;
  githubPatMessage: GitHubPatMessage;
  isLoadingGitHubPat: boolean;
  isSavingGitHubPat: boolean;
  isTestingGitHubPat: boolean;
  onClear: () => void;
  onInputChange: (value: string) => void;
  onReveal: () => Promise<string | null>;
  onSave: () => void;
  onTest: () => void;
}

export function GitHubPatSettingsSection({
  githubPatState,
  githubPatInput,
  githubPatMessage,
  isLoadingGitHubPat,
  isSavingGitHubPat,
  isTestingGitHubPat,
  onClear,
  onInputChange,
  onReveal,
  onSave,
  onTest,
}: GitHubPatSettingsSectionProps) {
  const { t } = useTranslation();
  const [revealError, setRevealError] = useState<string | null>(null);
  const storageLabel =
    githubPatState.configured || githubPatState.storageState === "unreadable"
      ? t(`settings.githubPatStorageState.${githubPatState.storageState}`)
      : t("settings.githubPatNotConfigured");
  const storageTone = githubPatStorageTone(githubPatState.storageState);
  const effectiveMessage =
    githubPatMessage ??
    (revealError
      ? {
          type: "error" as const,
          text: t("settings.githubPatRevealFailed"),
          detail: revealError,
        }
      : null);

  useEffect(() => {
    setRevealError(null);
  }, [githubPatState.configured, githubPatInput]);

  return (
    <SettingsCollapsibleCard
      sectionId="github-pat"
      title={t("settings.githubPatTitle")}
      description={t("settings.githubPatDesc")}
      icon={<KeyRound className="size-5 shrink-0 text-muted-foreground" />}
    >
      <div className="space-y-4">
        <div className="rounded-xl border border-border/70 bg-background/70 p-3 shadow-sm">
          <SecretValueInput
            id="github-pat"
            label={t("settings.githubPatLabel")}
            value={githubPatInput}
            configured={githubPatState.configured}
            disabled={isLoadingGitHubPat || isSavingGitHubPat}
            placeholder={t("settings.githubPatPlaceholder")}
            revealScopeKey="github-pat"
            inputShowLabel={t("settings.githubPatShowInput")}
            inputHideLabel={t("settings.githubPatHideInput")}
            savedRevealLabel={t("settings.githubPatRevealSaved")}
            savedHideLabel={t("settings.githubPatHideSaved")}
            savedHiddenHint={t("settings.githubPatSavedHiddenHint")}
            savedRevealedHint={t("settings.githubPatSavedRevealedHint")}
            inputReplacementHint={t("settings.githubPatWillReplace")}
            onChange={onInputChange}
            onRevealSaved={onReveal}
            onRevealError={setRevealError}
          />
        </div>

        <div className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-2">
          <span
            className={`inline-flex items-center gap-1 rounded-full border px-2.5 py-1 ${storageTone}`}
          >
            <ShieldCheck className="size-3.5" />
            {storageLabel}
          </span>
          <span className="inline-flex items-center gap-1 rounded-full border border-border/70 bg-muted/20 px-2.5 py-1">
            <Zap className="size-3.5 text-primary" />
            {t("settings.githubPatRateLimitChip")}
          </span>
          <span className="rounded-lg border border-border/70 bg-muted/20 px-3 py-2 sm:col-span-2">
            {t("settings.githubPatDirectOnly")}
          </span>
          <span className="rounded-lg border border-border/70 bg-muted/20 px-3 py-2">
            {t("settings.githubPatRateLimitHint")}
          </span>
          <span className="rounded-lg border border-border/70 bg-muted/20 px-3 py-2">
            {t("settings.githubPatAppWideHint")}
          </span>
          {githubPatState.error ? (
            <span className="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-warning-foreground sm:col-span-2">
              {t("settings.githubPatMigrationWarning")}
            </span>
          ) : null}
        </div>

        {effectiveMessage ? (
          <p
            className={
              effectiveMessage.type === "error"
                ? "text-sm text-destructive"
                : "text-sm text-success-foreground"
            }
            role="status"
          >
            {effectiveMessage.text}
            {effectiveMessage.detail ? (
              <span className="mt-1 block text-xs opacity-80">
                {effectiveMessage.detail}
              </span>
            ) : null}
          </p>
        ) : null}

        <div className="flex flex-wrap items-center gap-2">
          <Button
            onClick={onSave}
            disabled={
              isLoadingGitHubPat || isSavingGitHubPat || !githubPatInput.trim()
            }
          >
            {isSavingGitHubPat ? (
              <Loader2 className="size-4 animate-spin" />
            ) : null}
            <span>{t("common.save")}</span>
          </Button>
          <Button
            variant="outline"
            onClick={onClear}
            disabled={
              isLoadingGitHubPat ||
              isSavingGitHubPat ||
              !githubPatState.configured
            }
          >
            <span>{t("settings.githubPatClear")}</span>
          </Button>
          <Button
            variant="outline"
            onClick={onTest}
            disabled={
              isLoadingGitHubPat ||
              isSavingGitHubPat ||
              isTestingGitHubPat ||
              !githubPatState.configured ||
              Boolean(githubPatInput.trim())
            }
          >
            {isTestingGitHubPat ? (
              <Loader2 className="size-4 animate-spin" />
            ) : null}
            <span>{t("settings.githubPatTest")}</span>
          </Button>
          {isLoadingGitHubPat ? (
            <span className="text-xs text-muted-foreground">
              {t("settings.loading")}
            </span>
          ) : null}
        </div>
      </div>
    </SettingsCollapsibleCard>
  );
}

function githubPatStorageTone(state: SecretStorageState) {
  switch (state) {
    case "stored":
      return "border-success/40 bg-success/10 text-success-foreground";
    case "session":
      return "border-warning/40 bg-warning/10 text-warning-foreground";
    case "unreadable":
      return "border-destructive/40 bg-destructive/10 text-destructive";
    case "missing":
    default:
      return "border-border bg-background text-muted-foreground";
  }
}
