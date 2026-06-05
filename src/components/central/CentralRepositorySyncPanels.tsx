import type { ReactNode } from "react";
import { AlertTriangle, Download, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  formatConflictSourceLabel,
  repositoryDisplayName,
  type SkillConflictSourceInfo,
} from "@/lib/centralConflictSource";
import type { DeleteCentralSkillPreview, DuplicateResolution } from "@/types";
import type {
  CentralRemoteAddedSkill,
  CentralRemoteMissingSkill,
  CentralRepositorySyncFailure,
  CentralRepositorySyncPreview,
} from "@/types/centralRepositorySync";
import {
  addedKey,
  conflictSkillId,
  copyKey,
  type AddedDecision,
  type MissingDecision,
  missingSkillId,
  REPOSITORY_SYNC_TABS,
  type RepositorySyncTab,
  type SkippedDecision,
} from "@/components/central/repositorySyncUtils";

const ADDED_RESOLUTIONS: readonly DuplicateResolution[] = [
  "overwrite",
  "rename",
  "skip",
];
const SKIPPED_ACTIONS: readonly SkippedDecision["action"][] = [
  "keep",
  "import",
  "rename",
  "unskip",
];

interface SummaryProps {
  preview: CentralRepositorySyncPreview | null;
  selectedMissingCount: number;
  selectedDeleteOldCount: number;
  actionCount: number;
}

export function RepositorySyncSummary({
  preview,
  selectedMissingCount,
  selectedDeleteOldCount,
  actionCount,
}: SummaryProps) {
  const { t } = useTranslation();
  const remoteAddedCount = preview?.remoteAdded.length ?? 0;
  const skippedRemoteAddedCount = preview?.skippedRemoteAdded.length ?? 0;
  const remoteMissingCount = preview?.remoteMissing.length ?? 0;
  const failedRepositoryCount = preview?.failedRepositories.length ?? 0;

  const cards = [
    {
      key: "pending",
      label: t("central.repositorySyncPendingChip", {
        count: remoteAddedCount,
      }),
      value: remoteAddedCount,
      className: "border-primary/25 bg-primary/10 text-primary",
    },
    {
      key: "skipped",
      label: t("central.repositorySyncSkippedChip", {
        count: skippedRemoteAddedCount,
      }),
      value: skippedRemoteAddedCount,
      className: "border-muted-foreground/20 bg-muted/40 text-muted-foreground",
    },
    {
      key: "missing",
      label: t("central.repositorySyncMissingChip", {
        count: remoteMissingCount,
      }),
      value: remoteMissingCount,
      className: "border-destructive/20 bg-destructive/10 text-destructive",
    },
    {
      key: "failed",
      label: t("central.repositorySyncFailedChip", {
        count: failedRepositoryCount,
      }),
      value: failedRepositoryCount,
      className: "border-warning/25 bg-warning/10 text-warning-foreground",
    },
  ];

  return (
    <section className="rounded-2xl border border-border bg-muted/20 p-4">
      <p className="text-sm text-muted-foreground">
        {t("central.repositorySyncDesc", {
          added: remoteAddedCount,
          skipped: skippedRemoteAddedCount,
          missing: remoteMissingCount,
        })}
      </p>
      <div className="mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
        {cards.map((card) => (
          <div
            key={card.key}
            className={`rounded-xl border px-3 py-2 ${card.className}`}
          >
            <div className="text-lg font-semibold leading-none">
              {card.value}
            </div>
            <div className="mt-1 text-xs font-medium">{card.label}</div>
          </div>
        ))}
      </div>
      <p className="mt-3 text-xs text-muted-foreground">
        {t("central.repositorySyncSummaryActions", {
          total: actionCount,
          deleting: selectedMissingCount + selectedDeleteOldCount,
        })}
      </p>
    </section>
  );
}

interface TabsProps {
  activeTab: RepositorySyncTab;
  counts: Record<RepositorySyncTab, number>;
  onTabChange: (tab: RepositorySyncTab) => void;
}

export function RepositorySyncTabs({
  activeTab,
  counts,
  onTabChange,
}: TabsProps) {
  const { t } = useTranslation();
  return (
    <div role="tablist" className="flex flex-wrap gap-1 border-b border-border">
      {REPOSITORY_SYNC_TABS.map((tab) => (
        <button
          key={tab}
          type="button"
          role="tab"
          aria-selected={activeTab === tab}
          onClick={() => onTabChange(tab)}
          className={
            activeTab === tab
              ? "rounded-t-lg bg-primary/10 px-3 py-2 text-sm font-medium text-primary"
              : "rounded-t-lg px-3 py-2 text-sm text-muted-foreground hover:bg-muted"
          }
        >
          {t(`central.repositorySyncTabs.${tab}`, { count: counts[tab] })}
        </button>
      ))}
    </div>
  );
}

interface DeletePreviewBlockProps {
  item: DeleteCentralSkillPreview | null;
  error: string | null;
  selectedCopyKeys: ReadonlySet<string>;
  agentNameById: ReadonlyMap<string, string>;
  onToggleCopy: (skillId: string, agentId: string, checked: boolean) => void;
}

export function DeletePreviewBlock({
  item,
  error,
  selectedCopyKeys,
  agentNameById,
  onToggleCopy,
}: DeletePreviewBlockProps) {
  const { t } = useTranslation();
  if (error) {
    return (
      <div className="mt-2 rounded-lg border border-warning/30 bg-warning/10 p-2 text-xs text-warning-foreground">
        {t("central.remoteMissingPreviewFailed", { error })}
      </div>
    );
  }
  if (!item) {
    return (
      <div className="mt-2 rounded-lg border border-dashed border-border p-2 text-xs text-muted-foreground">
        {t("central.repositorySyncDeletePreviewUnavailable")}
      </div>
    );
  }
  return (
    <div className="mt-3 space-y-3 rounded-xl border border-destructive/20 bg-destructive/5 p-3">
      <div>
        <div className="text-xs font-medium uppercase tracking-wide text-destructive">
          {t("central.repositorySyncDeletePreviewTitle")}
        </div>
        <div className="mt-1 break-all text-xs text-muted-foreground">
          {t("central.repositorySyncCentralPath", { path: item.central_path })}
        </div>
      </div>
      <div className="rounded-lg border border-border/70 bg-background/80 p-2 text-xs text-muted-foreground">
        {item.auto_removed_agent_ids.length > 0
          ? t("central.repositorySyncAutoRemovedLinks", {
              platforms: item.auto_removed_agent_ids
                .map((agentId) => agentNameById.get(agentId) ?? agentId)
                .join(", "),
            })
          : t("central.repositorySyncAutoRemovedLinksEmpty")}
      </div>
      {item.copy_installations.length > 0 && (
        <div className="space-y-2">
          <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t("central.remoteMissingCopyInstalls")}
          </div>
          {item.copy_installations.map((installation) => {
            const checked = selectedCopyKeys.has(
              copyKey(item.skill_id, installation.agent_id),
            );
            return (
              <label
                key={`${item.skill_id}:${installation.agent_id}`}
                className="flex cursor-pointer items-start gap-2 rounded-lg border border-border/70 bg-background/80 p-2 text-sm"
              >
                <Checkbox
                  checked={checked}
                  onCheckedChange={(value) =>
                    onToggleCopy(item.skill_id, installation.agent_id, !!value)
                  }
                  aria-label={t("central.remoteMissingCopyLabel", {
                    platform:
                      agentNameById.get(installation.agent_id) ??
                      installation.agent_id,
                    skill: item.skill_name,
                  })}
                />
                <span className="min-w-0">
                  <span className="block font-medium text-foreground">
                    {agentNameById.get(installation.agent_id) ??
                      installation.agent_id}
                  </span>
                  <span className="block truncate text-xs text-muted-foreground">
                    {installation.installed_path}
                  </span>
                </span>
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
}

interface PendingAdditionsPanelProps {
  items: CentralRemoteAddedSkill[];
  decisions: Record<string, AddedDecision>;
  renameErrors: ReadonlyMap<string, string>;
  existingSkillSources: ReadonlyMap<string, SkillConflictSourceInfo>;
  deletePreviewBySkillId: ReadonlyMap<string, DeleteCentralSkillPreview>;
  failedDeletePreviewBySkillId: ReadonlyMap<string, string>;
  selectedCopyKeys: ReadonlySet<string>;
  agentNameById: ReadonlyMap<string, string>;
  onChange: (key: string, patch: Partial<AddedDecision>) => void;
  onToggleCopy: (skillId: string, agentId: string, checked: boolean) => void;
}

export function PendingAdditionsPanel({
  items,
  decisions,
  renameErrors,
  existingSkillSources,
  deletePreviewBySkillId,
  failedDeletePreviewBySkillId,
  selectedCopyKeys,
  agentNameById,
  onChange,
  onToggleCopy,
}: PendingAdditionsPanelProps) {
  const { t } = useTranslation();
  const unassignedSourceLabel = t("central.unassignedSource");
  if (items.length === 0) return <EmptyTab />;

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold text-foreground">
        <Download className="mr-1 inline size-4" />
        {t("central.repositorySyncAddedTitle", { count: items.length })}
      </h3>
      <div className="space-y-2">
        {items.map((item) => {
          const key = addedKey(item.repositoryId, item.preview.sourcePath);
          const conflictId = conflictSkillId(item);
          const deletePreview = conflictId
            ? (deletePreviewBySkillId.get(conflictId) ?? null)
            : null;
          const deleteError = conflictId
            ? (failedDeletePreviewBySkillId.get(conflictId) ?? null)
            : null;
          const decision = decisions[key] ?? {
            selected: true,
            resolution: item.preview.conflict ? "skip" : "overwrite",
            renamedSkillId: item.preview.skillId,
            deleteExisting: false,
          };
          const existingConflict = conflictId
            ? existingSkillSources.get(conflictId)
            : null;
          const remoteSource = formatConflictSourceLabel(
            repositoryDisplayName(item.repo),
            item.preview.sourcePath,
            unassignedSourceLabel,
          );
          const existingSource = formatConflictSourceLabel(
            existingConflict?.repositoryLabel,
            existingConflict?.sourcePath,
            unassignedSourceLabel,
          );
          return (
            <article
              key={key}
              className="rounded-xl border border-border bg-background p-3"
            >
              <div className="flex items-start gap-3">
                <Checkbox
                  checked={decision.selected}
                  disabled={decision.deleteExisting}
                  onCheckedChange={(checked) =>
                    onChange(key, { selected: !!checked })
                  }
                  aria-label={t("central.repositorySyncSelectAdded", {
                    skill: item.preview.skillName,
                  })}
                />
                <div className="min-w-0 flex-1">
                  <div className="font-medium text-foreground">
                    {item.preview.skillName}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {item.repo.owner}/{item.repo.repo} ·{" "}
                    {item.preview.sourcePath}
                  </div>
                  {item.preview.conflict && (
                    <div className="mt-1 text-xs text-warning-foreground">
                      {t("central.repositorySyncConflict", {
                        remoteSource,
                        skill:
                          existingConflict?.skillName ??
                          item.preview.conflict.existingName,
                        existingSource,
                      })}
                    </div>
                  )}
                </div>
                <ActionGroup disabled={decision.deleteExisting}>
                  {ADDED_RESOLUTIONS.map((resolution) => (
                    <ActionButton
                      key={resolution}
                      selected={decision.resolution === resolution}
                      disabled={decision.deleteExisting}
                      onClick={() => onChange(key, { resolution })}
                    >
                      {t(`central.repositorySyncResolution.${resolution}`)}
                    </ActionButton>
                  ))}
                </ActionGroup>
              </div>
              {decision.resolution === "rename" && !decision.deleteExisting && (
                <RenameField
                  value={decision.renamedSkillId}
                  error={renameErrors.get(key)}
                  onChange={(value) => onChange(key, { renamedSkillId: value })}
                />
              )}
              {item.preview.conflict && (
                <DeleteOldSkillAction
                  selected={decision.deleteExisting}
                  disabled={!deletePreview}
                  reason={
                    deleteError ??
                    (!deletePreview
                      ? t("central.repositorySyncDeleteUnavailable")
                      : null)
                  }
                  onClick={() =>
                    onChange(key, {
                      deleteExisting: !decision.deleteExisting,
                      selected: decision.deleteExisting,
                    })
                  }
                />
              )}
              {decision.deleteExisting && (
                <DeletePreviewBlock
                  item={deletePreview}
                  error={deleteError}
                  selectedCopyKeys={selectedCopyKeys}
                  agentNameById={agentNameById}
                  onToggleCopy={onToggleCopy}
                />
              )}
            </article>
          );
        })}
      </div>
    </div>
  );
}

interface SkippedAdditionsPanelProps {
  items: CentralRemoteAddedSkill[];
  decisions: Record<string, SkippedDecision>;
  renameErrors: ReadonlyMap<string, string>;
  existingSkillSources: ReadonlyMap<string, SkillConflictSourceInfo>;
  deletePreviewBySkillId: ReadonlyMap<string, DeleteCentralSkillPreview>;
  failedDeletePreviewBySkillId: ReadonlyMap<string, string>;
  selectedCopyKeys: ReadonlySet<string>;
  agentNameById: ReadonlyMap<string, string>;
  onChange: (key: string, patch: Partial<SkippedDecision>) => void;
  onToggleCopy: (skillId: string, agentId: string, checked: boolean) => void;
}

export function SkippedAdditionsPanel(props: SkippedAdditionsPanelProps) {
  const { t } = useTranslation();
  const unassignedSourceLabel = t("central.unassignedSource");
  const {
    items,
    decisions,
    renameErrors,
    existingSkillSources,
    deletePreviewBySkillId,
    failedDeletePreviewBySkillId,
    selectedCopyKeys,
    agentNameById,
    onChange,
    onToggleCopy,
  } = props;
  if (items.length === 0) return <EmptyTab />;

  return (
    <div className="space-y-3">
      <div>
        <h3 className="text-sm font-semibold text-foreground">
          {t("central.repositorySyncSkippedTitle", { count: items.length })}
        </h3>
        <p className="mt-1 text-xs text-muted-foreground">
          {t("central.repositorySyncSkippedDesc")}
        </p>
      </div>
      <div className="space-y-2">
        {items.map((item) => {
          const key = addedKey(item.repositoryId, item.preview.sourcePath);
          const conflictId = conflictSkillId(item);
          const deletePreview = conflictId
            ? (deletePreviewBySkillId.get(conflictId) ?? null)
            : null;
          const deleteError = conflictId
            ? (failedDeletePreviewBySkillId.get(conflictId) ?? null)
            : null;
          const decision = decisions[key] ?? {
            action: "keep",
            renamedSkillId: item.preview.skillId,
          };
          const existingConflict = conflictId
            ? existingSkillSources.get(conflictId)
            : null;
          const remoteSource = formatConflictSourceLabel(
            repositoryDisplayName(item.repo),
            item.preview.sourcePath,
            unassignedSourceLabel,
          );
          const existingSource = formatConflictSourceLabel(
            existingConflict?.repositoryLabel,
            existingConflict?.sourcePath,
            unassignedSourceLabel,
          );
          return (
            <article
              key={key}
              className="rounded-xl border border-border bg-background p-3"
            >
              <div className="flex flex-wrap items-start gap-3">
                <div className="min-w-0 flex-1">
                  <div className="font-medium text-foreground">
                    {item.preview.skillName}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {item.repo.owner}/{item.repo.repo} ·{" "}
                    {item.preview.sourcePath}
                  </div>
                  {item.preview.conflict && (
                    <div className="mt-1 text-xs text-warning-foreground">
                      {t("central.repositorySyncConflict", {
                        remoteSource,
                        skill:
                          existingConflict?.skillName ??
                          item.preview.conflict.existingName,
                        existingSource,
                      })}
                    </div>
                  )}
                </div>
                <ActionGroup disabled={decision.action === "delete"}>
                  {SKIPPED_ACTIONS.map((action) => (
                    <ActionButton
                      key={action}
                      selected={decision.action === action}
                      disabled={decision.action === "delete"}
                      onClick={() => onChange(key, { action })}
                    >
                      {t(`central.repositorySyncSkippedAction.${action}`)}
                    </ActionButton>
                  ))}
                </ActionGroup>
              </div>
              {decision.action === "rename" && (
                <RenameField
                  value={decision.renamedSkillId}
                  error={renameErrors.get(key)}
                  onChange={(value) => onChange(key, { renamedSkillId: value })}
                />
              )}
              {item.preview.conflict && (
                <DeleteOldSkillAction
                  selected={decision.action === "delete"}
                  disabled={!deletePreview}
                  reason={
                    deleteError ??
                    (!deletePreview
                      ? t("central.repositorySyncDeleteUnavailable")
                      : null)
                  }
                  onClick={() =>
                    onChange(key, {
                      action: decision.action === "delete" ? "keep" : "delete",
                    })
                  }
                />
              )}
              {decision.action === "delete" && (
                <DeletePreviewBlock
                  item={deletePreview}
                  error={deleteError}
                  selectedCopyKeys={selectedCopyKeys}
                  agentNameById={agentNameById}
                  onToggleCopy={onToggleCopy}
                />
              )}
            </article>
          );
        })}
      </div>
    </div>
  );
}

interface RemoteMissingPanelProps {
  items: CentralRemoteMissingSkill[];
  decisions: Record<string, MissingDecision>;
  deletePreviewBySkillId: ReadonlyMap<string, DeleteCentralSkillPreview>;
  failedDeletePreviewBySkillId: ReadonlyMap<string, string>;
  selectedCopyKeys: ReadonlySet<string>;
  agentNameById: ReadonlyMap<string, string>;
  isPreviewLoading: boolean;
  unavailableCount: number;
  selectedCount: number;
  onChange: (skillId: string, decision: MissingDecision) => void;
  onBulkChange: (decision: MissingDecision) => void;
  onToggleCopy: (skillId: string, agentId: string, checked: boolean) => void;
}

export function RemoteMissingPanel({
  items,
  decisions,
  deletePreviewBySkillId,
  failedDeletePreviewBySkillId,
  selectedCopyKeys,
  agentNameById,
  isPreviewLoading,
  unavailableCount,
  selectedCount,
  onChange,
  onBulkChange,
  onToggleCopy,
}: RemoteMissingPanelProps) {
  const { t } = useTranslation();
  if (items.length === 0) return <EmptyTab />;

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-sm font-semibold text-foreground">
          <Trash2 className="mr-1 inline size-4" />
          {t("central.repositorySyncMissingTitle", { count: items.length })}
        </h3>
        <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => onBulkChange("delete")}
            disabled={isPreviewLoading || deletePreviewBySkillId.size === 0}
          >
            {t("central.repositorySyncDeleteAllRemovable")}
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => onBulkChange("keep")}
          >
            {t("central.repositorySyncKeepAll")}
          </Button>
        </div>
      </div>
      <p className="text-xs text-muted-foreground">
        {t("central.repositorySyncMissingSelectionSummary", {
          total: items.length,
          selected: selectedCount,
          unavailable: unavailableCount,
        })}
      </p>
      <div className="space-y-2">
        {items.map((missing) => {
          const skillId = missingSkillId(missing);
          const item = deletePreviewBySkillId.get(skillId) ?? null;
          const previewError =
            failedDeletePreviewBySkillId.get(skillId) ?? null;
          const decision = decisions[skillId] ?? "keep";
          return (
            <article
              key={skillId}
              className="rounded-xl border border-border bg-background p-3"
            >
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <div className="font-medium text-foreground">{skillId}</div>
                  <div className="mt-1 space-y-0.5 text-xs text-muted-foreground">
                    <div>
                      {missing.repo
                        ? t("central.remoteMissingRepository", {
                            repository: `${missing.repo.owner}/${missing.repo.repo}`,
                            branch: missing.repo.branch,
                          })
                        : t("central.remoteMissingRepositoryUnknown")}
                    </div>
                    <div>
                      {missing.state.source_path
                        ? t("central.remoteMissingSource", {
                            path: missing.state.source_path,
                          })
                        : t("central.remoteMissingSourceUnknown")}
                    </div>
                  </div>
                </div>
                <MissingChoice
                  decision={decision}
                  item={item}
                  onChange={(next) => onChange(skillId, next)}
                />
              </div>
              {previewError && decision !== "delete" && (
                <div className="mt-2 text-xs text-warning-foreground">
                  {t("central.remoteMissingPreviewFailed", {
                    error: previewError,
                  })}
                </div>
              )}
              {decision === "delete" && (
                <DeletePreviewBlock
                  item={item}
                  error={previewError}
                  selectedCopyKeys={selectedCopyKeys}
                  agentNameById={agentNameById}
                  onToggleCopy={onToggleCopy}
                />
              )}
            </article>
          );
        })}
      </div>
    </div>
  );
}

export function FailedRepositoriesPanel({
  items,
}: {
  items: CentralRepositorySyncFailure[];
}) {
  const { t } = useTranslation();
  if (items.length === 0) return <EmptyTab />;
  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold text-foreground">
        <AlertTriangle className="mr-1 inline size-4" />
        {t("central.repositorySyncFailedTitle", { count: items.length })}
      </h3>
      <div className="space-y-2">
        {items.map((item) => (
          <article
            key={item.repositoryId}
            className="rounded-xl border border-warning/30 bg-warning/10 p-3"
          >
            <div className="text-sm font-medium text-warning-foreground">
              {item.name || item.repositoryId}
            </div>
            <div className="mt-1 break-words text-xs text-warning-foreground">
              {item.error}
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}

function ActionGroup({
  disabled,
  children,
}: {
  disabled?: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className={`flex flex-wrap gap-1 text-xs ${disabled ? "opacity-60" : ""}`}
    >
      {children}
    </div>
  );
}

function ActionButton({
  selected,
  disabled,
  onClick,
  children,
}: {
  selected: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className={`rounded-lg border px-2 py-1 ${
        selected
          ? "border-primary bg-primary text-primary-foreground"
          : "border-border text-muted-foreground"
      }`}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function RenameField({
  value,
  error,
  onChange,
}: {
  value: string;
  error?: string;
  onChange: (value: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="mt-2 space-y-1">
      <input
        className={`w-full rounded-lg border bg-background px-3 py-2 text-sm ${
          error ? "border-destructive" : "border-border"
        }`}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        aria-label={t("central.repositorySyncRenameLabel")}
        aria-invalid={Boolean(error)}
      />
      {error && (
        <p className="text-xs text-destructive" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

function DeleteOldSkillAction({
  selected,
  disabled,
  reason,
  onClick,
}: {
  selected: boolean;
  disabled: boolean;
  reason: string | null;
  onClick: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="mt-3 border-t border-border/70 pt-3">
      <Button
        type="button"
        variant={selected ? "destructive" : "outline"}
        size="sm"
        disabled={disabled}
        onClick={onClick}
      >
        <Trash2 className="size-3.5" />
        {t("central.repositorySyncDeleteOldSkill")}
      </Button>
      <p className="mt-1 text-xs text-muted-foreground">
        {reason ?? t("central.repositorySyncDeleteOldSkillDesc")}
      </p>
    </div>
  );
}

function MissingChoice({
  decision,
  item,
  onChange,
}: {
  decision: MissingDecision;
  item: DeleteCentralSkillPreview | null;
  onChange: (decision: MissingDecision) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="grid grid-cols-2 rounded-xl border border-border/70 bg-muted/20 p-1 text-xs">
      {(["keep", "delete"] as MissingDecision[]).map((next) => (
        <button
          key={next}
          type="button"
          className={`rounded-lg px-3 py-1.5 font-medium ${
            decision === next
              ? next === "delete"
                ? "bg-destructive text-destructive-foreground"
                : "bg-primary text-primary-foreground"
              : "text-muted-foreground hover:bg-background"
          }`}
          disabled={next === "delete" && !item}
          onClick={() => onChange(next)}
        >
          {t(
            next === "delete"
              ? "central.remoteMissingDelete"
              : "central.remoteMissingKeep",
          )}
        </button>
      ))}
    </div>
  );
}

function EmptyTab() {
  const { t } = useTranslation();
  return (
    <p className="rounded-xl border border-dashed border-border bg-muted/20 p-4 text-sm text-muted-foreground">
      {t("central.repositorySyncTabEmpty")}
    </p>
  );
}
