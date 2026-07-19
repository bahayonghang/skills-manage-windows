// Local skill archive (ZIP) import types.
//
// These mirror the Rust DTOs in `src-tauri/src/services/local_archive_import/types.rs`.
// Preview and result types never expose absolute user-directory paths; only
// the archive basename, post-strip relative paths, the fingerprint, and the
// conflict summary cross the IPC boundary.

/** Archive fingerprint (SHA-256 + byte length) returned by preview. */
export interface ArchiveFingerprint {
  /** Lowercase hex SHA-256 of the archive bytes. */
  sha256: string;
  /** Total archive file size (ZIP byte length on disk). */
  byteLen: number;
}

/** Conflict resolution strategy (mirrors the GitHub import vocabulary). */
export type LocalArchiveImportResolution = "overwrite" | "skip" | "rename";

/** A regular file in the preview tree. */
export interface LocalArchivePreviewFile {
  /** Relative path (post-strip, forward-slash). */
  path: string;
  byteLen: number;
}

/** Central conflict info for a preview candidate. */
export interface LocalSkillConflict {
  existingSkillId: string;
  existingName: string;
  existingCanonicalPath: string | null;
  proposedSkillId: string;
  proposedName: string;
}

/** A discovered skill candidate inside the archive. */
export interface LocalArchivePreviewSkill {
  /** Archive-relative root directory (empty = archive root). */
  rootDirectory: string;
  skillId: string;
  skillName: string;
  description: string | null;
  /** Relative path to `SKILL.md` after wrapper stripping. */
  skillMdPath: string;
  files: LocalArchivePreviewFile[];
  fileCount: number;
  totalExpandedBytes: number;
  conflict: LocalSkillConflict | null;
}

/** Read-only preview DTO returned by `preview_local_skill_archive`. */
export interface LocalArchivePreview {
  /** Archive basename only; never the absolute user path. */
  archiveDisplayName: string;
  fingerprint: ArchiveFingerprint;
  skills: LocalArchivePreviewSkill[];
  totalFiles: number;
  totalExpandedBytes: number;
  totalCompressedBytes: number;
  archiveByteLen: number;
}

/** Result of a successful local archive import. */
export interface LocalArchiveImportResult {
  importedSkillId: string;
  skillName: string;
  rootDirectory: string;
  resolution: LocalArchiveImportResolution;
  fileCount: number;
  totalExpandedBytes: number;
  replacedExisting: boolean;
}
