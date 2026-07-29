export type StartupIssue =
  | "data_directory_unavailable"
  | "database_open_failed"
  | "schema_initialization_failed"
  | "database_recovery_failed"
  | "startup_state_unavailable";

export type StartupDiagnostic =
  | "not_run"
  | "healthy"
  | "corrupt"
  | "unavailable";

export type StartupStatus =
  | { phase: "checking" }
  | { phase: "ready" }
  | {
      phase: "recovery_required";
      issue: StartupIssue;
      diagnostic: StartupDiagnostic;
      canRebuild: boolean;
      backupCreated: boolean;
    }
  | { phase: "fatal"; issue: StartupIssue };
