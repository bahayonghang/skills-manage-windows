export type LocalRemoteSyncItemStatus = "add" | "update" | "skip" | "error";
export type LocalRemoteSyncItemKind = "repo" | "skill";

export interface LocalRemoteSyncPreviewRequest {
  targetId: string;
  repoPath?: string | null;
}

export interface LocalRemoteSyncApplyRequest {
  targetId: string;
  repoPath?: string | null;
}

export interface LocalRemoteSyncItemPreview {
  id: string;
  label: string;
  kind: LocalRemoteSyncItemKind;
  localPath: string;
  remotePath: string;
  fileCount: number;
  byteCount: number;
  localHash: string;
  remoteHash?: string | null;
  status: LocalRemoteSyncItemStatus;
  error?: string | null;
}

export interface LocalRemoteSyncPreview {
  targetId: string;
  targetLabel: string;
  repoRoot: string;
  skillsRoot: string;
  repo: LocalRemoteSyncItemPreview;
  skills: LocalRemoteSyncItemPreview[];
  totalFileCount: number;
  totalByteCount: number;
}

export interface LocalRemoteSyncFailure {
  id: string;
  label: string;
  targetPath: string;
  error: string;
}

export interface LocalRemoteSyncApplyResult {
  targetId: string;
  targetLabel: string;
  syncedRepo?: LocalRemoteSyncItemPreview | null;
  syncedSkills: LocalRemoteSyncItemPreview[];
  skippedSkills: LocalRemoteSyncItemPreview[];
  failed: LocalRemoteSyncFailure[];
}
