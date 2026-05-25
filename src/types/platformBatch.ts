export interface BatchUninstallSkillRequest {
  skill_id: string;
  row_id?: string;
}

export interface BatchUninstallSkillSuccess {
  skill_id: string;
  row_id?: string;
}

export interface BatchUninstallSkillFailure {
  skill_id: string;
  row_id?: string;
  error: string;
}

export interface BatchUninstallSkillResult {
  succeeded: BatchUninstallSkillSuccess[];
  failed: BatchUninstallSkillFailure[];
}
