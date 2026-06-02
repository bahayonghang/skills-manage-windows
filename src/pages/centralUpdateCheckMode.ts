import type { CentralSkillsCheckButtonState } from "@/pages/centralSkillsCheckButton";
import type { UpdateCenterTab } from "@/stores/updateCenterStore";
import type { SkillRepositoryWithStats } from "@/types";
import type {
  SkillRefreshContext,
  SkillRefreshScope,
  SkillUpdateInventory,
} from "@/types/skillUpdateInventory";

export type UpdateCheckMode = "regular" | "sync";

export const CENTRAL_UPDATE_CHECK_MODE_SETTING_KEY = "central_update_check_mode_v1";
export const DEFAULT_UPDATE_CHECK_MODE: UpdateCheckMode = "regular";

export function normalizeUpdateCheckMode(value: unknown): UpdateCheckMode {
  return value === "sync" ? "sync" : DEFAULT_UPDATE_CHECK_MODE;
}

export function hasSyncableGitHubRepository(
  repositories: readonly SkillRepositoryWithStats[],
): boolean {
  return repositories.some((repo) => repo.source_type === "github" && !repo.is_unknown);
}

export function buildUpdateCheckScope(
  mode: UpdateCheckMode,
  state: CentralSkillsCheckButtonState,
): SkillRefreshScope {
  if (mode === "regular") {
    return { kind: "skills", mode, skillIds: state.targetSkillIds };
  }
  if (state.scope === "current-results" && state.repositoryIds?.length === 1) {
    return { kind: "repositories", mode, repositoryIds: state.repositoryIds };
  }
  return { kind: "all", mode };
}

export function buildUpdateCheckRefreshContext(
  scope: SkillRefreshScope,
  state: CentralSkillsCheckButtonState,
): Partial<SkillRefreshContext> & Pick<SkillRefreshScope, "mode"> {
  const mode = scope.mode;
  if (scope.kind === "repositories") {
    return { mode, repositoryIds: scope.repositoryIds ?? [], skillIds: state.targetSkillIds };
  }
  if (scope.kind === "skills") {
    return { mode, skillIds: scope.skillIds ?? [] };
  }
  return { mode };
}

export function preferredUpdateCenterTab(
  inventory: SkillUpdateInventory | null | undefined,
): UpdateCenterTab {
  if (!inventory) return "updatable";
  if (inventory.updatable.length > 0) return "updatable";
  if (inventory.remoteAdded.length > 0) return "added";
  if (inventory.remoteMissing.length > 0) return "missing";
  if (inventory.failedRepositories.length > 0) return "failed";
  if (inventory.platformDuplicates.length > 0) return "duplicates";
  if ((inventory.deletedPlatformCopies ?? []).length > 0) return "deletedPlatformCopies";
  if (inventory.orphans.length > 0) return "orphans";
  return "updatable";
}
