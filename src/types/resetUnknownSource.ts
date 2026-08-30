import type { BatchDeleteCentralSkillPreviewResult } from "@/types";

export interface ResetUnknownSourceSkillsPreview {
  skillIds: string[];
  preview: BatchDeleteCentralSkillPreviewResult;
}
