import { useEffect, useMemo, useState } from "react";
import { Check, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  Dialog,
  DialogBody,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { buildSkillConflictSourceMap } from "@/lib/centralConflictSource";
import type {
  AgentWithStatus,
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  SkillWithLinks,
} from "@/types";
import type {
  CentralRepositoryAdditionSkipRequest,
  CentralRepositoryAdditionUnskipRequest,
  CentralRepositoryAddedSkillSelection,
  CentralRemoteAddedSkill,
  CentralRepositorySyncPreview,
} from "@/types/centralRepositorySync";
import {
  FailedRepositoriesPanel,
  PendingAdditionsPanel,
  RemoteMissingPanel,
  RepositorySyncSummary,
  RepositorySyncTabs,
  SkippedAdditionsPanel,
} from "@/components/central/CentralRepositorySyncPanels";
import {
  addedKey,
  buildDeletePreviewMaps,
  conflictSkillId,
  copyKey,
  defaultRepositorySyncTab,
  deleteRequestFromPreview,
  mergeDeleteRequest,
  missingSkillId,
  type AddedDecision,
  type MissingDecision,
  type RepositorySyncTab,
  type SkippedDecision,
  validateSkillId,
} from "@/components/central/repositorySyncUtils";

interface CentralRepositorySyncDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  preview: CentralRepositorySyncPreview | null;
  deletePreview: BatchDeleteCentralSkillPreviewResult | null;
  agents: AgentWithStatus[];
  skills: SkillWithLinks[];
  isPreviewLoading: boolean;
  isApplying: boolean;
  error: string | null;
  onConfirm: (
    keepSkillIds: string[],
    deleteRequests: BatchDeleteCentralSkillRequest[],
    additions: CentralRepositoryAddedSkillSelection[],
    skipAdditions: CentralRepositoryAdditionSkipRequest[],
    unskipAdditions: CentralRepositoryAdditionUnskipRequest[]
  ) => Promise<void>;
}

/**
 * @deprecated 使用 `UpdateCenterDialog` 代替（plans/update-mechanism-overhaul-plan.md P5/P6）。
 * 本组件保留以兼容旧 workflow（仓库同步预览/新增技能导入流程），
 * 将在下个 minor release 删除。
 */
export function CentralRepositorySyncDialog({
  open,
  onOpenChange,
  preview,
  deletePreview,
  agents,
  skills,
  isPreviewLoading,
  isApplying,
  error,
  onConfirm,
}: CentralRepositorySyncDialogProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<RepositorySyncTab>("pending");
  const [missingDecisions, setMissingDecisions] = useState<Record<string, MissingDecision>>({});
  const [addedDecisions, setAddedDecisions] = useState<Record<string, AddedDecision>>({});
  const [skippedDecisions, setSkippedDecisions] = useState<Record<string, SkippedDecision>>({});
  const [selectedCopyKeys, setSelectedCopyKeys] = useState<Set<string>>(new Set());

  const previewKey = useMemo(
    () =>
      [
        ...(preview?.remoteAdded ?? []).map((item) =>
          addedKey(item.repositoryId, item.preview.sourcePath)
        ),
        ...(preview?.skippedRemoteAdded ?? []).map((item) =>
          addedKey(item.repositoryId, item.preview.sourcePath)
        ),
        ...(preview?.remoteMissing ?? []).map(missingSkillId),
        ...(preview?.failedRepositories ?? []).map((item) => item.repositoryId),
      ].join("\0"),
    [preview]
  );

  useEffect(() => {
    if (!open || !preview) return;
    setActiveTab(defaultRepositorySyncTab(preview));
    setMissingDecisions(
      Object.fromEntries(preview.remoteMissing.map((item) => [missingSkillId(item), "keep"]))
    );
    setAddedDecisions(
      Object.fromEntries(
        preview.remoteAdded.map((item) => [
          addedKey(item.repositoryId, item.preview.sourcePath),
          {
            selected: true,
            resolution: item.preview.conflict ? "skip" : "overwrite",
            renamedSkillId: item.preview.skillId,
            deleteExisting: false,
          },
        ])
      )
    );
    setSkippedDecisions(
      Object.fromEntries(
        preview.skippedRemoteAdded.map((item) => [
          addedKey(item.repositoryId, item.preview.sourcePath),
          {
            action: "keep",
            renamedSkillId: item.preview.skillId,
          },
        ])
      )
    );
    setSelectedCopyKeys(new Set());
  }, [open, previewKey, preview]);

  const { previewBySkillId: deletePreviewBySkillId, failedBySkillId: failedDeletePreviewBySkillId } =
    useMemo(() => buildDeletePreviewMaps(deletePreview), [deletePreview]);
  const skillSourceById = useMemo(() => buildSkillConflictSourceMap(skills), [skills]);
  const agentNameById = useMemo(
    () => new Map(agents.map((agent) => [agent.id, agent.display_name])),
    [agents]
  );

  const renameErrors = useMemo(() => {
    const errors = new Map<string, string>();
    if (!preview) return errors;
    for (const item of preview.remoteAdded) {
      const key = addedKey(item.repositoryId, item.preview.sourcePath);
      const decision = addedDecisions[key];
      if (decision?.selected && !decision.deleteExisting && decision.resolution === "rename") {
        const errorMessage = validateSkillId(decision.renamedSkillId, t);
        if (errorMessage) errors.set(key, errorMessage);
      }
    }
    for (const item of preview.skippedRemoteAdded) {
      const key = addedKey(item.repositoryId, item.preview.sourcePath);
      const decision = skippedDecisions[key];
      if (decision?.action === "rename") {
        const errorMessage = validateSkillId(decision.renamedSkillId, t);
        if (errorMessage) errors.set(key, errorMessage);
      }
    }
    return errors;
  }, [addedDecisions, preview, skippedDecisions, t]);

  function updateAddedDecision(key: string, patch: Partial<AddedDecision>) {
    setAddedDecisions((current) => ({
      ...current,
      [key]: {
        ...(current[key] ?? {
          selected: true,
          resolution: "overwrite",
          renamedSkillId: "",
          deleteExisting: false,
        }),
        ...patch,
      },
    }));
  }

  function updateSkippedDecision(key: string, patch: Partial<SkippedDecision>) {
    setSkippedDecisions((current) => ({
      ...current,
      [key]: {
        ...(current[key] ?? { action: "keep", renamedSkillId: "" }),
        ...patch,
      },
    }));
  }

  function pushAdditionSelection(
    additionsByRepo: Map<string, CentralRepositoryAddedSkillSelection>,
    item: CentralRemoteAddedSkill,
    resolution: "overwrite" | "rename",
    renamedSkillId: string
  ) {
    const entry = additionsByRepo.get(item.repositoryId) ?? {
      repositoryId: item.repositoryId,
      selections: [],
    };
    entry.selections.push({
      sourcePath: item.preview.sourcePath,
      resolution,
      renamedSkillId: resolution === "rename" ? renamedSkillId.trim() : null,
    });
    additionsByRepo.set(item.repositoryId, entry);
  }

  function toggleCopy(skillId: string, agentId: string, checked: boolean) {
    setSelectedCopyKeys((current) => {
      const next = new Set(current);
      const key = copyKey(skillId, agentId);
      if (checked) next.add(key);
      else next.delete(key);
      return next;
    });
  }

  function setAllRemovableMissingDecisions(next: MissingDecision) {
    setMissingDecisions((current) => {
      if (!preview) return current;
      const decisions = { ...current };
      for (const missing of preview.remoteMissing) {
        const skillId = missingSkillId(missing);
        if (next === "delete" && !deletePreviewBySkillId.has(skillId)) continue;
        decisions[skillId] = next;
      }
      return decisions;
    });
  }

  async function handleConfirm() {
    if (!preview || renameErrors.size > 0) return;
    const keepSkillIds = preview.remoteMissing
      .filter((item) => missingDecisions[missingSkillId(item)] !== "delete")
      .map(missingSkillId);
    const deleteRequests = new Map<string, BatchDeleteCentralSkillRequest>();

    for (const missing of preview.remoteMissing) {
      if (missingDecisions[missingSkillId(missing)] !== "delete") continue;
      const item = deletePreviewBySkillId.get(missingSkillId(missing));
      if (item) mergeDeleteRequest(deleteRequests, deleteRequestFromPreview(item, selectedCopyKeys));
    }

    const additionsByRepo = new Map<string, CentralRepositoryAddedSkillSelection>();
    const skipAdditions: CentralRepositoryAdditionSkipRequest[] = [];
    const unskipAdditions: CentralRepositoryAdditionUnskipRequest[] = [];

    for (const item of preview.remoteAdded) {
      const key = addedKey(item.repositoryId, item.preview.sourcePath);
      const decision = addedDecisions[key];
      if (!decision) continue;
      if (decision.deleteExisting) {
        const existingSkillId = conflictSkillId(item);
        const deleteItem = existingSkillId ? deletePreviewBySkillId.get(existingSkillId) : null;
        if (deleteItem) mergeDeleteRequest(deleteRequests, deleteRequestFromPreview(deleteItem, selectedCopyKeys));
        continue;
      }
      if (!decision.selected) continue;
      if (decision.resolution === "skip") {
        skipAdditions.push({
          repositoryId: item.repositoryId,
          sourcePath: item.preview.sourcePath,
          skillId: item.preview.skillId,
          skillName: item.preview.skillName,
        });
        continue;
      }
      pushAdditionSelection(additionsByRepo, item, decision.resolution, decision.renamedSkillId);
    }

    for (const item of preview.skippedRemoteAdded) {
      const key = addedKey(item.repositoryId, item.preview.sourcePath);
      const decision = skippedDecisions[key] ?? { action: "keep", renamedSkillId: item.preview.skillId };
      if (decision.action === "delete") {
        const existingSkillId = conflictSkillId(item);
        const deleteItem = existingSkillId ? deletePreviewBySkillId.get(existingSkillId) : null;
        if (deleteItem) mergeDeleteRequest(deleteRequests, deleteRequestFromPreview(deleteItem, selectedCopyKeys));
        continue;
      }
      if (decision.action === "unskip") {
        unskipAdditions.push({ repositoryId: item.repositoryId, sourcePath: item.preview.sourcePath });
      }
      if (decision.action === "keep") {
        skipAdditions.push({
          repositoryId: item.repositoryId,
          sourcePath: item.preview.sourcePath,
          skillId: item.preview.skillId,
          skillName: item.preview.skillName,
        });
      }
      if (decision.action === "import" || decision.action === "rename") {
        pushAdditionSelection(
          additionsByRepo,
          item,
          decision.action === "rename" ? "rename" : "overwrite",
          decision.renamedSkillId
        );
      }
    }

    await onConfirm(
      keepSkillIds,
      Array.from(deleteRequests.values()),
      Array.from(additionsByRepo.values()),
      skipAdditions,
      unskipAdditions
    );
  }

  const counts: Record<RepositorySyncTab, number> = {
    pending: preview?.remoteAdded.length ?? 0,
    skipped: preview?.skippedRemoteAdded.length ?? 0,
    missing: preview?.remoteMissing.length ?? 0,
    failed: preview?.failedRepositories.length ?? 0,
  };
  const selectedRemoteMissingCount =
    preview?.remoteMissing.filter((item) => missingDecisions[missingSkillId(item)] === "delete")
      .length ?? 0;
  const selectedDeleteOldCount = useMemo(() => {
    const pending = preview?.remoteAdded.filter((item) => {
      const key = addedKey(item.repositoryId, item.preview.sourcePath);
      return Boolean(addedDecisions[key]?.deleteExisting);
    }).length ?? 0;
    const skipped = preview?.skippedRemoteAdded.filter((item) => {
      const key = addedKey(item.repositoryId, item.preview.sourcePath);
      return skippedDecisions[key]?.action === "delete";
    }).length ?? 0;
    return pending + skipped;
  }, [addedDecisions, preview, skippedDecisions]);
  const unavailableRemoteMissingCount =
    preview?.remoteMissing.filter((item) => !deletePreviewBySkillId.has(missingSkillId(item)))
      .length ?? 0;
  const actionCount = useMemo(() => {
    if (!preview) return 0;
    let count = preview.remoteMissing.length;
    for (const item of preview.remoteAdded) {
      const decision = addedDecisions[addedKey(item.repositoryId, item.preview.sourcePath)];
      if (decision?.deleteExisting || decision?.selected) count += 1;
    }
    for (const item of preview.skippedRemoteAdded) {
      const decision = skippedDecisions[addedKey(item.repositoryId, item.preview.sourcePath)] ?? {
        action: "keep",
      };
      if (decision.action) count += 1;
    }
    return count;
  }, [addedDecisions, preview, skippedDecisions]);
  const canApply = Boolean(preview) && !isPreviewLoading && !isApplying && actionCount > 0 && renameErrors.size === 0;

  function renderActiveTab() {
    if (!preview) return null;
    switch (activeTab) {
      case "pending":
        return (
          <PendingAdditionsPanel
            items={preview.remoteAdded}
            decisions={addedDecisions}
            renameErrors={renameErrors}
            existingSkillSources={skillSourceById}
            deletePreviewBySkillId={deletePreviewBySkillId}
            failedDeletePreviewBySkillId={failedDeletePreviewBySkillId}
            selectedCopyKeys={selectedCopyKeys}
            agentNameById={agentNameById}
            onChange={updateAddedDecision}
            onToggleCopy={toggleCopy}
          />
        );
      case "skipped":
        return (
          <SkippedAdditionsPanel
            items={preview.skippedRemoteAdded}
            decisions={skippedDecisions}
            renameErrors={renameErrors}
            existingSkillSources={skillSourceById}
            deletePreviewBySkillId={deletePreviewBySkillId}
            failedDeletePreviewBySkillId={failedDeletePreviewBySkillId}
            selectedCopyKeys={selectedCopyKeys}
            agentNameById={agentNameById}
            onChange={updateSkippedDecision}
            onToggleCopy={toggleCopy}
          />
        );
      case "missing":
        return (
          <RemoteMissingPanel
            items={preview.remoteMissing}
            decisions={missingDecisions}
            deletePreviewBySkillId={deletePreviewBySkillId}
            failedDeletePreviewBySkillId={failedDeletePreviewBySkillId}
            selectedCopyKeys={selectedCopyKeys}
            agentNameById={agentNameById}
            isPreviewLoading={isPreviewLoading}
            unavailableCount={unavailableRemoteMissingCount}
            selectedCount={selectedRemoteMissingCount}
            onChange={(skillId, decision) =>
              setMissingDecisions((current) => ({ ...current, [skillId]: decision }))
            }
            onBulkChange={setAllRemovableMissingDecisions}
            onToggleCopy={toggleCopy}
          />
        );
      case "failed":
        return <FailedRepositoriesPanel items={preview.failedRepositories} />;
      default:
        return null;
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex h-[88vh] w-[94vw] max-w-[96rem] grid-rows-none flex-col gap-0 p-0 sm:max-w-[96rem]">
        <DialogHeader className="shrink-0 border-b border-border p-5 pr-12">
          <DialogTitle>{t("central.repositorySyncTitle")}</DialogTitle>
          <DialogDescription>
            {t("central.repositorySyncLargeDialogDesc")}
          </DialogDescription>
          <DialogClose />
        </DialogHeader>
        <DialogBody className="max-h-none min-h-0 flex-1 space-y-4 overflow-y-auto p-5">
          <RepositorySyncSummary
            preview={preview}
            selectedMissingCount={selectedRemoteMissingCount}
            selectedDeleteOldCount={selectedDeleteOldCount}
            actionCount={actionCount}
          />
          <RepositorySyncTabs activeTab={activeTab} counts={counts} onTabChange={setActiveTab} />
          <div className="min-h-[28rem] rounded-xl border border-border p-4 text-sm">
            {isPreviewLoading && (
              <div className="mb-3 flex items-center gap-2 rounded-xl border border-border p-3 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                {t("central.batchDeletePreviewLoading")}
              </div>
            )}
            {renderActiveTab()}
          </div>
          {error && (
            <p className="text-xs text-destructive-text" role="alert">
              {error}
            </p>
          )}
        </DialogBody>
        <DialogFooter className="mx-0 mb-0 shrink-0 rounded-b-xl">
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isApplying}>
            {t("common.cancel")}
          </Button>
          <Button onClick={handleConfirm} disabled={!canApply} data-testid="confirm-repo-sync">
            {isApplying ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("central.repositorySyncApplying")}
              </>
            ) : (
              <>
                <Check className="size-3.5" />
                {t("central.repositorySyncApplyWithCount", { count: actionCount })}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
