export interface GitHubRepoRef {
  owner: string;
  repo: string;
  branch: string;
  normalizedUrl: string;
}

export interface GitHubSkillConflict {
  existingSkillId: string;
  existingName: string;
  existingCanonicalPath?: string | null;
  proposedSkillId: string;
  proposedName: string;
}

export interface GitHubSkillPreviewFile {
  path: string;
  byteLen: number;
  /** `sha256-v1:<hex>` digest of the previewed file bytes. */
  sha256: string;
}

export interface GitHubSkillPreview {
  sourcePath: string;
  skillId: string;
  skillName: string;
  description?: string | null;
  pluginName?: string | null;
  rootDirectory: string;
  skillDirectoryName: string;
  downloadUrl: string;
  conflict?: GitHubSkillConflict | null;
  files?: GitHubSkillPreviewFile[] | null;
}

/**
 * Immutable preview snapshot handle.
 *
 * `previewId` is an opaque backend token. Every subsequent Markdown read and
 * the confirmed import must send it; the backend resolves content only from the
 * registered snapshot and never re-resolves the branch.
 */
export interface GitHubRepoPreview {
  repo: GitHubRepoRef;
  skills: GitHubSkillPreview[];
  previewId: string;
  resolvedCommitSha: string;
  snapshotDigest: string;
  /** RFC 3339 timestamp after which the snapshot must be previewed again. */
  expiresAt: string;
}

export type DuplicateResolution = "overwrite" | "skip" | "rename";

export interface GitHubSkillImportSelection {
  sourcePath: string;
  resolution: DuplicateResolution;
  renamedSkillId?: string | null;
}

export interface ImportedGitHubSkillSummary {
  sourcePath: string;
  originalSkillId: string;
  importedSkillId: string;
  skillName: string;
  targetDirectory: string;
  resolution: DuplicateResolution;
}

export interface GitHubRepoImportResult {
  repo: GitHubRepoRef;
  importedSkills: ImportedGitHubSkillSummary[];
  skippedSkills: string[];
}

export type GitHubImportProgressPhase = "preparing" | "writing" | "finalizing";

export interface GitHubImportProgressPayload {
  phase: GitHubImportProgressPhase;
  currentSkill?: string | null;
  currentPath?: string | null;
  completedFiles: number;
  totalFiles: number;
  completedBytes: number;
  totalBytes: number;
}
