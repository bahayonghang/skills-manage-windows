import { KeyRound, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { GitHubPatState, SecretStorageState } from "@/types";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";

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
  onSave,
  onTest,
}: GitHubPatSettingsSectionProps) {
  const { t } = useTranslation();
  const storageLabel =
    githubPatState.configured || githubPatState.storageState === "unreadable"
      ? t(`settings.githubPatStorageState.${githubPatState.storageState}`)
      : t("settings.githubPatNotConfigured");
  const storageTone = githubPatStorageTone(githubPatState.storageState);

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <KeyRound className="size-5 text-muted-foreground" />
          <div>
            <CardTitle>{t("settings.githubPatTitle")}</CardTitle>
            <CardDescription className="mt-1">
              {t("settings.githubPatDesc")}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          <div>
            <label htmlFor="github-pat" className="mb-1 block text-xs text-muted-foreground">
              {t("settings.githubPatLabel")}
            </label>
            <Input
              id="github-pat"
              type="password"
              placeholder="github_pat_..."
              value={githubPatInput}
              onChange={(event) => onInputChange(event.target.value)}
              disabled={isLoadingGitHubPat || isSavingGitHubPat}
            />
            {githubPatState.configured && !githubPatInput ? (
              <p className="mt-2 text-xs text-muted-foreground">
                {t("settings.githubPatConfiguredNoReveal")}
              </p>
            ) : null}
          </div>

          <div className="rounded-lg border border-border/70 bg-muted/20 p-3 text-sm text-muted-foreground">
            <div className="mb-2 flex flex-wrap items-center gap-2">
              <span
                className={`rounded-full border px-2 py-0.5 text-[11px] ${storageTone}`}
              >
                {storageLabel}
              </span>
              {githubPatState.error ? (
                <span className="text-xs text-amber-600 dark:text-amber-300">
                  {t("settings.githubPatMigrationWarning")}
                </span>
              ) : null}
            </div>
            <p>{t("settings.githubPatDirectOnly")}</p>
            <p className="mt-2">{t("settings.githubPatRateLimitHint")}</p>
            <p className="mt-2">{t("settings.githubPatAppWideHint")}</p>
          </div>

          {githubPatMessage ? (
            <p
              className={
                githubPatMessage.type === "error"
                  ? "text-sm text-destructive"
                  : "text-sm text-emerald-600 dark:text-emerald-400"
              }
              role="status"
            >
              {githubPatMessage.text}
              {githubPatMessage.detail ? (
                <span className="mt-1 block text-xs opacity-80">
                  {githubPatMessage.detail}
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
              {isSavingGitHubPat ? <Loader2 className="size-4 animate-spin" /> : null}
              <span>{t("common.save")}</span>
            </Button>
            <Button
              variant="outline"
              onClick={onClear}
              disabled={
                isLoadingGitHubPat || isSavingGitHubPat || !githubPatState.configured
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
              {isTestingGitHubPat ? <Loader2 className="size-4 animate-spin" /> : null}
              <span>{t("settings.githubPatTest")}</span>
            </Button>
            {isLoadingGitHubPat ? (
              <span className="text-xs text-muted-foreground">{t("settings.loading")}</span>
            ) : null}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function githubPatStorageTone(state: SecretStorageState) {
  switch (state) {
    case "stored":
      return "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300";
    case "session":
      return "border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300";
    case "unreadable":
      return "border-destructive/40 bg-destructive/10 text-destructive";
    case "missing":
    default:
      return "border-border bg-background text-muted-foreground";
  }
}
