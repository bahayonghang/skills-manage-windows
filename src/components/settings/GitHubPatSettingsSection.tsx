import { KeyRound, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";

type GitHubPatMessage = { type: "success" | "error"; text: string } | null;

interface GitHubPatSettingsSectionProps {
  githubPat: string;
  githubPatInput: string;
  githubPatMessage: GitHubPatMessage;
  isGitHubPatDirty: boolean;
  isLoadingGitHubPat: boolean;
  isSavingGitHubPat: boolean;
  isTestingGitHubPat: boolean;
  onClear: () => void;
  onInputChange: (value: string) => void;
  onSave: () => void;
  onTest: () => void;
}

export function GitHubPatSettingsSection({
  githubPat,
  githubPatInput,
  githubPatMessage,
  isGitHubPatDirty,
  isLoadingGitHubPat,
  isSavingGitHubPat,
  isTestingGitHubPat,
  onClear,
  onInputChange,
  onSave,
  onTest,
}: GitHubPatSettingsSectionProps) {
  const { t } = useTranslation();

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
          </div>

          <div className="rounded-lg border border-border/70 bg-muted/20 p-3 text-sm text-muted-foreground">
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
            </p>
          ) : null}

          <div className="flex flex-wrap items-center gap-2">
            <Button
              onClick={onSave}
              disabled={isLoadingGitHubPat || isSavingGitHubPat || !isGitHubPatDirty}
            >
              {isSavingGitHubPat ? <Loader2 className="size-4 animate-spin" /> : null}
              <span>{t("common.save")}</span>
            </Button>
            <Button
              variant="outline"
              onClick={onClear}
              disabled={isLoadingGitHubPat || isSavingGitHubPat || !githubPat}
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
                !githubPat ||
                isGitHubPatDirty
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
