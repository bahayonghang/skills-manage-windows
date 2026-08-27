// Generated from Rust/Serde metadata by `pnpm ipc:codegen`. Do not edit.
// This artifact contains contract metadata only and never invokes Tauri.

type GeneratedIpcCommandSpec<Args, Result> = { args: Args; result: Result };
const command = <Args, Result>() => ({}) as GeneratedIpcCommandSpec<Args, Result>;

export const GENERATED_IPC_COMMANDS = {
  apply_central_repository_sync: command<{ decisions: CentralRepositorySyncDecisions }, CentralRepositorySyncApplyResult_Serialize>(),
  apply_central_store_location_change: command<{ request: CentralStoreLocationApplyRequest }, CentralStoreLocationChangeResult>(),
  apply_local_remote_sync: command<{ request: LocalRemoteSyncApplyRequest }, LocalRemoteSyncApplyResult_Serialize>(),
  batch_install_central_skills: command<{ skillIds: string[]; agentIds: string[]; method: string | null; projectPath: string | null }, CentralBatchInstallResult>(),
  batch_install_collection: command<{ collectionId: string; agentIds: string[] }, BatchInstallResult>(),
  batch_install_to_agents: command<{ skillId: string; agentIds: string[]; method: string | null }, BatchInstallResult>(),
  cancel_skills_cli_job: command<{ jobId: string }, boolean>(),
  clear_ai_api_key: command<{ provider: string | null }, AiApiKeyState_Serialize>(),
  clear_github_pat: command<undefined, GitHubPatState_Serialize>(),
  clear_skill_update_inventory: command<{ scope: {
	kind: SkillRefreshScopeKind,
	mode?: SkillRefreshMode | null,
	cachePolicy?: SkillRefreshCachePolicy | null,
	skillIds?: string[] | null,
	repositoryIds?: string[] | null,
	agentIds?: string[] | null,
} | null }, null>(),
  delete_central_skill: command<{ skillId: string; removeAgentIds: string[]; force: boolean | null }, DeleteCentralSkillResult>(),
  delete_central_skills: command<{ requests: BatchDeleteCentralSkillRequest[] }, BatchDeleteCentralSkillResult_Serialize>(),
  delete_collection: command<{ collectionId: string }, null>(),
  delete_skill_repository: command<{ repositoryId: string; requests: BatchDeleteCentralSkillRequest[] }, DeleteSkillRepositoryResult_Serialize>(),
  force_mirror_central_repositories: command<{ request: ForceRepositoryMirrorRequest }, ForceRepositoryMirrorResult_Serialize>(),
  force_update_central_skills: command<{ request: ForceSkillUpdateRequest }, ForceSkillUpdateResult>(),
  get_ai_api_key_state: command<{ provider: string | null }, AiApiKeyState_Serialize>(),
  get_central_skill_update_states: command<undefined, SkillUpdateState[]>(),
  get_github_pat: command<undefined, GitHubPatState_Serialize>(),
  get_skill_update_inventory: command<{ scope: {
	kind: SkillRefreshScopeKind,
	mode?: SkillRefreshMode | null,
	cachePolicy?: SkillRefreshCachePolicy | null,
	skillIds?: string[] | null,
	repositoryIds?: string[] | null,
	agentIds?: string[] | null,
} | null }, SkillUpdateInventory_Serialize>(),
  import_collection: command<{ json: string }, Collection>(),
  import_obsidian_skill_to_central: command<{ dirPath: string }, ObsidianImportResult>(),
  import_obsidian_skill_to_platform: command<{ dirPath: string; agentId: string; method: string | null }, ObsidianImportResult>(),
  install_from_skills_sh: command<{ source: string; skillId: string }, string>(),
  install_marketplace_skill: command<{ skillId: string }, null>(),
  install_skill_to_agent: command<{ skillId: string; agentId: string; method: string | null }, InstallResult>(),
  install_skill_to_project: command<{ projectId: string; skillId: string; agentId: string; method: string }, ProjectSkillInstallation>(),
  keep_remote_missing_central_skills: command<{ skillIds: string[] }, string[]>(),
  preview_central_store_location_change: command<{ request: CentralStoreLocationPreviewRequest }, CentralStoreLocationPreview>(),
  preview_local_remote_sync: command<{ request: LocalRemoteSyncPreviewRequest }, LocalRemoteSyncPreview_Serialize>(),
  refresh_skill_update_inventory: command<{ scope: SkillRefreshScope; operationId: string }, SkillUpdateInventory_Serialize>(),
  remove_project: command<{ id: string; uninstallSkills: boolean }, null>(),
  remove_registry: command<{ registryId: string }, null>(),
  remove_scan_directory: command<{ path: string }, null>(),
  remove_skill_from_collection: command<{ collectionId: string; skillId: string }, null>(),
  retry_failed_update_repositories: command<{ scope: SkillRefreshScope; repositoryIds: string[]; modeOverride: "regular" | "sync" | null; operationId: string }, SkillUpdateInventory_Serialize>(),
  scan_deleted_platform_copies: command<{ agentIds: string[] | null }, DeletedPlatformCopyGroup[]>(),
  scan_platform_duplicate_skills: command<{ agentIds: string[] | null }, PlatformDuplicateGroup[]>(),
  set_ai_api_key: command<{ value: string; provider: string | null }, AiApiKeyState_Serialize>(),
  set_github_pat: command<{ value: string }, GitHubPatState_Serialize>(),
  skills_cli_add_global: command<{ jobId: string; source: string; skillNames: string[]; skillportAgentIds: string[] }, SkillsCliAddResult>(),
  skills_cli_apply_updates: command<{ request: SkillsCliApplyUpdateRequest }, SkillsCliApplyResult>(),
  skills_cli_check_updates: command<{ jobId: string }, SkillsCliUpdateInventory>(),
  skills_cli_doctor: command<undefined, SkillsCliDoctorReport>(),
  skills_cli_export_inventory: command<{ path: string; json: string }, null>(),
  skills_cli_install_targets: command<undefined, SkillsCliInstallTarget[]>(),
  skills_cli_link_platform: command<{ jobId: string; skillName: string; skillportAgentId: string }, SkillsCliPlacement>(),
  skills_cli_list_global: command<undefined, SkillsCliGlobalSnapshot>(),
  skills_cli_preview_remove_global: command<{ skillName: string }, SkillsCliRemovePlan>(),
  skills_cli_preview_source: command<{ source: string }, SkillsCliSourcePreview>(),
  skills_cli_read_skill_md: command<{ skillName: string }, SkillsCliSkillDoc>(),
  skills_cli_remove_global: command<{ jobId: string; skillName: string }, SkillsCliRemoveResult>(),
  skills_cli_retry_update_recovery: command<{ jobId: string; operationId: string }, SkillsCliApplyRecoveryResult>(),
  skills_cli_reveal_skill_folder: command<{ skillName: string }, null>(),
  skills_cli_unlink_platform: command<{ jobId: string; skillName: string; skillportAgentId: string }, SkillsCliPlacement>(),
  skills_cli_update_inventory: command<undefined, SkillsCliUpdateInventory>(),
  skills_cli_verify_update_baseline: command<{ jobId: string; skillNames: string[] }, SkillsCliUpdateInventory>(),
  test_ai_connection: command<undefined, AiConnectionTestResult_Serialize>(),
  test_github_pat: command<undefined, GitHubPatTestResult>(),
  unassign_skill_tags: command<{ skillId: string; tagIds: string[] }, null>(),
  uninstall_skill_from_project: command<{ projectId: string; skillId: string; agentId: string }, null>(),
} as const;

export const GENERATED_IPC_COMMAND_NAMES = [
  "apply_central_repository_sync",
  "apply_central_store_location_change",
  "apply_local_remote_sync",
  "batch_install_central_skills",
  "batch_install_collection",
  "batch_install_to_agents",
  "cancel_skills_cli_job",
  "clear_ai_api_key",
  "clear_github_pat",
  "clear_skill_update_inventory",
  "delete_central_skill",
  "delete_central_skills",
  "delete_collection",
  "delete_skill_repository",
  "force_mirror_central_repositories",
  "force_update_central_skills",
  "get_ai_api_key_state",
  "get_central_skill_update_states",
  "get_github_pat",
  "get_skill_update_inventory",
  "import_collection",
  "import_obsidian_skill_to_central",
  "import_obsidian_skill_to_platform",
  "install_from_skills_sh",
  "install_marketplace_skill",
  "install_skill_to_agent",
  "install_skill_to_project",
  "keep_remote_missing_central_skills",
  "preview_central_store_location_change",
  "preview_local_remote_sync",
  "refresh_skill_update_inventory",
  "remove_project",
  "remove_registry",
  "remove_scan_directory",
  "remove_skill_from_collection",
  "retry_failed_update_repositories",
  "scan_deleted_platform_copies",
  "scan_platform_duplicate_skills",
  "set_ai_api_key",
  "set_github_pat",
  "skills_cli_add_global",
  "skills_cli_apply_updates",
  "skills_cli_check_updates",
  "skills_cli_doctor",
  "skills_cli_export_inventory",
  "skills_cli_install_targets",
  "skills_cli_link_platform",
  "skills_cli_list_global",
  "skills_cli_preview_remove_global",
  "skills_cli_preview_source",
  "skills_cli_read_skill_md",
  "skills_cli_remove_global",
  "skills_cli_retry_update_recovery",
  "skills_cli_reveal_skill_folder",
  "skills_cli_unlink_platform",
  "skills_cli_update_inventory",
  "skills_cli_verify_update_baseline",
  "test_ai_connection",
  "test_github_pat",
  "unassign_skill_tags",
  "uninstall_skill_from_project",
] as const;

export const GENERATED_REVIEWED_IPC_ERROR_CODES = [
  "ai.client_build_failed",
  "ai.connect",
  "ai.dns",
  "ai.empty_response",
  "ai.invalid_api_key",
  "ai.missing_api_key",
  "ai.network",
  "ai.proxy",
  "ai.rate_limit",
  "ai.request_failed",
  "ai.response_error",
  "ai.response_parse_failed",
  "ai.response_read_failed",
  "ai.timeout",
  "ai.tls",
  "central.reset_failed",
  "central_operation.delete_restore_collision",
  "central_skills.budget_exceeded",
  "central_skills.database_failed",
  "central_skills.delete_failed",
  "central_skills.delete_preview_failed",
  "central_skills.force_delete_blocked",
  "central_skills.mutation_lock_failed",
  "central_skills.remote_failed",
  "central_updates.inventory_invariant",
  "central_updates.inventory_refresh_required",
  "central_updates.relocation_failed",
  "central_updates.repository_check_failed",
  "central_updates.skill_source_missing",
  "central_updates.snapshot_changed",
  "credential.ssh_password_unavailable",
  "github_import.access_denied",
  "github_import.archive_redirect_rejected",
  "github_import.archive_unavailable",
  "github_import.branch_conflict",
  "github_import.branch_invalid",
  "github_import.budget_exceeded",
  "github_import.configured_token_failed",
  "github_import.credential_unavailable",
  "github_import.duplicate_selection",
  "github_import.invalid_candidate",
  "github_import.invalid_url",
  "github_import.no_importable_skills",
  "github_import.preview_busy",
  "github_import.preview_capacity",
  "github_import.preview_cleanup_pending",
  "github_import.preview_commit_unresolved",
  "github_import.preview_expired",
  "github_import.preview_integrity",
  "github_import.preview_mismatch",
  "github_import.preview_missing",
  "github_import.rate_limited",
  "github_import.rename_conflict",
  "github_import.repo_not_found",
  "github_import.response_invalid",
  "github_import.selection_unavailable",
  "github_import.source_path_missing",
  "github_import.target_exists",
  "github_import.transport_failed",
  "input.invalid",
  "installation.pending_central_recovery",
  "internal.unexpected",
  "job.central_update_busy",
  "job.id_mismatch",
  "job.invalid_id",
  "job.portability_busy",
  "job.registry_unavailable",
  "local_archive.ambiguous_archive_layout",
  "local_archive.archive_changed_since_preview",
  "local_archive.archive_not_found",
  "local_archive.archive_read_failed",
  "local_archive.budget_exceeded",
  "local_archive.central_mutation",
  "local_archive.db",
  "local_archive.internal",
  "local_archive.invalid_archive_entry",
  "local_archive.invalid_skill_identifier",
  "local_archive.io",
  "local_archive.no_skill_manifest",
  "local_archive.path_conflict",
  "local_archive.remote_target_unsupported",
  "local_archive.rollback_failed",
  "local_archive.skill_frontmatter_missing",
  "local_archive.unsupported_archive_entry",
  "marketplace.identity_ambiguous",
  "marketplace.install_failed",
  "marketplace.install_unavailable",
  "marketplace.registry_disabled",
  "marketplace.registry_stale",
  "marketplace.source_unsupported",
  "operation.cancelled",
  "permission.denied",
  "portable_state.invalid_manifest_json",
  "portable_state.unsupported_export_kind",
  "portable_state.unsupported_export_version",
  "recovery.reconcile_guard_unavailable",
  "recovery.reconcile_preflight_blocked",
  "resource.conflict",
  "resource.not_found",
  "runtime.desktop_required",
  "skills_cli.agent_unmapped",
  "skills_cli.busy",
  "skills_cli.cancelled",
  "skills_cli.canonical_missing",
  "skills_cli.cli_failed",
  "skills_cli.cli_unavailable",
  "skills_cli.direct_copy_not_toggleable",
  "skills_cli.export_failed",
  "skills_cli.export_invalid",
  "skills_cli.local_target_only",
  "skills_cli.node_missing",
  "skills_cli.placement_conflict",
  "skills_cli.placement_unavailable",
  "skills_cli.preview_unparsed",
  "skills_cli.recovery_required",
  "skills_cli.remote_unavailable",
  "skills_cli.reveal_failed",
  "skills_cli.selection_empty",
  "skills_cli.skill_doc_invalid_utf8",
  "skills_cli.skill_doc_missing",
  "skills_cli.skill_doc_too_large",
  "skills_cli.skill_not_owned",
  "skills_cli.source_invalid",
  "skills_cli.timeout",
  "skills_cli.update_baseline_required",
  "skills_cli.update_check_failed",
  "skills_cli.update_integrity",
  "skills_cli.update_local_modified",
  "skills_cli.update_migration",
  "skills_cli.update_rate_limited",
  "skills_cli.update_recovery_required",
  "skills_cli.update_stale",
  "skills_cli.update_topology_conflict",
  "skills_cli.update_unsupported",
  "startup.rebuild_unavailable",
  "storage.unavailable",
] as const;

export type AiApiKeyState = AiApiKeyState_Serialize | AiApiKeyState_Deserialize;

export type AiApiKeyState_Deserialize = {
	provider: string,
	configured: boolean,
	storageState: SecretStorageState,
	fingerprint: string | null,
	error: string | null,
};

export type AiApiKeyState_Serialize = {
	provider: string,
	configured: boolean,
	storageState: SecretStorageState,
	fingerprint?: string | null,
	error?: string | null,
};

export type AiConnectionTestResult = AiConnectionTestResult_Serialize | AiConnectionTestResult_Deserialize;

export type AiConnectionTestResult_Deserialize = {
	ok: boolean,
	msg: string,
	code: string | null,
	details: string | null,
};

export type AiConnectionTestResult_Serialize = {
	ok: boolean,
	msg: string,
	code?: string | null,
	details?: string | null,
};

export type BatchDeleteCentralSkillRequest = {
	skill_id: string,
	remove_agent_ids: string[],
	force?: boolean,
};

export type BatchDeleteCentralSkillResult = BatchDeleteCentralSkillResult_Serialize | BatchDeleteCentralSkillResult_Deserialize;

export type BatchDeleteCentralSkillResult_Deserialize = {
	succeeded: BatchDeleteCentralSkillSuccess[],
	failed: FailedCentralSkillDelete_Deserialize[],
};

export type BatchDeleteCentralSkillResult_Serialize = {
	succeeded: BatchDeleteCentralSkillSuccess[],
	failed: FailedCentralSkillDelete_Serialize[],
};

export type BatchDeleteCentralSkillSuccess = {
	skill_id: string,
	removed_central_path: string,
	removed_agent_ids: string[],
	retained_agent_ids: string[],
};

/**  Result of a batch install across multiple agents. */
export type BatchInstallResult = {
	succeeded: string[],
	skipped: SkippedInstall[],
	failed: FailedInstall[],
};

/**  Probe ledger status. Unsupported and unverified both fail closed. */
export type CapabilitySupport = "verified_supported" | "verified_unsupported" | "unverified";

/**  Failed item from a Central batch install request. */
export type CentralBatchInstallFailure = {
	skill_id: string,
	agent_id: string,
	error: string,
};

/**  Result of installing multiple Central skills to multiple targets. */
export type CentralBatchInstallResult = {
	succeeded: CentralBatchInstallSuccess[],
	skipped: CentralBatchInstallSkipped[],
	failed: CentralBatchInstallFailure[],
};

/**  Skipped item from a Central batch install request. */
export type CentralBatchInstallSkipped = {
	skill_id: string,
	agent_id: string,
	target_path: string,
	reason: string,
};

/**  Successful item from a Central batch install request. */
export type CentralBatchInstallSuccess = {
	skill_id: string,
	agent_id: string,
	target_path: string,
};

/**
 *  Central repository sync confirms additions through its own verified
 *  inventory snapshot, not through a renderer preview token. It therefore
 *  carries no `previewId` and must never fabricate one.
 */
export type CentralRepositoryAddedSkillSelection = {
	repositoryId: string,
	selections: GitHubSkillImportSelection[],
};

export type CentralRepositoryAdditionSkipRequest = {
	repositoryId: string,
	sourcePath: string,
	skillId: string,
	skillName: string,
};

export type CentralRepositoryAdditionUnskipRequest = {
	repositoryId: string,
	sourcePath: string,
};

export type CentralRepositorySyncApplyResult = CentralRepositorySyncApplyResult_Serialize | CentralRepositorySyncApplyResult_Deserialize;

export type CentralRepositorySyncApplyResult_Deserialize = {
	keptSkillIds: string[],
	deleteResult: BatchDeleteCentralSkillResult_Deserialize,
	importResults: GitHubRepoImportResult[],
	skippedAdditions: CentralRepositoryAdditionSkipRequest[],
	unskippedAdditions: CentralRepositoryAdditionUnskipRequest[],
	failedRepositories: CentralRepositorySyncFailure[],
	states: SkillUpdateState[],
};

export type CentralRepositorySyncApplyResult_Serialize = {
	keptSkillIds: string[],
	deleteResult: BatchDeleteCentralSkillResult_Serialize,
	importResults: GitHubRepoImportResult[],
	skippedAdditions: CentralRepositoryAdditionSkipRequest[],
	unskippedAdditions: CentralRepositoryAdditionUnskipRequest[],
	failedRepositories: CentralRepositorySyncFailure[],
	states: SkillUpdateState[],
};

export type CentralRepositorySyncDecisions = {
	keepSkillIds: string[],
	deleteRequests: BatchDeleteCentralSkillRequest[],
	additions: CentralRepositoryAddedSkillSelection[],
	skipAdditions?: CentralRepositoryAdditionSkipRequest[],
	unskipAdditions?: CentralRepositoryAdditionUnskipRequest[],
};

export type CentralRepositorySyncFailure = {
	repositoryId: string,
	name: string | null,
	error: string,
};

export type CentralStoreLocationApplyRequest = {
	targetPath: string,
	overwriteExisting: boolean,
};

export type CentralStoreLocationChangeResult = {
	sourcePath: string,
	targetPath: string,
	copied: number,
	overwritten: number,
	targetOnlyImported: number,
	symlinkRebuildFailed: number,
	symlinkFailures: CentralStoreLocationSymlinkFailure[],
	completedAt: string,
};

export type CentralStoreLocationPreview = {
	sourcePath: string,
	targetPath: string,
	skillsToCopy: number,
	skillsToOverwrite: number,
	targetOnlySkills: number,
};

export type CentralStoreLocationPreviewRequest = {
	targetPath: string,
};

export type CentralStoreLocationSymlinkFailure = {
	table: string,
	skillId: string,
	ownerId: string,
	installedPath: string,
	error: string,
};

export type Collection = {
	id: string,
	name: string,
	description: string | null,
	created_at: string,
	updated_at: string,
};

export type DeleteCentralSkillResult = {
	removed_central_path: string,
	removed_agent_ids: string[],
	retained_agent_ids: string[],
};

export type DeleteSkillRepositoryResult = DeleteSkillRepositoryResult_Serialize | DeleteSkillRepositoryResult_Deserialize;

export type DeleteSkillRepositoryResult_Deserialize = {
	repository: SkillRepository,
	deleted_repository: boolean,
	delete_result: BatchDeleteCentralSkillResult_Deserialize,
};

export type DeleteSkillRepositoryResult_Serialize = {
	repository: SkillRepository,
	deleted_repository: boolean,
	delete_result: BatchDeleteCentralSkillResult_Serialize,
};

export type DeletedPlatformCopyGroup = {
	agentId: string,
	skillId: string,
	skillName: string,
	writablePaths: string[],
};

export type DuplicateResolution = "overwrite" | "skip" | "rename";

export type FailedCentralSkillDelete = FailedCentralSkillDelete_Serialize | FailedCentralSkillDelete_Deserialize;

export type FailedCentralSkillDelete_Deserialize = {
	skill_id: string,
	phase?: string | null,
	error_code?: string | null,
	error_category?: string | null,
	error: string,
};

export type FailedCentralSkillDelete_Serialize = {
	skill_id: string,
	phase: string | null,
	error_code: string | null,
	error_category: string | null,
	error: string,
};

/**  Describes a single failed install within a batch operation. */
export type FailedInstall = {
	agent_id: string,
	error: string,
};

export type FailedRepository = {
	repositoryId: string,
	error: string,
	/**
	 *  Stable IPC-style code for failures the domain classified, so the UI can
	 *  localize the reason instead of showing backend English. `None` for the
	 *  pre-existing reconciliation reasons that carry their own sentence, and
	 *  for inventories persisted before this field existed.
	 */
	errorCode?: string | null,
	/**
	 *  Static snapshot acquisition family. This is intentionally separate from
	 *  the public code so transport subtypes remain diagnosable without raw
	 *  request, response, URL, or status detail.
	 */
	diagnosticCategory?: string | null,
	retry?: FailedRepositoryRetry,
	diagnostics?: SkillUpdateDiagnostic | null,
};

/**  What the Update Center may offer on a failed repository row. */
export type FailedRepositoryRetry = 
/**
 *  Snapshot acquisition, relocation and addition-collection failures:
 *  running the same scope again can produce a different result.
 */
"retryable" | 
/**
 *  The tracked source path is gone and no unique new path was found, so a
 *  user decision (keep or delete) is required in incremental mode.
 */
"decision_required" | 
/**  Entries persisted before this field existed. No in-place action. */
"unknown";

export type ForceRepositoryMirrorRequest = {
	repositoryIds: string[],
	deleteMissing?: boolean,
	importAdded?: boolean,
	overwriteTracked?: boolean,
	removeCopyInstallationsForDeleted?: boolean,
};

export type ForceRepositoryMirrorResult = ForceRepositoryMirrorResult_Serialize | ForceRepositoryMirrorResult_Deserialize;

export type ForceRepositoryMirrorResult_Deserialize = {
	overwritten: ForceSkillUpdateSuccess[],
	imported: ImportedGitHubSkillSummary[],
	deleted: BatchDeleteCentralSkillResult_Deserialize,
	skipped: ForceSkillUpdateSkip[],
	failedRepositories: FailedRepository[],
	failedItems: ForceSkillUpdateFailure[],
};

export type ForceRepositoryMirrorResult_Serialize = {
	overwritten: ForceSkillUpdateSuccess[],
	imported: ImportedGitHubSkillSummary[],
	deleted: BatchDeleteCentralSkillResult_Serialize,
	skipped: ForceSkillUpdateSkip[],
	failedRepositories: FailedRepository[],
	failedItems: ForceSkillUpdateFailure[],
};

export type ForceSkillUpdateFailure = {
	skillId: string,
	repositoryId: string | null,
	sourcePath: string | null,
	error: string,
};

export type ForceSkillUpdateRequest = {
	skillIds: string[],
	refreshCopyInstallations?: boolean,
};

export type ForceSkillUpdateResult = {
	overwritten: ForceSkillUpdateSuccess[],
	skipped: ForceSkillUpdateSkip[],
	failed: ForceSkillUpdateFailure[],
};

export type ForceSkillUpdateSkip = {
	skillId: string,
	reason: string,
};

export type ForceSkillUpdateSuccess = {
	skillId: string,
	repositoryId: string | null,
	sourcePath: string | null,
	localHash: string | null,
	remoteHash: string | null,
	bytesChanged: boolean,
	copyInstallationsRefreshed: boolean,
};

export type GitHubPatState = GitHubPatState_Serialize | GitHubPatState_Deserialize;

export type GitHubPatState_Deserialize = {
	configured: boolean,
	storageState: SecretStorageState,
	error: string | null,
};

export type GitHubPatState_Serialize = {
	configured: boolean,
	storageState: SecretStorageState,
	error?: string | null,
};

export type GitHubPatTestResult = {
	configured: boolean,
	ok: boolean,
	status: number | null,
	messageKey: string,
	message: string,
};

export type GitHubRepoImportResult = {
	repo: GitHubRepoRef,
	importedSkills: ImportedGitHubSkillSummary[],
	skippedSkills: string[],
};

export type GitHubRepoRef = {
	owner: string,
	repo: string,
	branch: string,
	normalizedUrl: string,
};

export type GitHubSkillImportSelection = {
	sourcePath: string,
	resolution: DuplicateResolution,
	renamedSkillId: string | null,
};

export type ImportedGitHubSkillSummary = {
	sourcePath: string,
	originalSkillId: string,
	importedSkillId: string,
	skillName: string,
	targetDirectory: string,
	resolution: DuplicateResolution,
};

/**  Result of a single skill install operation. */
export type InstallResult = {
	symlink_path: string,
};

/**  Stable error envelope serialized across the Tauri command boundary. */
export type IpcError = IpcError_Serialize | IpcError_Deserialize;

/**  Stable error envelope serialized across the Tauri command boundary. */
export type IpcError_Deserialize = {
	code: string,
	message: string,
	retryable: boolean,
	/**
	 *  Operation Log row UUID used to correlate the rejection with audit and
	 *  Runtime evidence. Missing for legacy/backend-internal failures.
	 */
	correlationId?: string | null,
};

/**  Stable error envelope serialized across the Tauri command boundary. */
export type IpcError_Serialize = {
	code: string,
	message: string,
	retryable: boolean,
	/**
	 *  Operation Log row UUID used to correlate the rejection with audit and
	 *  Runtime evidence. Missing for legacy/backend-internal failures.
	 */
	correlationId?: string | null,
};

export type LocalRemoteSyncApplyRequest = {
	targetId: string,
	repoPath?: string | null,
};

export type LocalRemoteSyncApplyResult = LocalRemoteSyncApplyResult_Serialize | LocalRemoteSyncApplyResult_Deserialize;

export type LocalRemoteSyncApplyResult_Deserialize = {
	targetId: string,
	targetLabel: string,
	syncedRepo: LocalRemoteSyncItemPreview_Deserialize | null,
	syncedSkills: LocalRemoteSyncItemPreview_Deserialize[],
	skippedSkills: LocalRemoteSyncItemPreview_Deserialize[],
	failed: LocalRemoteSyncFailure[],
};

export type LocalRemoteSyncApplyResult_Serialize = {
	targetId: string,
	targetLabel: string,
	syncedRepo: LocalRemoteSyncItemPreview_Serialize | null,
	syncedSkills: LocalRemoteSyncItemPreview_Serialize[],
	skippedSkills: LocalRemoteSyncItemPreview_Serialize[],
	failed: LocalRemoteSyncFailure[],
};

export type LocalRemoteSyncFailure = {
	id: string,
	label: string,
	targetPath: string,
	error: string,
};

export type LocalRemoteSyncItemKind = "repo" | "skill";

export type LocalRemoteSyncItemPreview = LocalRemoteSyncItemPreview_Serialize | LocalRemoteSyncItemPreview_Deserialize;

export type LocalRemoteSyncItemPreview_Deserialize = {
	id: string,
	label: string,
	kind: LocalRemoteSyncItemKind,
	localPath: string,
	remotePath: string,
	fileCount: number,
	byteCount: number,
	localHash: string,
	remoteHash: string | null,
	status: LocalRemoteSyncItemStatus,
	error: string | null,
};

export type LocalRemoteSyncItemPreview_Serialize = {
	id: string,
	label: string,
	kind: LocalRemoteSyncItemKind,
	localPath: string,
	remotePath: string,
	fileCount: number,
	byteCount: number,
	localHash: string,
	remoteHash: string | null,
	status: LocalRemoteSyncItemStatus,
	error?: string | null,
};

export type LocalRemoteSyncItemStatus = "add" | "update" | "skip" | "error";

export type LocalRemoteSyncPreview = LocalRemoteSyncPreview_Serialize | LocalRemoteSyncPreview_Deserialize;

export type LocalRemoteSyncPreviewRequest = {
	targetId: string,
	repoPath?: string | null,
};

export type LocalRemoteSyncPreview_Deserialize = {
	targetId: string,
	targetLabel: string,
	repoRoot: string,
	skillsRoot: string,
	repo: LocalRemoteSyncItemPreview_Deserialize,
	skills: LocalRemoteSyncItemPreview_Deserialize[],
	totalFileCount: number,
	totalByteCount: number,
};

export type LocalRemoteSyncPreview_Serialize = {
	targetId: string,
	targetLabel: string,
	repoRoot: string,
	skillsRoot: string,
	repo: LocalRemoteSyncItemPreview_Serialize,
	skills: LocalRemoteSyncItemPreview_Serialize[],
	totalFileCount: number,
	totalByteCount: number,
};

export type ObsidianImportResult = {
	skill_id: string,
	target: string,
};

export type OrphanSkillEntry = {
	skillId: string,
	brokenPath: string,
};

export type PlatformDuplicateGroup = {
	agentId: string,
	skillId: string,
	skillName: string,
	writablePaths: string[],
	pluginPaths: string[],
};

/**  项目下某个 agent 目录中登记的 skill 安装。 */
export type ProjectSkillInstallation = {
	project_id: string,
	skill_id: string,
	name: string,
	description: string | null,
	file_path: string,
	/**  `'central'` | `'project'`：中央库安装或项目原有/手动放入。 */
	source_origin: string,
	agent_id: string,
	installed_path: string,
	/**  `'symlink'` | `'copy'`。 */
	link_type: string,
	symlink_target: string | null,
	created_at: string,
};

export type RemoteAddedSkill = {
	repositoryId: string,
	sourcePath: string,
	skillId: string,
	skillName: string,
	conflictExistingSkillId: string | null,
};

export type RemoteMissingSkill = {
	state: SkillUpdateState,
	repositoryId: string | null,
	diagnostics?: SkillUpdateDiagnostic | null,
};

export type Result<T, E> = {
	ok: T,
	err: E,
};

export type SecretStorageState = "stored" | "session" | "missing" | "unreadable";

export type SkillRefreshCachePolicy = "use_fresh" | "bypass";

export type SkillRefreshMode = "regular" | "sync";

export type SkillRefreshScope = {
	kind: SkillRefreshScopeKind,
	mode?: SkillRefreshMode | null,
	cachePolicy?: SkillRefreshCachePolicy | null,
	skillIds?: string[] | null,
	repositoryIds?: string[] | null,
	agentIds?: string[] | null,
};

export type SkillRefreshScopeKind = "all" | "skills" | "repositories" | "platform";

export type SkillRepository = {
	id: string,
	name: string,
	source_type: string,
	owner: string | null,
	repo: string | null,
	branch: string | null,
	url: string | null,
	pinned: boolean,
	is_unknown: boolean,
	created_at: string,
	updated_at: string,
	/**
	 *  repo 级最后一次 inventory refresh 的时间戳（ISO-8601）。Phase P2 引入。
	 *  旧 DB 升级时通过 ensure_column 安全加列，默认为 NULL。
	 */
	last_synced_at?: string | null,
};

export type SkillUpdateDiagnostic = {
	sourceUrl: string | null,
	ref: string | null,
	sourcePath: string | null,
	localHash: string | null,
	baselineHash: string | null,
	remoteHash: string | null,
	localVersion: string | null,
	remoteVersion: string | null,
	cachePolicy: SkillRefreshCachePolicy,
	cacheHit: boolean,
	snapshotFetchedAt: string | null,
};

export type SkillUpdateInventory = SkillUpdateInventory_Serialize | SkillUpdateInventory_Deserialize;

export type SkillUpdateInventory_Deserialize = {
	updatable: UpdatableSkill[],
	remoteAdded: RemoteAddedSkill[],
	remoteMissing: RemoteMissingSkill[],
	unsupported?: UnsupportedSkill[],
	platformDuplicates: PlatformDuplicateGroup[],
	deletedPlatformCopies?: DeletedPlatformCopyGroup[],
	/**  Phase P2 始终空，留位给后续 orphan 扫描（broken symlink / 孤儿 .copy 目录）。 */
	orphans: OrphanSkillEntry[],
	failedRepositories: FailedRepository[],
	snapshotRetryAttempted?: number | null,
	snapshotRetryRecovered?: number | null,
	generatedAt: string,
};

export type SkillUpdateInventory_Serialize = {
	updatable: UpdatableSkill[],
	remoteAdded: RemoteAddedSkill[],
	remoteMissing: RemoteMissingSkill[],
	unsupported: UnsupportedSkill[],
	platformDuplicates: PlatformDuplicateGroup[],
	deletedPlatformCopies: DeletedPlatformCopyGroup[],
	/**  Phase P2 始终空，留位给后续 orphan 扫描（broken symlink / 孤儿 .copy 目录）。 */
	orphans: OrphanSkillEntry[],
	failedRepositories: FailedRepository[],
	snapshotRetryAttempted: number | null,
	snapshotRetryRecovered: number | null,
	generatedAt: string,
};

export type SkillUpdateState = {
	skill_id: string,
	source_type: string,
	source_url: string | null,
	ref: string | null,
	source_path: string | null,
	last_remote_hash: string | null,
	latest_remote_hash: string | null,
	last_checked_at: string | null,
	last_updated_at: string | null,
	status: SkillUpdateStatus,
	error: string | null,
};

export type SkillUpdateStatus = "up_to_date" | "update_available" | "unsupported" | "remote_missing" | "error" | "cancelled";

/**  Summary of a completed global install. */
export type SkillsCliAddResult = {
	installedSkills: number,
	targetedPlatforms: number,
};

export type SkillsCliApplyRecoveryResult = {
	operationId: string,
	phase: string,
};

export type SkillsCliApplyResult = {
	appliedSkillNames: string[],
	installedRevisionSha: string,
};

export type SkillsCliApplySelection = {
	skillName: string,
	skillPath: string,
	expectedInstalledRevision: string | null,
	expectedInstalledLocalDigest: string | null,
	expectedPendingRevision: string,
	expectedPendingDigest: string,
};

export type SkillsCliApplyUpdateRequest = {
	jobId: string,
	repositoryKey: string,
	selections: SkillsCliApplySelection[],
};

/**  Result of `skills_cli_doctor`. */
export type SkillsCliDoctorReport = {
	nodeVersion: string,
	npmSpec: string,
};

/**  One global skill projected from lock v3 + filesystem (no CLI spawn). */
export type SkillsCliGlobalSkill = {
	name: string,
	path: string | null,
	installKind: SkillsCliInstallKind,
	scope: string | null,
	agents: string[],
	source: string | null,
	sourceUrl: string | null,
	sourceType: string | null,
	sourceTypeBucket: SkillsCliSourceTypeBucket,
	canonicalPath: string | null,
	folderHash: string | null,
	installedAt: string | null,
	updatedAt: string | null,
	placements: SkillsCliPlacement[],
};

/**  Lock + filesystem snapshot returned by `skills_cli_list_global`. */
export type SkillsCliGlobalSnapshot = {
	skills: SkillsCliGlobalSkill[],
	canonicalRoot: string,
	lockPath: string,
};

export type SkillsCliInstallKind = "canonical" | "copy" | "missing";

/**  One detected, mappable Local platform offered by the install flow. */
export type SkillsCliInstallTarget = {
	id: string,
	displayName: string,
	iconName: string | null,
	/**  CLI `--agent` id this platform maps to. */
	cliAgent: string,
	/**  SkillPort enablement state; drives the default selection. */
	isEnabled: boolean,
	defaultSelected: boolean,
};

export type SkillsCliManagedLinkKind = "windows_junction" | "symlink";

export type SkillsCliPendingRecovery = {
	operationId: string,
	phase: string,
	lastErrorCode: string | null,
};

export type SkillsCliPlacement = {
	agentId: string,
	displayName: string,
	targetPath: string,
	state: SkillsCliPlacementState,
	managedLinkKind: SkillsCliManagedLinkKind | null,
	reasonCode: string | null,
};

export type SkillsCliPlacementConflict = {
	agentId: string,
	displayName: string,
	reasonCode: string,
};

export type SkillsCliPlacementState = "managed_link" | "direct_copy" | "missing" | "conflict" | "unavailable";

export type SkillsCliRemovePlacementSummary = {
	agentId: string,
	displayName: string,
};

export type SkillsCliRemovePlan = {
	skillName: string,
	ownedCanonical: boolean,
	managedPlacements: SkillsCliRemovePlacementSummary[],
	retainedDirectCopies: SkillsCliRemovePlacementSummary[],
	conflicts: SkillsCliPlacementConflict[],
	confirmable: boolean,
};

export type SkillsCliRemoveResult = {
	removedCanonical: boolean,
	removedManagedAgentIds: string[],
	retainedDirectCopyAgentIds: string[],
};

export type SkillsCliSkillDoc = {
	skillName: string,
	content: string,
	byteSize: number,
};

/**  Parsed result of `skills add <source> --list`. */
export type SkillsCliSourcePreview = {
	source: string,
	skills: string[],
};

export type SkillsCliSourceTypeBucket = "github" | "gitlab" | "git" | "mintlify" | "huggingface" | "local" | "well-known" | "unknown";

export type SkillsCliUpdateBlocker = {
	code: string,
	skillName: string,
};

export type SkillsCliUpdateCapabilityPlan = {
	npmSpec: string,
	forceFlag: CapabilitySupport,
	keepLinksFlag: CapabilitySupport,
	pinnedFullShaSource: CapabilitySupport,
	directCopyRefresh: CapabilitySupport,
	applyMethod: string,
};

export type SkillsCliUpdateInventory = {
	skills: SkillsCliUpdateSkillRow[],
	repositories: SkillsCliUpdateRepositoryRow[],
	lastSuccessAt: string | null,
	pendingRecovery: SkillsCliPendingRecovery | null,
	capability: SkillsCliUpdateCapabilityPlan,
};

export type SkillsCliUpdateRepositoryRow = {
	repositoryKey: string,
	normalizedSource: string,
	branch: string,
	observedRevisionSha: string | null,
	status: string,
	lastCheckedAt: string | null,
	lastErrorCode: string | null,
	rateLimitResetAt: string | null,
	pendingCount: number,
};

export type SkillsCliUpdateSkillRow = {
	skillName: string,
	repositoryKey: string | null,
	normalizedSource: string | null,
	skillPath: string | null,
	status: SkillsCliUpdateStatus,
	installedRevisionSha: string | null,
	observedRevisionSha: string | null,
	pendingRevisionSha: string | null,
	installedLocalDigest: string | null,
	observedUpstreamDigest: string | null,
	pendingUpstreamDigest: string | null,
	isStale: boolean,
	lastErrorCode: string | null,
	changeSummary: string[],
	blockers: SkillsCliUpdateBlocker[],
	argvPreview: string[],
};

export type SkillsCliUpdateStatus = "not_checked" | "checking" | "current" | "update_available" | "local_modified" | "baseline_required" | "unsupported" | "rate_limited" | "failed";

/**  Describes a target that was already installed and safely left in place. */
export type SkippedInstall = {
	agent_id: string,
	target_path: string,
	reason: string,
};

export type UnsupportedSkill = {
	skillId: string,
	reasonCode: UnsupportedSkillReasonCode,
};

export type UnsupportedSkillReasonCode = "unknown_source" | "unsupported_source_type" | "missing_source_path" | "unsupported_source";

export type UpdatableSkill = {
	state: SkillUpdateState,
	repositoryId: string | null,
	diagnostics?: SkillUpdateDiagnostic | null,
};
