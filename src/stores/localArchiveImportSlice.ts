import type { TFunction } from "i18next";
import { create } from "zustand";

import { invoke } from "@/lib/ipc";
import { parseBackendError } from "@/lib/backendError";
import type {
  ArchiveFingerprint,
  LocalArchiveImportResolution,
  LocalArchiveImportResult,
  LocalArchivePreview,
} from "@/types";

export type LocalArchiveImportStep =
  | "choose"
  | "preview"
  | "importing"
  | "result";

export interface LocalArchiveImportData {
  isOpen: boolean;
  step: LocalArchiveImportStep;
  archivePath: string | null;
  preview: LocalArchivePreview | null;
  previewError: string | null;
  isPreviewLoading: boolean;
  importResult: LocalArchiveImportResult | null;
  importError: string | null;
  isImporting: boolean;
  resolution: LocalArchiveImportResolution;
  renamedSkillId: string;
}

export interface LocalArchiveImportStore extends LocalArchiveImportData {
  openWizard: () => void;
  closeWizard: () => void;
  previewArchive: (archivePath: string) => Promise<LocalArchivePreview>;
  reportPreviewFailure: (error: unknown) => void;
  importArchive: () => Promise<LocalArchiveImportResult>;
  setResolution: (resolution: LocalArchiveImportResolution) => void;
  setRenamedSkillId: (renamedSkillId: string) => void;
  reset: () => void;
}

export function createInitialLocalArchiveImportState(): LocalArchiveImportData {
  return {
    isOpen: false,
    step: "choose",
    archivePath: null,
    preview: null,
    previewError: null,
    isPreviewLoading: false,
    importResult: null,
    importError: null,
    isImporting: false,
    resolution: "overwrite",
    renamedSkillId: "",
  };
}

function localArchiveErrorCode(error: unknown): string {
  const code = parseBackendError(error).code;
  return code?.startsWith("local_archive.")
    ? code
    : "local_archive.unknown";
}

export function formatLocalArchiveError(
  errorCode: string | null,
  t: TFunction,
): string | null {
  if (!errorCode) return null;
  const fallback = t("backendErrors.local_archive.unknown");
  return t(`backendErrors.${errorCode}`, { defaultValue: fallback });
}

async function previewLocalSkillArchive(
  archivePath: string,
): Promise<LocalArchivePreview> {
  return invoke("preview_local_skill_archive", { archivePath });
}

async function importLocalSkillArchive(
  archivePath: string,
  expectedFingerprint: ArchiveFingerprint,
  resolution: LocalArchiveImportResolution,
  renamedSkillId?: string,
): Promise<LocalArchiveImportResult> {
  return invoke("import_local_skill_archive", {
    archivePath,
    expectedFingerprint,
    resolution,
    renamedSkillId,
  });
}

export const useLocalArchiveImportStore = create<LocalArchiveImportStore>()(
  (set, get) => ({
    ...createInitialLocalArchiveImportState(),

    openWizard: () => set({ ...createInitialLocalArchiveImportState(), isOpen: true }),

    closeWizard: () => set(createInitialLocalArchiveImportState()),

    reset: () => set(createInitialLocalArchiveImportState()),

    setResolution: (resolution) =>
      set({
        resolution,
        importError: null,
        ...(resolution === "rename" ? {} : { renamedSkillId: "" }),
      }),

    setRenamedSkillId: (renamedSkillId) => set({ renamedSkillId, importError: null }),

    reportPreviewFailure: (error) =>
      set({
        previewError: localArchiveErrorCode(error),
        isPreviewLoading: false,
        step: "choose",
      }),

    previewArchive: async (archivePath) => {
      set({
        archivePath,
        preview: null,
        previewError: null,
        importResult: null,
        importError: null,
        isPreviewLoading: true,
      });
      try {
        const preview = await previewLocalSkillArchive(archivePath);
        set({ preview, step: "preview", resolution: "overwrite" });
        return preview;
      } catch (error) {
        set({ previewError: localArchiveErrorCode(error), step: "choose" });
        throw error;
      } finally {
        set({ isPreviewLoading: false });
      }
    },

    importArchive: async () => {
      const { archivePath, preview, resolution, renamedSkillId } = get();
      if (!archivePath || !preview) {
        const error = new Error("local_archive.unknown:Import preview is missing.");
        set({ importError: "local_archive.unknown" });
        throw error;
      }

      set({ isImporting: true, importError: null, step: "importing" });
      try {
        const result = await importLocalSkillArchive(
          archivePath,
          preview.fingerprint,
          resolution,
          resolution === "rename" ? renamedSkillId.trim() || undefined : undefined,
        );
        set({ importResult: result, step: "result" });
        return result;
      } catch (error) {
        set({ importError: localArchiveErrorCode(error), step: "preview" });
        throw error;
      } finally {
        set({ isImporting: false });
      }
    },
  }),
);
