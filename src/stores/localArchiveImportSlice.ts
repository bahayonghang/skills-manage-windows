import { invoke, isTauriRuntime } from "@/lib/ipc";
import type {
  ArchiveFingerprint,
  LocalArchiveImportResolution,
  LocalArchiveImportResult,
  LocalArchivePreview,
} from "@/types";

/** State machine for the local archive import wizard. */
export type LocalArchiveImportStep =
  | "choose" // user is picking a file
  | "preview" // preview loaded, awaiting confirm
  | "importing" // import in flight
  | "result"; // import result (or error) shown

export interface LocalArchiveImportState {
  step: LocalArchiveImportStep;
  /** Absolute archive path on the local machine. Never returned to UI. */
  archivePath: string | null;
  preview: LocalArchivePreview | null;
  previewError: string | null;
  isPreviewLoading: boolean;
  importResult: LocalArchiveImportResult | null;
  importError: string | null;
  isImporting: boolean;
}

export function createInitialLocalArchiveImportState(): LocalArchiveImportState {
  return {
    step: "choose",
    archivePath: null,
    preview: null,
    previewError: null,
    isPreviewLoading: false,
    importResult: null,
    importError: null,
    isImporting: false,
  };
}

/**
 * Invoke the preview IPC command. Returns the preview DTO or throws on error.
 * The absolute `archivePath` is passed to the backend but never stored in the
 * preview DTO; the DTO only carries the archive basename.
 */
export async function previewLocalSkillArchive(
  archivePath: string,
): Promise<LocalArchivePreview> {
  if (!isTauriRuntime()) {
    throw new Error(
      "Desktop-only feature: local ZIP preview is available in the Tauri app.",
    );
  }
  return invoke<LocalArchivePreview>("preview_local_skill_archive", {
    archivePath,
  });
}

/**
 * Invoke the import IPC command. The caller must pass the `expectedFingerprint`
 * returned by preview so the backend can verify the archive is byte-identical.
 */
export async function importLocalSkillArchive(
  archivePath: string,
  expectedFingerprint: ArchiveFingerprint,
  resolution: LocalArchiveImportResolution,
  renamedSkillId?: string,
): Promise<LocalArchiveImportResult> {
  if (!isTauriRuntime()) {
    throw new Error(
      "Desktop-only feature: local ZIP import is available in the Tauri app.",
    );
  }
  const payload: {
    archivePath: string;
    expectedFingerprint: ArchiveFingerprint;
    resolution: LocalArchiveImportResolution;
    renamedSkillId?: string;
  } = {
    archivePath,
    expectedFingerprint,
    resolution,
  };
  if (renamedSkillId) {
    payload.renamedSkillId = renamedSkillId;
  }
  return invoke<LocalArchiveImportResult>(
    "import_local_skill_archive",
    payload,
  );
}
