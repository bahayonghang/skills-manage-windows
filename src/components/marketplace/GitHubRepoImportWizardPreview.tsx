import type { Dispatch, RefObject, SetStateAction } from "react";
import {
  AlertCircle,
  CheckSquare,
  ExternalLink,
  FileQuestion,
  RefreshCw,
  Sparkles,
  XSquare,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type { GitHubRepoPreview, GitHubSkillPreview } from "@/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { MarkdownPreview } from "@/components/marketplace/MarkdownPreview";
import { GitHubImportFileTree } from "@/components/marketplace/GitHubImportFileTree";
import type {
  GitHubImportAiSummaryEntry,
  SkillMarkdownEntry,
} from "@/stores/marketplaceStore";
import { cn } from "@/lib/utils";
import type {
  DetailTab,
  SelectionState,
} from "@/components/marketplace/githubImportWizardUtils";

interface GitHubImportPreviewSkillSection {
  key: string;
  label: string | null;
  skills: GitHubSkillPreview[];
}

function buildGitHubImportPreviewSkillSections(
  skills: GitHubSkillPreview[],
  otherLabel: string,
): GitHubImportPreviewSkillSection[] {
  const hasPluginGroups = skills.some((skill) => skill.pluginName?.trim());
  if (!hasPluginGroups) {
    return [{ key: "flat", label: null, skills }];
  }

  const sections: GitHubImportPreviewSkillSection[] = [];
  const sectionByPluginName = new Map<
    string,
    GitHubImportPreviewSkillSection
  >();
  const ungroupedSkills: GitHubSkillPreview[] = [];

  skills.forEach((skill) => {
    const pluginName = skill.pluginName?.trim();
    if (!pluginName) {
      ungroupedSkills.push(skill);
      return;
    }

    let section = sectionByPluginName.get(pluginName);
    if (!section) {
      section = {
        key: `plugin:${pluginName}`,
        label: pluginName,
        skills: [],
      };
      sectionByPluginName.set(pluginName, section);
      sections.push(section);
    }
    section.skills.push(skill);
  });

  if (ungroupedSkills.length > 0) {
    sections.push({
      key: "plugin:other",
      label: otherLabel,
      skills: ungroupedSkills,
    });
  }

  return sections;
}

interface GitHubRepoImportPreviewWorkspaceProps {
  preview: GitHubRepoPreview;
  selectionState: Record<string, SelectionState>;
  selectedPreviewSkill: GitHubSkillPreview | null;
  detailTab: DetailTab;
  isRenameEditing: boolean;
  browserMode: boolean;
  allSkillsSelected: boolean;
  noSkillsSelected: boolean;
  blockingFileManifest: GitHubSkillPreview | undefined;
  skillMarkdown: Record<string, SkillMarkdownEntry>;
  aiSummaries: Record<string, GitHubImportAiSummaryEntry>;
  detailScrollRef: RefObject<HTMLDivElement | null>;
  renameInputRef: RefObject<HTMLInputElement | null>;
  onSelectAll: () => void;
  onDeselectAll: () => void;
  onSelectSkillPath: (path: string) => void;
  onToggleSkillSelected: (skill: GitHubSkillPreview, selected: boolean) => void;
  onSetDetailTab: Dispatch<SetStateAction<DetailTab>>;
  onStartRenameEditing: (skill: GitHubSkillPreview) => void;
  onCancelRenameEditing: (skill: GitHubSkillPreview) => void;
  onConfirmRenameEditing: (skill: GitHubSkillPreview) => void;
  onUpdateSelection: (
    skill: GitHubSkillPreview,
    next: Partial<SelectionState>,
  ) => void;
  onRetryMarkdown: (sourcePath: string, downloadUrl: string) => void;
  onRegenerateAiSummary: (
    sourcePath: string,
    skillName: string,
    content: string,
  ) => void;
}

export function GitHubRepoImportPreviewWorkspace({
  preview,
  selectionState,
  selectedPreviewSkill,
  detailTab,
  isRenameEditing,
  browserMode,
  allSkillsSelected,
  noSkillsSelected,
  blockingFileManifest,
  skillMarkdown,
  aiSummaries,
  detailScrollRef,
  renameInputRef,
  onSelectAll,
  onDeselectAll,
  onSelectSkillPath,
  onToggleSkillSelected,
  onSetDetailTab,
  onStartRenameEditing,
  onCancelRenameEditing,
  onConfirmRenameEditing,
  onUpdateSelection,
  onRetryMarkdown,
  onRegenerateAiSummary,
}: GitHubRepoImportPreviewWorkspaceProps) {
  const { t } = useTranslation();
  const skillSections = buildGitHubImportPreviewSkillSections(
    preview.skills,
    t("marketplace.githubImportPluginGroupOther"),
  );

  return (
    <div className="flex h-full min-h-0 flex-col gap-2 overflow-y-auto">
      {blockingFileManifest ? (
        <div
          className="flex shrink-0 items-start gap-2 border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive-text"
          data-testid="github-import-file-manifest-blocker"
          role="alert"
        >
          <AlertCircle className="mt-0.5 size-3.5 shrink-0" />
          <span>{t("marketplace.githubImportFileManifestError")}</span>
        </div>
      ) : null}
      <div
        className="grid min-h-[44rem] flex-1 gap-4 md:min-h-[22rem] md:grid-cols-[minmax(260px,0.82fr)_minmax(0,1.38fr)] md:overflow-hidden lg:grid-cols-[minmax(320px,0.9fr)_minmax(0,1.45fr)] xl:grid-cols-[minmax(360px,0.88fr)_minmax(0,1.52fr)]"
        data-testid="github-import-preview-workspace"
      >
        <div className="flex min-h-[22rem] min-w-0 flex-col overflow-hidden rounded-xl border border-border/70 bg-card/70 shadow-sm md:min-h-0">
          <div className="border-b border-border/60 px-4 py-3">
            <div className="flex flex-col items-start gap-3 xl:flex-row xl:justify-between">
              <div className="min-w-0 xl:flex-1">
                <div className="flex items-center gap-2">
                  <div className="text-sm font-semibold">
                    {t("marketplace.githubImportSelectionTitle")}
                  </div>
                  <span className="rounded-full border border-border/70 bg-background/80 px-2 py-0.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {preview.skills.length}
                  </span>
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  {t("marketplace.githubImportSelectionDesc", {
                    count: preview.skills.length,
                  })}
                </div>
              </div>
              <div
                className="flex shrink-0 items-center gap-1 rounded-lg border border-border/60 bg-background/70 p-0.5"
                data-testid="github-import-bulk-selection-controls"
              >
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="h-7 gap-1 rounded-md px-2 text-ui-meta"
                  onClick={onSelectAll}
                  disabled={allSkillsSelected}
                >
                  <CheckSquare className="size-3.5" />
                  <span>{t("marketplace.githubImportSelectAllSkills")}</span>
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-7 gap-1 rounded-md px-2 text-ui-meta"
                  onClick={onDeselectAll}
                  disabled={noSkillsSelected}
                >
                  <XSquare className="size-3.5" />
                  <span>{t("marketplace.githubImportDeselectAllSkills")}</span>
                </Button>
              </div>
            </div>
          </div>

          <div
            className="min-h-0 flex-1 space-y-2 overflow-y-auto p-3"
            data-testid="github-import-summary-list"
          >
            {skillSections.map((section) => (
              <div
                key={section.key}
                className="space-y-2"
                data-testid={
                  section.label ? "github-import-skill-group" : undefined
                }
              >
                {section.label ? (
                  <div className="flex items-center justify-between gap-3 px-1 pt-1">
                    <div
                      className="min-w-0 truncate text-xs font-semibold uppercase tracking-wide text-muted-foreground"
                      data-testid="github-import-skill-group-title"
                    >
                      {section.label}
                    </div>
                    <span className="shrink-0 rounded-full border border-border/70 bg-background/80 px-1.5 py-0.5 text-ui-micro font-medium text-muted-foreground">
                      {section.skills.length}
                    </span>
                  </div>
                ) : null}
                {section.skills.map((skill) => {
                  const state = selectionState[skill.sourcePath];
                  const selected = state?.selected ?? true;
                  const isActive =
                    selectedPreviewSkill?.sourcePath === skill.sourcePath;

                  const resolution =
                    state?.resolution ??
                    (skill.conflict ? "skip" : "overwrite");

                  return (
                    <div
                      key={skill.sourcePath}
                      role="button"
                      tabIndex={0}
                      onClick={() => onSelectSkillPath(skill.sourcePath)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === " ") {
                          event.preventDefault();
                          onSelectSkillPath(skill.sourcePath);
                        }
                      }}
                      className={cn(
                        "w-full rounded-xl border p-3 text-left transition-colors",
                        isActive
                          ? "border-primary/40 bg-primary/10 shadow-sm"
                          : "border-border/70 bg-background hover:border-primary/20 hover:bg-muted/30",
                      )}
                    >
                      <div className="flex items-start gap-3">
                        <input
                          aria-label={t("marketplace.selectSkill")}
                          type="checkbox"
                          className="mt-1"
                          checked={selected}
                          onChange={(event) => {
                            event.stopPropagation();
                            onToggleSkillSelected(skill, event.target.checked);
                          }}
                          onClick={(event) => event.stopPropagation()}
                        />
                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-center gap-2">
                            <div className="truncate text-sm font-semibold">
                              {skill.skillName}
                            </div>
                          </div>
                          <div className="mt-1 line-clamp-2 text-xs text-muted-foreground">
                            {skill.description ||
                              t("marketplace.githubImportNoDescription")}
                          </div>
                          {skill.conflict ? (
                            <div
                              className="mt-3 inline-flex max-w-full items-center gap-1 rounded-lg border border-border/70 bg-muted/25 p-0.5"
                              role="group"
                              aria-label={t(
                                "marketplace.githubImportInlineResolutionLabel",
                                { name: skill.skillName },
                              )}
                              onClick={(event) => event.stopPropagation()}
                            >
                              {(["skip", "overwrite"] as const).map(
                                (nextResolution) => {
                                  const isCurrent =
                                    resolution === nextResolution;
                                  return (
                                    <Button
                                      key={nextResolution}
                                      type="button"
                                      variant={isCurrent ? "default" : "ghost"}
                                      size="sm"
                                      className={cn(
                                        "h-7 rounded-md px-2.5 text-ui-meta",
                                        isCurrent
                                          ? "shadow-sm"
                                          : "text-muted-foreground hover:text-foreground",
                                      )}
                                      aria-pressed={isCurrent}
                                      onClick={() =>
                                        onUpdateSelection(skill, {
                                          resolution: nextResolution,
                                        })
                                      }
                                    >
                                      {t(
                                        `marketplace.duplicateResolution.${nextResolution}`,
                                      )}
                                    </Button>
                                  );
                                },
                              )}
                            </div>
                          ) : null}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
        </div>

        <div
          className="flex min-h-[22rem] min-w-0 flex-col overflow-hidden rounded-xl border border-border/70 bg-card/80 shadow-sm md:min-h-0"
          data-testid="github-import-detail-pane"
        >
          {selectedPreviewSkill ? (
            <GitHubRepoImportPreviewDetailPane
              preview={preview}
              selectedPreviewSkill={selectedPreviewSkill}
              selectionState={selectionState}
              detailTab={detailTab}
              isRenameEditing={isRenameEditing}
              browserMode={browserMode}
              skillMarkdown={skillMarkdown}
              aiSummaries={aiSummaries}
              detailScrollRef={detailScrollRef}
              renameInputRef={renameInputRef}
              onSetDetailTab={onSetDetailTab}
              onStartRenameEditing={onStartRenameEditing}
              onCancelRenameEditing={onCancelRenameEditing}
              onConfirmRenameEditing={onConfirmRenameEditing}
              onUpdateSelection={onUpdateSelection}
              onRetryMarkdown={onRetryMarkdown}
              onRegenerateAiSummary={onRegenerateAiSummary}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}

interface GitHubRepoImportPreviewDetailPaneProps {
  preview: GitHubRepoPreview;
  selectedPreviewSkill: GitHubSkillPreview;
  selectionState: Record<string, SelectionState>;
  detailTab: DetailTab;
  isRenameEditing: boolean;
  browserMode: boolean;
  skillMarkdown: Record<string, SkillMarkdownEntry>;
  aiSummaries: Record<string, GitHubImportAiSummaryEntry>;
  detailScrollRef: RefObject<HTMLDivElement | null>;
  renameInputRef: RefObject<HTMLInputElement | null>;
  onSetDetailTab: Dispatch<SetStateAction<DetailTab>>;
  onStartRenameEditing: (skill: GitHubSkillPreview) => void;
  onCancelRenameEditing: (skill: GitHubSkillPreview) => void;
  onConfirmRenameEditing: (skill: GitHubSkillPreview) => void;
  onUpdateSelection: (
    skill: GitHubSkillPreview,
    next: Partial<SelectionState>,
  ) => void;
  onRetryMarkdown: (sourcePath: string, downloadUrl: string) => void;
  onRegenerateAiSummary: (
    sourcePath: string,
    skillName: string,
    content: string,
  ) => void;
}

function GitHubRepoImportPreviewDetailPane({
  preview,
  selectedPreviewSkill,
  selectionState,
  detailTab,
  isRenameEditing,
  browserMode,
  skillMarkdown,
  aiSummaries,
  detailScrollRef,
  renameInputRef,
  onSetDetailTab,
  onStartRenameEditing,
  onCancelRenameEditing,
  onConfirmRenameEditing,
  onUpdateSelection,
  onRetryMarkdown,
  onRegenerateAiSummary,
}: GitHubRepoImportPreviewDetailPaneProps) {
  const { t } = useTranslation();
  const currentSelection = selectionState[selectedPreviewSkill.sourcePath];
  const currentResolution =
    currentSelection?.resolution ??
    (selectedPreviewSkill.conflict ? "skip" : "overwrite");
  const skillGithubHref = `https://github.com/${preview.repo.owner}/${preview.repo.repo}/blob/${preview.repo.branch}/${selectedPreviewSkill.sourcePath}`;
  const resolvedRenameId =
    currentSelection?.renamedSkillId?.trim() || selectedPreviewSkill.skillId;
  const statusBadgeClassName = !selectedPreviewSkill.conflict
    ? "border-success/30 bg-success/10 text-success-foreground"
    : currentResolution === "overwrite"
      ? "border-warning/30 bg-warning/10 text-warning-foreground"
      : currentResolution === "rename"
        ? "border-info/30 bg-info/10 text-info-foreground"
        : "border-warning/30 bg-warning/10 text-warning-foreground";
  const aiSummary = aiSummaries[selectedPreviewSkill.sourcePath];
  const markdownEntry = skillMarkdown[selectedPreviewSkill.sourcePath];

  return (
    <>
      <div className="border-b border-border/60 bg-background/50 px-5 pt-4 pb-3">
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1">
            {!isRenameEditing ? (
              <div className="flex flex-wrap items-center gap-2">
                <div className="min-w-0 truncate text-base font-semibold">
                  {selectedPreviewSkill.skillName}
                </div>
                <code className="shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-ui-meta text-muted-foreground">
                  {selectedPreviewSkill.skillId}
                </code>
                <span
                  className={cn(
                    "inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium",
                    statusBadgeClassName,
                  )}
                >
                  {!selectedPreviewSkill.conflict
                    ? t("marketplace.githubImportStatusReady")
                    : currentResolution === "overwrite"
                      ? t("marketplace.githubImportStatusWillOverwrite", {
                          name: selectedPreviewSkill.conflict.existingName,
                        })
                      : currentResolution === "rename"
                        ? t("marketplace.githubImportStatusWillRename", {
                            id: resolvedRenameId,
                          })
                        : t("marketplace.githubImportStatusWillSkip", {
                            name: selectedPreviewSkill.conflict.existingName,
                          })}
                </span>
              </div>
            ) : (
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  {t("marketplace.githubImportIdLabel")}
                </span>
                <Input
                  ref={renameInputRef}
                  value={
                    currentSelection?.renamedSkillId ??
                    selectedPreviewSkill.skillId
                  }
                  onChange={(event) =>
                    onUpdateSelection(selectedPreviewSkill, {
                      renamedSkillId: event.target.value,
                    })
                  }
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      onConfirmRenameEditing(selectedPreviewSkill);
                    } else if (event.key === "Escape") {
                      event.preventDefault();
                      onCancelRenameEditing(selectedPreviewSkill);
                    }
                  }}
                  placeholder={t("marketplace.renameSkillIdPlaceholder")}
                  className="h-8 w-[min(24rem,100%)] font-mono text-sm"
                />
                <Button
                  type="button"
                  size="sm"
                  onClick={() => onConfirmRenameEditing(selectedPreviewSkill)}
                >
                  {t("common.confirm")}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => onCancelRenameEditing(selectedPreviewSkill)}
                >
                  {t("common.cancel")}
                </Button>
              </div>
            )}
            <div className="mt-1 break-all text-ui-meta text-muted-foreground">
              {!isRenameEditing ? (
                <>
                  {selectedPreviewSkill.sourcePath}
                  {selectedPreviewSkill.rootDirectory
                    ? ` · ${t("marketplace.githubImportRootDirectory")}: ${selectedPreviewSkill.rootDirectory}`
                    : ""}
                </>
              ) : (
                <>
                  {selectedPreviewSkill.skillName}
                  {" · "}
                  {selectedPreviewSkill.sourcePath}
                </>
              )}
            </div>
          </div>
          <div className="ml-auto flex shrink-0 flex-wrap items-center justify-end gap-2">
            {!isRenameEditing && selectedPreviewSkill.conflict ? (
              currentResolution === "overwrite" ? (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() =>
                    onUpdateSelection(selectedPreviewSkill, {
                      resolution: "skip",
                    })
                  }
                >
                  {t("marketplace.githubImportStatusResetDefault")}
                </Button>
              ) : currentResolution === "rename" ? (
                <>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => onStartRenameEditing(selectedPreviewSkill)}
                  >
                    {t("marketplace.githubImportStatusChangeToRename")}
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() =>
                      onUpdateSelection(selectedPreviewSkill, {
                        resolution: "skip",
                      })
                    }
                  >
                    {t("marketplace.githubImportStatusResetDefault")}
                  </Button>
                </>
              ) : (
                <>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() =>
                      onUpdateSelection(selectedPreviewSkill, {
                        resolution: "overwrite",
                      })
                    }
                  >
                    {t("marketplace.githubImportStatusChangeToOverwrite")}
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => onStartRenameEditing(selectedPreviewSkill)}
                  >
                    {t("marketplace.githubImportStatusChangeToRename")}
                  </Button>
                </>
              )
            ) : null}
            {skillGithubHref && !isRenameEditing ? (
              <a
                href={skillGithubHref}
                target="_blank"
                rel="noreferrer"
                className="inline-flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                aria-label={t("marketplace.githubImportOpenOnGithub")}
                title={t("marketplace.githubImportOpenOnGithub")}
              >
                <ExternalLink className="size-3.5" />
              </a>
            ) : null}
          </div>
        </div>
      </div>

      <div
        className="border-b border-border/60 px-5"
        data-testid="github-import-detail-tabs"
      >
        <div className="flex gap-5">
          {(
            [
              ["overview", t("marketplace.githubImportDetailTabs.overview")],
              ["files", t("marketplace.githubImportDetailTabs.files")],
              ["ai", t("marketplace.githubImportDetailTabs.ai")],
            ] as const
          ).map(([tabId, label]) => {
            const isActive = detailTab === tabId;
            return (
              <button
                key={tabId}
                type="button"
                onClick={() => onSetDetailTab(tabId)}
                aria-selected={isActive}
                className={cn(
                  "relative -mb-px inline-flex items-center gap-1.5 border-b-2 px-0.5 py-2.5 text-xs font-medium transition-colors",
                  isActive
                    ? "border-primary text-foreground"
                    : "border-transparent text-muted-foreground hover:text-foreground",
                )}
                data-testid={`github-import-detail-tab-${tabId}`}
              >
                <span>{label}</span>
              </button>
            );
          })}
        </div>
      </div>

      <div
        ref={detailScrollRef}
        className={cn(
          "min-h-0 flex-1 px-5 py-4",
          detailTab === "files" ? "overflow-hidden" : "overflow-y-auto",
        )}
        data-testid="github-import-detail-scroll"
      >
        {detailTab === "overview" ? (
          <div data-testid="github-import-detail-panel-overview">
            {browserMode ? (
              <div className="flex flex-col items-center gap-2 py-10 text-center text-sm text-muted-foreground">
                <FileQuestion className="size-6 text-muted-foreground" />
                <span>
                  {t("marketplace.githubImportMarkdownBrowserFallback")}
                </span>
              </div>
            ) : !markdownEntry || markdownEntry.status === "loading" ? (
              <div className="space-y-3 py-2">
                <div className="h-3 w-3/5 animate-pulse rounded bg-muted" />
                <div className="h-3 w-11/12 animate-pulse rounded bg-muted" />
                <div className="h-3 w-4/5 animate-pulse rounded bg-muted" />
                <div className="mt-5 h-3 w-2/3 animate-pulse rounded bg-muted" />
                <div className="h-3 w-5/6 animate-pulse rounded bg-muted" />
              </div>
            ) : markdownEntry.status === "error" ? (
              <div className="flex flex-col items-center gap-3 py-10 text-center">
                <AlertCircle className="size-6 text-destructive-text" />
                <div className="text-sm text-muted-foreground">
                  {t("marketplace.githubImportMarkdownError")}
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    onRetryMarkdown(
                      selectedPreviewSkill.sourcePath,
                      selectedPreviewSkill.downloadUrl,
                    )
                  }
                >
                  <RefreshCw className="size-3.5" />
                  <span>{t("marketplace.githubImportMarkdownRetry")}</span>
                </Button>
              </div>
            ) : !markdownEntry.content?.trim() ? (
              <div className="flex flex-col items-center gap-2 py-10 text-center text-sm text-muted-foreground">
                <FileQuestion className="size-6 text-muted-foreground" />
                <span>{t("marketplace.githubImportMarkdownEmpty")}</span>
              </div>
            ) : (
              <MarkdownPreview content={markdownEntry.content ?? ""} />
            )}
          </div>
        ) : null}

        {detailTab === "files" ? (
          <div
            className="h-full min-h-0"
            data-testid="github-import-detail-panel-files"
          >
            <GitHubImportFileTree
              files={selectedPreviewSkill.files}
              rootName={
                currentResolution === "rename"
                  ? resolvedRenameId
                  : selectedPreviewSkill.skillId
              }
            />
          </div>
        ) : null}

        {detailTab === "ai" ? (
          <div
            className="space-y-4"
            data-testid="github-import-detail-panel-ai"
          >
            <div className="rounded-lg border border-border/70 bg-muted/20 p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="flex items-center gap-2 text-sm font-medium">
                    <Sparkles className="size-4 text-primary" />
                    <span>{t("marketplace.githubImportAiSummaryTitle")}</span>
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {t("marketplace.githubImportAiSummaryDesc")}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    const content =
                      markdownEntry?.status === "ready" &&
                      markdownEntry.content?.trim()
                        ? markdownEntry.content
                        : (selectedPreviewSkill.description?.trim() ?? "");
                    if (!content) return;
                    onRegenerateAiSummary(
                      selectedPreviewSkill.sourcePath,
                      selectedPreviewSkill.skillName,
                      content,
                    );
                  }}
                  disabled={aiSummary?.isLoading}
                >
                  <RefreshCw className="size-3.5" />
                  <span>{t("detail.regenerateExplanation")}</span>
                </Button>
              </div>
              {aiSummary?.isLoading || aiSummary?.isStreaming ? (
                <div className="mt-3 flex items-center gap-2 text-sm text-muted-foreground">
                  <RefreshCw className="size-4 animate-spin" />
                  {aiSummary?.summary
                    ? t("detail.explanationStreaming")
                    : t("detail.explanationLoading")}
                </div>
              ) : null}
              {aiSummary?.summary ? (
                <div className="mt-3 whitespace-pre-wrap text-sm leading-6 text-muted-foreground">
                  {aiSummary.summary}
                </div>
              ) : aiSummary?.error ? (
                <div className="mt-3 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive-text">
                  {aiSummary.error}
                </div>
              ) : selectedPreviewSkill.description ? (
                <p className="mt-3 text-sm leading-6 text-muted-foreground">
                  {t("marketplace.githubImportAiSummaryBody", {
                    name: selectedPreviewSkill.skillName,
                    description: selectedPreviewSkill.description,
                  })}
                </p>
              ) : (
                <p className="mt-3 text-sm leading-6 text-muted-foreground">
                  {t("marketplace.githubImportAiSummaryFallback", {
                    name: selectedPreviewSkill.skillName,
                  })}
                </p>
              )}
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}
