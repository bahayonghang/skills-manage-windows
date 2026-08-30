import { useState } from "react";
import {
  AlertCircle,
  Check,
  GitBranch,
  Loader2,
  Sparkles,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  formatGitHubImportError,
  isPreviewSnapshotFailure,
  looksLikeConfiguredGitHubTokenFailure,
  looksLikeGitHubAuthGuidance,
} from "@/components/marketplace/githubImportWizardUtils";
import { cn } from "@/lib/utils";

interface GitHubRepoImportUrlInputBlockProps {
  repoUrl: string;
  branch: string;
  previewError: string | null;
  isPreviewLoading: boolean;
  browserMode: boolean;
  onRepoUrlChange: (value: string) => void;
  onBranchChange: (value: string) => void;
  onPreviewSubmit: (branch: string) => void;
}

type GitHubBranchMode = "main" | "dev" | "custom";

function getInitialBranchMode(branch: string): GitHubBranchMode {
  if (branch === "dev") return "dev";
  if (branch && branch !== "main") return "custom";
  return "main";
}

export function GitHubRepoImportUrlInputBlock({
  repoUrl,
  branch,
  previewError,
  isPreviewLoading,
  browserMode,
  onRepoUrlChange,
  onBranchChange,
  onPreviewSubmit,
}: GitHubRepoImportUrlInputBlockProps) {
  const { t } = useTranslation();
  const [branchMode, setBranchMode] = useState<GitHubBranchMode>(() =>
    getInitialBranchMode(branch),
  );
  const [customBranch, setCustomBranch] = useState(() =>
    getInitialBranchMode(branch) === "custom" ? branch : "",
  );
  const effectiveBranch =
    branchMode === "custom" ? customBranch : branchMode;

  function handleBranchModeChange(nextMode: GitHubBranchMode) {
    setBranchMode(nextMode);
    onBranchChange(nextMode === "custom" ? customBranch : nextMode);
  }

  function handleCustomBranchChange(value: string) {
    setCustomBranch(value);
    onBranchChange(value);
  }

  function handlePreviewClick() {
    if (branch !== effectiveBranch) {
      onBranchChange(effectiveBranch);
    }
    onPreviewSubmit(effectiveBranch);
  }

  return (
    <div className="mt-4 rounded-xl border border-border/70 bg-muted/10 p-4">
      <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(14rem,auto)_auto] sm:items-start">
        <div className="min-w-0">
          <label
            className="mb-2 block text-sm font-medium"
            htmlFor="github-repo-url"
          >
            {t("marketplace.githubRepoUrl")}
          </label>
          <Input
            id="github-repo-url"
            value={repoUrl}
            onChange={(event) => onRepoUrlChange(event.target.value)}
            placeholder="https://github.com/owner/repo"
            className="font-mono"
          />
        </div>
        <div className="min-w-0">
          <div
            id="github-repo-branch-label"
            className="mb-2 block text-sm font-medium"
          >
            {t("marketplace.githubBranchLabel")}
          </div>
          <div
            className="grid grid-cols-3 gap-1 rounded-lg border border-input bg-background p-1"
            role="radiogroup"
            aria-labelledby="github-repo-branch-label"
            aria-describedby="github-repo-branch-hint"
          >
            {(["main", "dev", "custom"] as GitHubBranchMode[]).map(
              (mode) => {
                const selected = branchMode === mode;
                return (
                  <button
                    key={mode}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    onClick={() => handleBranchModeChange(mode)}
                    className={cn(
                      "focus-ring flex min-h-8 min-w-0 items-center justify-center gap-1.5 rounded-md px-2 text-xs font-medium transition-[scale,background-color,color] active:scale-[0.96]",
                      selected
                        ? "bg-primary/15 text-foreground"
                        : "text-muted-foreground hover:bg-muted/65 hover:text-foreground",
                    )}
                  >
                    {selected ? (
                      <Check className="size-3.5 shrink-0" aria-hidden="true" />
                    ) : (
                      <span className="size-3.5 shrink-0" aria-hidden="true" />
                    )}
                    <span className="truncate">
                      {t(`marketplace.githubBranchOption.${mode}`)}
                    </span>
                  </button>
                );
              },
            )}
          </div>
          {branchMode === "custom" ? (
            <div className="mt-2">
              <label className="sr-only" htmlFor="github-repo-custom-branch">
                {t("marketplace.githubCustomBranchLabel")}
              </label>
              <div className="relative">
                <GitBranch className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  id="github-repo-custom-branch"
                  value={customBranch}
                  onChange={(event) =>
                    handleCustomBranchChange(event.target.value)
                  }
                  placeholder={t("marketplace.githubCustomBranchPlaceholder")}
                  aria-describedby="github-repo-branch-hint"
                  className="pl-9"
                  autoFocus
                />
              </div>
            </div>
          ) : null}
        </div>
        <Button
          className="sm:mt-7"
          onClick={handlePreviewClick}
          disabled={
            isPreviewLoading ||
            !repoUrl.trim() ||
            (branchMode === "custom" && !customBranch.trim())
          }
        >
          {isPreviewLoading ? (
            <Loader2 className="size-4 animate-spin" />
          ) : (
            <Sparkles className="size-4" />
          )}
          <span>{t("marketplace.previewImport")}</span>
        </Button>
      </div>
      <p
        id="github-repo-branch-hint"
        className="mt-2 text-xs text-muted-foreground"
      >
        {branchMode === "custom"
          ? t("marketplace.githubCustomBranchHint")
          : t("marketplace.githubBranchDefaultHint")}
      </p>
      <p className="mt-2 text-xs text-muted-foreground">
        {browserMode
          ? t("marketplace.githubImportDesktopOnlyHint")
          : t("marketplace.githubImportNoWriteHint")}
      </p>
      {browserMode ? (
        <div className="mt-3 rounded-lg border border-primary/20 bg-primary/5 px-3 py-2 text-sm text-muted-foreground">
          <div className="flex items-start gap-2">
            <AlertCircle className="mt-0.5 size-4 shrink-0 text-primary" />
            <span>{t("marketplace.githubImportDesktopOnlyState")}</span>
          </div>
        </div>
      ) : null}
      {previewError ? (
        <div className="mt-3 rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive-text">
          <div className="flex items-start gap-2">
            <AlertCircle className="mt-0.5 size-4 shrink-0" />
            <div className="space-y-2">
              <span className="block">
                {formatGitHubImportError(previewError, t)}
              </span>
              {isPreviewSnapshotFailure(previewError) ? (
                <span
                  className="block text-xs text-destructive-text"
                  data-testid="github-import-repreview-hint"
                >
                  {t("marketplace.githubImportRepreviewHint")}
                </span>
              ) : null}
              {looksLikeGitHubAuthGuidance(previewError) ? (
                <span className="block text-xs text-destructive-text">
                  {looksLikeConfiguredGitHubTokenFailure(previewError)
                    ? t("marketplace.githubPatConfiguredFailureHint")
                    : t("marketplace.githubPatSettingsHint")}
                </span>
              ) : null}
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
