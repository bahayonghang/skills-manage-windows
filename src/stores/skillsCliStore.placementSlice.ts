import { invoke } from "@/lib/ipc";
import { backendErrorStateValue, parseBackendError } from "@/lib/backendError";
import {
  emptyPlacementOutcome,
  partitionLinkBatch,
  partitionUnlinkBatch,
  partitionUnlinkBatchForAgent,
  selectedSkillsInStoreOrder,
  SKILLS_CLI_SKIP_ALREADY_LINKED,
  SKILLS_CLI_SKIP_NO_PLACEMENT,
  SKILLS_CLI_SKIP_NOT_LINKED,
  type PlacementMutationOutcome,
  type PlacementPartitionItem,
} from "@/pages/skillsCliBatchModel";
import {
  applySelectionsForNames,
  groupSkillNamesByRepositoryKey,
} from "@/pages/skillsCliViewModel";
import type { SkillsCliGlobalSkill, SkillsCliPlacement } from "@/types";
import {
  BUSY_ENVELOPE,
  newJobId,
  skillsCliOperationBusy,
} from "./skillsCliStore.updateSlice";
import { useTargetStore } from "./targetStore";
import type {
  SkillsCliBatchOperation,
  SkillsCliState,
} from "./skillsCliStore.types";

type SkillsCliSetter = (patch: Partial<SkillsCliState>) => void;
type SkillsCliGetter = () => SkillsCliState;

export function errorCodeFrom(error: unknown): string {
  return parseBackendError(error).code ?? "internal.unexpected";
}

function beginBatchProgress(
  set: SkillsCliSetter,
  operation: SkillsCliBatchOperation,
  total: number,
): void {
  set({
    batchProgress: { operation, completed: 0, total },
  });
}

function incrementBatchProgress(
  get: SkillsCliGetter,
  set: SkillsCliSetter,
): void {
  const current = get().batchProgress;
  if (!current) {
    return;
  }
  set({
    batchProgress: {
      ...current,
      completed: current.completed + 1,
    },
  });
}

function clearBatchProgress(set: SkillsCliSetter): void {
  set({ batchProgress: null });
}

function batchAlreadyRunning(state: SkillsCliState): boolean {
  return state.batchProgress !== null;
}

function replacePlacement(
  skills: SkillsCliGlobalSkill[],
  skillName: string,
  agentId: string,
  next: SkillsCliPlacement,
): SkillsCliGlobalSkill[] {
  return skills.map((skill) => {
    if (skill.name !== skillName) {
      return skill;
    }
    return {
      ...skill,
      placements: skill.placements.map((placement) =>
        placement.agentId === agentId ? next : placement,
      ),
    };
  });
}

function omitSkill(
  skills: SkillsCliGlobalSkill[],
  skillName: string,
): SkillsCliGlobalSkill[] {
  return skills.filter((skill) => skill.name !== skillName);
}

function restoreSkill(
  skills: SkillsCliGlobalSkill[],
  snapshot: SkillsCliGlobalSkill,
): SkillsCliGlobalSkill[] {
  if (skills.some((skill) => skill.name === snapshot.name)) {
    return skills.map((skill) =>
      skill.name === snapshot.name ? snapshot : skill,
    );
  }
  return [...skills, snapshot];
}

function throwIfSingleOutcomeFailed(outcome: PlacementMutationOutcome): void {
  const failed = outcome.failed[0];
  if (failed) {
    throw new Error(`${failed.errorCode}:`);
  }
  const skipped = outcome.skipped[0];
  if (skipped && outcome.succeeded.length === 0) {
    throw new Error(`${skipped.reasonCode}:`);
  }
}

function optimisticPlacement(
  previous: SkillsCliPlacement,
  kind: "link" | "unlink",
): SkillsCliPlacement {
  if (kind === "link") {
    return {
      ...previous,
      state: "managed_link",
      managedLinkKind: previous.managedLinkKind ?? "windows_junction",
      reasonCode: null,
    };
  }
  return {
    ...previous,
    state: "missing",
    managedLinkKind: null,
    reasonCode: null,
  };
}

function isRemoteSkillsCliTarget(): boolean {
  const kind = useTargetStore.getState().activeTarget.kind;
  return kind === "ssh" || kind === "wsl";
}

function placementKey(skillName: string, agentId: string): string {
  return `${skillName}\0${agentId}`;
}

async function refreshInventoryAfterMutation(
  get: SkillsCliGetter,
  set: SkillsCliSetter,
): Promise<void> {
  try {
    await get().loadAll();
  } catch (error) {
    set({
      isLoading: false,
      isRefreshing: false,
      inventoryError: get().inventoryError ?? backendErrorStateValue(error),
    });
  }
}

async function runPlacementBatch(
  get: SkillsCliGetter,
  set: SkillsCliSetter,
  items: PlacementPartitionItem[],
  kind: "link" | "unlink",
  force = false,
): Promise<PlacementMutationOutcome> {
  const outcome = emptyPlacementOutcome();
  if (items.length === 0) {
    return outcome;
  }
  if (isRemoteSkillsCliTarget()) {
    return runRemotePlacementBatch(get, set, items, kind, force);
  }
  const operation: SkillsCliBatchOperation = kind === "link" ? "link" : "unlink";
  set({ isMutating: true, actionError: null });
  beginBatchProgress(set, operation, items.length);
  try {
    for (const item of items) {
      const jobId = newJobId();
      set({ jobId });
      const previous =
        get().skills
          .find((skill) => skill.name === item.skillName)
          ?.placements.find((placement) => placement.agentId === item.agentId) ??
        item.placement;
      set({
        skills: replacePlacement(
          get().skills,
          item.skillName,
          item.agentId,
          optimisticPlacement(previous, kind),
        ),
      });
      try {
        const result =
          kind === "link"
            ? await invoke("skills_cli_link_platform", {
                jobId,
                skillName: item.skillName,
                skillportAgentId: item.agentId,
              })
            : await invoke("skills_cli_unlink_platform", {
                jobId,
                skillName: item.skillName,
                skillportAgentId: item.agentId,
                force,
              });
        if (get().jobId === jobId && result) {
          set({
            skills: replacePlacement(
              get().skills,
              item.skillName,
              item.agentId,
              result,
            ),
          });
        }
        outcome.succeeded.push({
          skillName: item.skillName,
          agentId: item.agentId,
        });
      } catch (error) {
        if (get().jobId === jobId) {
          set({
            skills: replacePlacement(
              get().skills,
              item.skillName,
              item.agentId,
              previous,
            ),
            actionError: backendErrorStateValue(error),
          });
        }
        outcome.failed.push({
          skillName: item.skillName,
          agentId: item.agentId,
          errorCode: errorCodeFrom(error),
        });
      }
      incrementBatchProgress(get, set);
    }
  } finally {
    clearBatchProgress(set);
    if (get().isMutating) {
      set({ isMutating: false, jobId: null });
    }
    await refreshInventoryAfterMutation(get, set);
  }
  return outcome;
}

async function runRemotePlacementBatch(
  get: SkillsCliGetter,
  set: SkillsCliSetter,
  items: PlacementPartitionItem[],
  kind: "link" | "unlink",
  force = false,
): Promise<PlacementMutationOutcome> {
  const outcome = emptyPlacementOutcome();
  const operation: SkillsCliBatchOperation = kind === "link" ? "link" : "unlink";
  const defaultSkip =
    kind === "link" ? SKILLS_CLI_SKIP_ALREADY_LINKED : SKILLS_CLI_SKIP_NOT_LINKED;
  const previousByKey = new Map<string, SkillsCliPlacement>();
  set({ isMutating: true, actionError: null });
  beginBatchProgress(set, operation, items.length);
  const jobId = newJobId();
  set({ jobId });
  try {
    for (const item of items) {
      const previous =
        get().skills
          .find((skill) => skill.name === item.skillName)
          ?.placements.find((placement) => placement.agentId === item.agentId) ??
        item.placement;
      previousByKey.set(placementKey(item.skillName, item.agentId), previous);
      set({
        skills: replacePlacement(
          get().skills,
          item.skillName,
          item.agentId,
          optimisticPlacement(previous, kind),
        ),
      });
    }
    const batchItems = items.map((item) => ({
      skillName: item.skillName,
      skillportAgentId: item.agentId,
    }));
    let result;
    switch (kind) {
      case "link":
        result = await invoke("skills_cli_link_platform_batch", {
          jobId,
          items: batchItems,
        });
        break;
      case "unlink":
        result = await invoke("skills_cli_unlink_platform_batch", {
          jobId,
          items: batchItems,
          force,
        });
        break;
      default: {
        const _exhaustive: never = kind;
        throw new Error(_exhaustive);
      }
    }
    if (get().jobId === jobId) {
      for (const item of result.failed) {
        const previous = previousByKey.get(
          placementKey(item.skillName, item.agentId),
        );
        if (previous) {
          set({
            skills: replacePlacement(
              get().skills,
              item.skillName,
              item.agentId,
              previous,
            ),
          });
        }
      }
      for (const item of result.skipped) {
        const previous = previousByKey.get(
          placementKey(item.skillName, item.agentId),
        );
        if (previous) {
          set({
            skills: replacePlacement(
              get().skills,
              item.skillName,
              item.agentId,
              previous,
            ),
          });
        }
      }
    }
    for (const item of result.succeeded) {
      outcome.succeeded.push({
        skillName: item.skillName,
        agentId: item.agentId,
      });
    }
    for (const item of result.failed) {
      outcome.failed.push({
        skillName: item.skillName,
        agentId: item.agentId,
        errorCode: item.errorCode,
      });
    }
    for (const item of result.skipped) {
      outcome.skipped.push({
        skillName: item.skillName,
        agentId: item.agentId,
        reasonCode: defaultSkip,
      });
    }
    const progress = get().batchProgress;
    if (progress) {
      set({
        batchProgress: { ...progress, completed: progress.total },
      });
    }
  } catch (error) {
    if (get().jobId === jobId) {
      for (const item of items) {
        const previous = previousByKey.get(
          placementKey(item.skillName, item.agentId),
        );
        if (previous) {
          set({
            skills: replacePlacement(
              get().skills,
              item.skillName,
              item.agentId,
              previous,
            ),
          });
        }
      }
      set({ actionError: backendErrorStateValue(error) });
    }
    const errorCode = errorCodeFrom(error);
    for (const item of items) {
      outcome.failed.push({
        skillName: item.skillName,
        agentId: item.agentId,
        errorCode,
      });
    }
  } finally {
    clearBatchProgress(set);
    if (get().isMutating) {
      set({ isMutating: false, jobId: null });
    }
    await refreshInventoryAfterMutation(get, set);
  }
  return outcome;
}

export function createSkillsCliPlacementSlice(
  set: SkillsCliSetter,
  get: SkillsCliGetter,
): Pick<
  SkillsCliState,
  | "linkPlatform"
  | "unlinkPlatform"
  | "linkPlatformBatch"
  | "unlinkManagedBatch"
  | "unlinkPlatformBatch"
  | "removeGlobalBatch"
  | "applyUpdatesBatch"
> {
  return {
    async linkPlatform(skillName, agentId) {
      const outcome = await get().linkPlatformBatch([skillName], agentId);
      throwIfSingleOutcomeFailed(outcome);
    },

    async unlinkPlatform(skillName, agentId, options) {
      if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
        throw new Error(BUSY_ENVELOPE);
      }
      const force = options?.force === true;
      if (force) {
        const placement = get()
          .skills.find((skill) => skill.name === skillName)
          ?.placements.find((item) => item.agentId === agentId);
        if (placement?.state === "conflict") {
          const batchOutcome = await runPlacementBatch(
            get,
            set,
            [{ skillName, agentId, placement }],
            "unlink",
            true,
          );
          throwIfSingleOutcomeFailed(batchOutcome);
          return;
        }
      }
      const partition = partitionUnlinkBatch(get().skills, [skillName]);
      const match = partition.allowed.filter((item) => item.agentId === agentId);
      const skipped = partition.skipped.filter((item) => item.agentId === agentId);
      const outcome = emptyPlacementOutcome();
      outcome.skipped.push(...skipped);
      if (match.length === 0) {
        if (skipped.length === 0) {
          outcome.skipped.push({
            skillName,
            agentId,
            reasonCode: SKILLS_CLI_SKIP_NO_PLACEMENT,
          });
        }
        throwIfSingleOutcomeFailed(outcome);
        return;
      }
      const batchOutcome = await runPlacementBatch(
        get,
        set,
        match,
        "unlink",
        force,
      );
      outcome.succeeded.push(...batchOutcome.succeeded);
      outcome.failed.push(...batchOutcome.failed);
      throwIfSingleOutcomeFailed(outcome);
    },

    async linkPlatformBatch(skillNames, agentId) {
      if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
        throw new Error(BUSY_ENVELOPE);
      }
      const partition = partitionLinkBatch(get().skills, skillNames, agentId);
      const outcome = emptyPlacementOutcome();
      outcome.skipped.push(...partition.skipped);
      if (partition.allowed.length === 0) {
        return outcome;
      }
      const ran = await runPlacementBatch(get, set, partition.allowed, "link");
      outcome.succeeded.push(...ran.succeeded);
      outcome.failed.push(...ran.failed);
      return outcome;
    },

    async unlinkManagedBatch(skillNames) {
      if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
        throw new Error(BUSY_ENVELOPE);
      }
      const partition = partitionUnlinkBatch(get().skills, skillNames);
      const outcome = emptyPlacementOutcome();
      outcome.skipped.push(...partition.skipped);
      if (partition.allowed.length === 0) {
        return outcome;
      }
      const ran = await runPlacementBatch(get, set, partition.allowed, "unlink");
      outcome.succeeded.push(...ran.succeeded);
      outcome.failed.push(...ran.failed);
      return outcome;
    },

    async unlinkPlatformBatch(skillNames, agentId) {
      if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
        throw new Error(BUSY_ENVELOPE);
      }
      const partition = partitionUnlinkBatchForAgent(
        get().skills,
        skillNames,
        agentId,
      );
      const outcome = emptyPlacementOutcome();
      outcome.skipped.push(...partition.skipped);
      if (partition.allowed.length === 0) {
        return outcome;
      }
      const ran = await runPlacementBatch(get, set, partition.allowed, "unlink");
      outcome.succeeded.push(...ran.succeeded);
      outcome.failed.push(...ran.failed);
      return outcome;
    },

    async removeGlobalBatch(skillNames, options) {
      if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
        throw new Error(BUSY_ENVELOPE);
      }
      const force = options?.force === true;
      const outcome = emptyPlacementOutcome();
      if (skillNames.length === 0) {
        return outcome;
      }
      set({ isMutating: true, actionError: null });
      beginBatchProgress(set, "cleanup", skillNames.length);
      try {
        for (const skillName of skillNames) {
          const jobId = newJobId();
          set({ jobId });
          const snapshot = get().skills.find((skill) => skill.name === skillName);
          if (snapshot) {
            set({ skills: omitSkill(get().skills, skillName) });
          }
          try {
            await invoke("skills_cli_remove_global", { jobId, skillName, force });
            outcome.succeeded.push({ skillName });
          } catch (error) {
            if (get().jobId === jobId && snapshot) {
              set({ skills: restoreSkill(get().skills, snapshot) });
            }
            if (get().jobId === jobId) {
              set({ actionError: backendErrorStateValue(error) });
            }
            outcome.failed.push({
              skillName,
              errorCode: errorCodeFrom(error),
            });
          }
          incrementBatchProgress(get, set);
        }
      } finally {
        clearBatchProgress(set);
        if (get().isMutating) {
          set({ isMutating: false, jobId: null });
        }
        await refreshInventoryAfterMutation(get, set);
      }
      return outcome;
    },

    async applyUpdatesBatch(skillNames) {
      if (batchAlreadyRunning(get())) {
        return emptyPlacementOutcome();
      }
      if (skillsCliOperationBusy(get())) {
        throw new Error(BUSY_ENVELOPE);
      }
      const outcome = emptyPlacementOutcome();
      const ordered = selectedSkillsInStoreOrder(
        get().skills,
        new Set(skillNames),
      ).map((skill) => skill.name);
      const inventory = get().updateInventory;
      const groups = groupSkillNamesByRepositoryKey(ordered, inventory)
        .map((group) => {
          const skillNames = group.skillNames.filter(
            (name) => applySelectionsForNames(inventory, [name]).length > 0,
          );
          return {
            repositoryKey: group.repositoryKey,
            skillNames,
            selections: applySelectionsForNames(inventory, skillNames),
          };
        })
        .filter((group) => group.selections.length > 0);
      if (groups.length === 0) {
        return outcome;
      }
      beginBatchProgress(set, "update", groups.length);
      try {
        for (const group of groups) {
          try {
            const result = await get().applyUpdates({
              repositoryKey: group.repositoryKey,
              skillNames: group.skillNames,
              selections: group.selections,
            });
            for (const skillName of result.appliedSkillNames) {
              outcome.succeeded.push({ skillName });
            }
          } catch (error) {
            const errorCode = errorCodeFrom(error);
            for (const skillName of group.skillNames) {
              outcome.failed.push({ skillName, errorCode });
            }
          }
          incrementBatchProgress(get, set);
        }
      } finally {
        clearBatchProgress(set);
      }
      return outcome;
    },
  };
}
