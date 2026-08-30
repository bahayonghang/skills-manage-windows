export interface SkillportStateExportedTarget {
  id: string;
  kind: string;
  label: string;
}

export type SkillportStateSourceStatus = "exists" | "will_add" | "duplicate";
export type SkillportStateSkillStatus =
  | "ready"
  | "conflict"
  | "missing"
  | "unrestorable"
  | "duplicate_skipped";
export type SkillportStateImportResolutionType = "overwrite" | "skip" | "rename";

export type SkillportStatePortabilityJobStatus =
  | "idle"
  | "running"
  | "completed"
  | "failed"
  | "cancelling"
  | "cancelled";

export type SkillportStatePortabilityPhase =
  | "exporting"
  | "previewing"
  | "importing"
  | "finalizing"
  | null;

export interface SkillportStatePortabilityJob {
  jobId: string | null;
  phase: SkillportStatePortabilityPhase;
  status: SkillportStatePortabilityJobStatus;
  total: number;
  completed: number;
  message?: string;
  currentItem?: string;
  error?: string;
}

export interface SkillportStatePortabilityProgressPayload {
  jobId: string;
  phase: Exclude<SkillportStatePortabilityPhase, null>;
  status: Exclude<SkillportStatePortabilityJobStatus, "idle" | "cancelling">;
  total: number;
  completed: number;
  message?: string | null;
  currentItem?: string | null;
  error?: string | null;
}

export interface SkillportStateImportPreviewSummary {
  sourcesToAdd: number;
  sourcesExisting: number;
  sourcesDuplicate?: number;
  ready: number;
  conflicts: number;
  missing: number;
  unrestorable: number;
  duplicateSkipped?: number;
}

export interface SkillportStateSourcePreview {
  name: string;
  url: string;
  status: SkillportStateSourceStatus;
}

export interface SkillportStateSkillPreview {
  id: string;
  name: string;
  sourcePath?: string | null;
  status: SkillportStateSkillStatus;
  existingSkillId?: string | null;
  reason?: string | null;
  detail?: string | null;
}

export interface SkillportStateImportPreview {
  githubSources: SkillportStateSourcePreview[];
  skills: SkillportStateSkillPreview[];
  summary: SkillportStateImportPreviewSummary;
  warnings: Array<{
    reason: string;
    detail: string;
    sourcePath?: string | null;
    repoUrl?: string | null;
  }>;
}

export interface SkillportStateImportResolution {
  skillId: string;
  sourcePath?: string | null;
  resolution: SkillportStateImportResolutionType;
  renamedSkillId?: string | null;
}

export interface SkillportStateImportedSkill {
  sourcePath: string;
  importedSkillId: string;
  skillName: string;
}

export interface SkillportStateImportFailure {
  skillId: string;
  sourcePath?: string | null;
  error: string;
}

export interface SkillportStateImportResult {
  sourcesAdded: number;
  sourcesSkipped: number;
  importedSkills: SkillportStateImportedSkill[];
  skippedSkills: string[];
  failedSkills: SkillportStateImportFailure[];
  tagsRestored: number;
  cancelled?: boolean;
}
