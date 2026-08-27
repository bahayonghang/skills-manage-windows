export interface SkillsCliDoctorReport {
  nodeVersion: string;
  npmSpec: string;
}

export type {
  SkillsCliGlobalSkill,
  SkillsCliGlobalSnapshot,
  SkillsCliInstallKind,
  SkillsCliSourceTypeBucket,
  SkillsCliPlacement,
  SkillsCliPlacementState,
  SkillsCliManagedLinkKind,
  SkillsCliSkillDoc,
  SkillsCliRemovePlan,
  SkillsCliRemoveResult,
  SkillsCliRemovePlacementSummary,
  SkillsCliPlacementConflict,
} from "@/lib/ipc/generatedCommandMap";

export interface SkillsCliInstallTarget {
  id: string;
  displayName: string;
  iconName: string | null;
  cliAgent: string;
  isEnabled: boolean;
  defaultSelected: boolean;
}

export interface SkillsCliSourcePreview {
  source: string;
  skills: string[];
}

export interface SkillsCliAddResult {
  installedSkills: number;
  targetedPlatforms: number;
}

export type SkillsCliUpdateStatus =
  | "not_checked"
  | "checking"
  | "current"
  | "update_available"
  | "local_modified"
  | "baseline_required"
  | "unsupported"
  | "rate_limited"
  | "failed";

export type SkillsCliUpdateCapabilitySupport =
  | "verified_supported"
  | "verified_unsupported"
  | "unverified";

export interface SkillsCliUpdateCapabilityPlan {
  npmSpec: string;
  forceFlag: SkillsCliUpdateCapabilitySupport;
  keepLinksFlag: SkillsCliUpdateCapabilitySupport;
  pinnedFullShaSource: SkillsCliUpdateCapabilitySupport;
  directCopyRefresh: SkillsCliUpdateCapabilitySupport;
  applyMethod: string;
}

export interface SkillsCliUpdateBlocker {
  code: string;
  skillName: string;
}

export interface SkillsCliUpdateSkillRow {
  skillName: string;
  repositoryKey: string | null;
  normalizedSource: string | null;
  skillPath: string | null;
  status: SkillsCliUpdateStatus;
  installedRevisionSha: string | null;
  observedRevisionSha: string | null;
  pendingRevisionSha: string | null;
  installedLocalDigest: string | null;
  observedUpstreamDigest: string | null;
  pendingUpstreamDigest: string | null;
  isStale: boolean;
  lastErrorCode: string | null;
  changeSummary: string[];
  blockers: SkillsCliUpdateBlocker[];
  argvPreview: string[];
}

export interface SkillsCliUpdateRepositoryRow {
  repositoryKey: string;
  normalizedSource: string;
  branch: string;
  observedRevisionSha: string | null;
  status: string;
  lastCheckedAt: string | null;
  lastErrorCode: string | null;
  rateLimitResetAt: string | null;
  pendingCount: number;
}

export interface SkillsCliPendingRecovery {
  operationId: string;
  phase: string;
  lastErrorCode: string | null;
}

export interface SkillsCliUpdateInventory {
  skills: SkillsCliUpdateSkillRow[];
  repositories: SkillsCliUpdateRepositoryRow[];
  lastSuccessAt: string | null;
  pendingRecovery: SkillsCliPendingRecovery | null;
  capability: SkillsCliUpdateCapabilityPlan;
}

export interface SkillsCliUpdateProgress {
  jobId: string;
  phase: string;
  repositoryTotal: number;
  repositoryCompleted: number;
  currentRepositoryKey: string | null;
  selectedTotal: number;
  selectedCompleted: number;
  terminalStatus: string | null;
}

export interface SkillsCliApplySelection {
  skillName: string;
  skillPath: string;
  expectedInstalledRevision: string | null;
  expectedInstalledLocalDigest: string | null;
  expectedPendingRevision: string;
  expectedPendingDigest: string;
}

export interface SkillsCliApplyUpdateRequest {
  jobId: string;
  repositoryKey: string;
  selections: SkillsCliApplySelection[];
}

export interface SkillsCliApplyResult {
  appliedSkillNames: string[];
  installedRevisionSha: string;
}

export interface SkillsCliApplyRecoveryResult {
  operationId: string;
  phase: string;
}

export type SkillsCliUpdateJobPhase =
  | "checking"
  | "verifying"
  | "applying"
  | "recovering"
  | null;

export const EMPTY_SKILLS_CLI_UPDATE_INVENTORY: SkillsCliUpdateInventory = {
  skills: [],
  repositories: [],
  lastSuccessAt: null,
  pendingRecovery: null,
  capability: {
    npmSpec: "skills@1.5.23",
    forceFlag: "verified_unsupported",
    keepLinksFlag: "verified_unsupported",
    pinnedFullShaSource: "unverified",
    directCopyRefresh: "unverified",
    applyMethod: "pinned_snapshot_canonical_refresh",
  },
};
