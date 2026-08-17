import type {
  SkillInstallation,
  SkillRepository,
  SkillRepositoryWithStats,
} from "./index";

export interface PendingDeleteRecoveryPreview {
  operation_id: string;
  operation_kind: string;
  phase: string;
  error_code?: string | null;
  force_delete_eligible: boolean;
  blocker_codes: string[];
}

export interface DeleteCentralSkillPreview {
  skill_id: string;
  skill_name: string;
  central_path: string;
  copy_installations: SkillInstallation[];
  auto_removed_agent_ids: string[];
  pending_recovery?: PendingDeleteRecoveryPreview | null;
}

export interface FailedCentralSkillDelete {
  skill_id: string;
  phase?: string | null;
  error_code?: string | null;
  error_category?: string | null;
  error: string;
}

export interface BatchDeleteCentralSkillPreviewResult {
  previews: DeleteCentralSkillPreview[];
  failed: FailedCentralSkillDelete[];
}

export interface BatchDeleteCentralSkillRequest {
  skill_id: string;
  remove_agent_ids: string[];
  force?: boolean;
}

export interface BatchDeleteCentralSkillSuccess {
  skill_id: string;
  removed_central_path: string;
  removed_agent_ids: string[];
  retained_agent_ids: string[];
}

export interface BatchDeleteCentralSkillResult {
  succeeded: BatchDeleteCentralSkillSuccess[];
  failed: FailedCentralSkillDelete[];
}

export interface DeleteSkillRepositoryPreview {
  repository: SkillRepositoryWithStats;
  delete_preview: BatchDeleteCentralSkillPreviewResult;
}

export interface DeleteSkillRepositoryResult {
  repository: SkillRepository;
  deleted_repository: boolean;
  delete_result: BatchDeleteCentralSkillResult;
}
