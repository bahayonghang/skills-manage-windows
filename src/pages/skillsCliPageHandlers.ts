import type { Dispatch, KeyboardEvent, MutableRefObject, SetStateAction } from "react";
import type { TFunction } from "i18next";

import { showSkillsCliActionToast } from "@/components/skillsCli/skillsCliActionToast";
import { formatBackendError, parseBackendError } from "@/lib/backendError";
import { isEditableEventTarget } from "@/lib/keyboardShortcuts";
import {
  emptyPlacementOutcome,
  selectedSkillsInStoreOrder,
  toggleGroupSkillSelection,
  type PlacementMutationOutcome,
} from "@/pages/skillsCliBatchModel";
import {
  buildSkillsCliDetailRows,
  summarizeDetailPlacements,
} from "@/pages/skillsCliDetailModel";
import { exportSkillsCliInventory } from "@/pages/skillsCliExport";
import {
  closeSkillsCliSurface,
  groupSkillNamesByRepositoryKey,
  applySelectionsForNames,
  openSkillsCliUpdate,
  type SkillsCliActiveSurface,
  type SkillsCliBucket,
} from "@/pages/skillsCliViewModel";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import type { SkillsCliGlobalSkill, SkillsCliInstallTarget } from "@/types";

export function toastSkillsCliPlacementOutcome(
  t: TFunction,
  outcome: PlacementMutationOutcome,
  kind: "link" | "unlink" | "update",
): void {
  const allOk = outcome.failed.length === 0 && outcome.skipped.length === 0;
  const failedMessages = [
    ...new Set(outcome.failed.map((item) => item.errorCode)),
  ].map((code) => formatBackendError(`${code}:`, t));
  const skippedMessages = [
    ...new Set(outcome.skipped.map((item) => item.reasonCode)),
  ].map((code) => formatBackendError(`${code}:`, t));
  const successKey = ((): string => {
    switch (kind) {
      case "link":
        return "skillsCli.batch.linkSuccess";
      case "unlink":
        return "skillsCli.batch.unlinkSuccess";
      case "update":
        return "skillsCli.batch.updateSuccess";
      default: {
        const _exhaustive: never = kind;
        return _exhaustive;
      }
    }
  })();
  const partialKey = ((): string => {
    switch (kind) {
      case "link":
        return "skillsCli.batch.linkPartial";
      case "unlink":
        return "skillsCli.batch.unlinkPartial";
      case "update":
        return "skillsCli.batch.updatePartial";
      default: {
        const _exhaustive: never = kind;
        return _exhaustive;
      }
    }
  })();
  showSkillsCliActionToast({
    semantic: allOk ? "success" : "error",
    message: allOk
      ? t(successKey, { succeeded: outcome.succeeded.length })
      : [
          t(partialKey, {
            succeeded: outcome.succeeded.length,
            failed: outcome.failed.length,
            skipped: outcome.skipped.length,
          }),
          ...failedMessages,
          ...skippedMessages,
        ]
          .join(" ")
          .trim(),
  });
}

export function createSkillsCliPageHandlers(input: {
  t: TFunction;
  activeSurface: SkillsCliActiveSurface;
  linkMenuOpen: boolean;
  unlinkMenuOpen: boolean;
  selectedCardNames: ReadonlySet<string>;
  targets: readonly SkillsCliInstallTarget[];
  detailSkill: SkillsCliGlobalSkill | null;
  returnFocusRef: MutableRefObject<HTMLElement | null>;
  setSelectMode: Dispatch<SetStateAction<boolean>>;
  setSelectedCardNames: Dispatch<SetStateAction<ReadonlySet<string>>>;
  setCollapsedGroupIds: Dispatch<SetStateAction<ReadonlySet<string>>>;
  setActiveSurface: Dispatch<SetStateAction<SkillsCliActiveSurface>>;
  setLinkMenuOpen: Dispatch<SetStateAction<boolean>>;
  setUnlinkMenuOpen: Dispatch<SetStateAction<boolean>>;
  setIsExporting: Dispatch<SetStateAction<boolean>>;
}) {
  const {
    t,
    activeSurface,
    linkMenuOpen,
    unlinkMenuOpen,
    selectedCardNames,
    targets,
    detailSkill,
    returnFocusRef,
    setSelectMode,
    setSelectedCardNames,
    setCollapsedGroupIds,
    setActiveSurface,
    setLinkMenuOpen,
    setUnlinkMenuOpen,
    setIsExporting,
  } = input;

  function captureReturnFocus(target: EventTarget | null) {
    if (target instanceof HTMLElement) {
      returnFocusRef.current = target;
    }
  }

  function handlePageKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key !== "Escape") {
      return;
    }
    if (
      activeSurface !== null ||
      linkMenuOpen ||
      unlinkMenuOpen ||
      event.defaultPrevented ||
      isEditableEventTarget(event.target)
    ) {
      return;
    }
    setSelectMode(false);
    setSelectedCardNames(new Set());
  }

  function handleSelectModeChange(next: boolean) {
    setSelectMode(next);
    if (!next) {
      setSelectedCardNames(new Set());
    }
  }

  function handleSelectAll(bucket: SkillsCliBucket) {
    setSelectMode(true);
    setSelectedCardNames((current) =>
      toggleGroupSkillSelection(
        current,
        bucket.skills.map((skill) => skill.name),
      ),
    );
  }

  async function handleExport(scope: "all" | "selected") {
    setIsExporting(true);
    try {
      const result = await exportSkillsCliInventory({
        scope,
        skills: useSkillsCliStore.getState().skills,
        selectedNames: selectedCardNames,
        targets,
        exportInventory: (exportInput) =>
          useSkillsCliStore.getState().exportInventory(exportInput),
      });
      if (result === "cancelled") {
        return;
      }
      showSkillsCliActionToast({
        semantic: "success",
        message: t(
          scope === "all"
            ? "skillsCli.export.successAll"
            : "skillsCli.export.successSelected",
        ),
      });
    } catch (error) {
      showSkillsCliActionToast({
        semantic: "error",
        message: t("skillsCli.export.error", {
          error: formatBackendError(error, t),
        }),
      });
    } finally {
      setIsExporting(false);
    }
  }

  async function handleLink(agentId: string) {
    setLinkMenuOpen(false);
    try {
      const outcome = await useSkillsCliStore
        .getState()
        .linkPlatformBatch([...selectedCardNames], agentId);
      toastSkillsCliPlacementOutcome(t, outcome, "link");
    } catch (error) {
      showSkillsCliActionToast({
        semantic: "error",
        message: formatBackendError(error, t),
      });
    }
  }

  async function handleUnlink() {
    setUnlinkMenuOpen(false);
    try {
      const outcome = await useSkillsCliStore
        .getState()
        .unlinkManagedBatch([...selectedCardNames]);
      toastSkillsCliPlacementOutcome(t, outcome, "unlink");
    } catch (error) {
      showSkillsCliActionToast({
        semantic: "error",
        message: formatBackendError(error, t),
      });
    }
  }

  async function handleUnlinkPlatform(agentId: string) {
    setUnlinkMenuOpen(false);
    try {
      const outcome = await useSkillsCliStore
        .getState()
        .unlinkPlatformBatch([...selectedCardNames], agentId);
      toastSkillsCliPlacementOutcome(t, outcome, "unlink");
    } catch (error) {
      showSkillsCliActionToast({
        semantic: "error",
        message: formatBackendError(error, t),
      });
    }
  }

  async function handleBatchUpdate() {
    if (useSkillsCliStore.getState().batchProgress !== null) {
      return;
    }
    const { skills, updateInventory } = useSkillsCliStore.getState();
    const ordered = selectedSkillsInStoreOrder(skills, selectedCardNames).map(
      (skill) => skill.name,
    );
    const groups = groupSkillNamesByRepositoryKey(ordered, updateInventory);
    if (groups.length === 0) {
      showSkillsCliActionToast({
        semantic: "error",
        message: t("skillsCli.updates.checkFirst"),
      });
      return;
    }
    const actionable = groups.filter(
      (group) => applySelectionsForNames(updateInventory, group.skillNames).length > 0,
    );
    if (actionable.length === 0) {
      showSkillsCliActionToast({
        semantic: "error",
        message: t("skillsCli.updates.noActionable"),
      });
      return;
    }
    try {
      const outcome = await useSkillsCliStore.getState().applyUpdatesBatch(ordered);
      if (
        outcome.succeeded.length === 0 &&
        outcome.failed.length === 0 &&
        outcome.skipped.length === 0
      ) {
        return;
      }
      toastSkillsCliPlacementOutcome(t, outcome, "update");
    } catch (error) {
      showSkillsCliActionToast({
        semantic: "error",
        message: formatBackendError(error, t),
      });
    }
  }

  function handleUninstalled(names: string[]) {
    setSelectedCardNames((current) => {
      const next = new Set(current);
      for (const name of names) {
        next.delete(name);
      }
      return next;
    });
  }

  function handleDetailClose() {
    setActiveSurface((current) =>
      current?.kind === "detail" ? closeSkillsCliSurface() : current,
    );
  }

  function openUpdateSurface(openInput: {
    repositoryKey: string | null;
    skillNames: readonly string[];
    from: EventTarget | null;
  }) {
    if (!openInput.repositoryKey) {
      showSkillsCliActionToast({
        semantic: "error",
        message: t("skillsCli.updates.checkFirst"),
      });
      return;
    }
    captureReturnFocus(openInput.from);
    setActiveSurface(
      openSkillsCliUpdate({
        repositoryKey: openInput.repositoryKey,
        skillNames: openInput.skillNames,
      }),
    );
  }

  async function handleDetailToggle(agentId: string, next: boolean) {
    if (!detailSkill) {
      return;
    }
    if (next) {
      await useSkillsCliStore.getState().linkPlatform(detailSkill.name, agentId);
      return;
    }
    await useSkillsCliStore.getState().unlinkPlatform(detailSkill.name, agentId);
  }

  async function handleDetailLinkAll() {
    if (!detailSkill) {
      return;
    }
    const rows = buildSkillsCliDetailRows(detailSkill, targets);
    const { missingAgentIds } = summarizeDetailPlacements(rows, targets);
    const outcome = emptyPlacementOutcome();
    for (const agentId of missingAgentIds) {
      try {
        await useSkillsCliStore
          .getState()
          .linkPlatform(detailSkill.name, agentId);
        outcome.succeeded.push({ skillName: detailSkill.name, agentId });
      } catch (error) {
        outcome.failed.push({
          skillName: detailSkill.name,
          agentId,
          errorCode: parseBackendError(error).code ?? "internal.unexpected",
        });
      }
    }
    toastSkillsCliPlacementOutcome(t, outcome, "link");
    const failed = outcome.failed[0];
    if (failed) {
      throw new Error(`${failed.errorCode}:`);
    }
  }

  async function handleDetailUnlinkAll() {
    if (!detailSkill) {
      return;
    }
    const outcome = await useSkillsCliStore
      .getState()
      .unlinkManagedBatch([detailSkill.name]);
    toastSkillsCliPlacementOutcome(t, outcome, "unlink");
    const failed = outcome.failed[0];
    if (failed) {
      throw new Error(`${failed.errorCode}:`);
    }
  }

  function toggleCollapsed(id: string) {
    setCollapsedGroupIds((current) => {
      const next = new Set(current);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  function toggleCardSelected(name: string) {
    setSelectedCardNames((current) => {
      const next = new Set(current);
      if (next.has(name)) {
        next.delete(name);
      } else {
        next.add(name);
      }
      return next;
    });
  }

  return {
    captureReturnFocus,
    handlePageKeyDown,
    handleSelectModeChange,
    handleSelectAll,
    handleExport,
    handleLink,
    handleUnlink,
    handleUnlinkPlatform,
    handleBatchUpdate,
    handleUninstalled,
    handleDetailClose,
    openUpdateSurface,
    handleDetailToggle,
    handleDetailLinkAll,
    handleDetailUnlinkAll,
    toggleCollapsed,
    toggleCardSelected,
  };
}
