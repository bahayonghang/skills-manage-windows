// Generated from Rust/Serde metadata by `pnpm ipc:codegen`. Do not edit.
// This artifact contains contract metadata only and never invokes Tauri.

type GeneratedIpcCommandSpec<Args, Result> = { args: Args; result: Result };
const command = <Args, Result>() => ({}) as GeneratedIpcCommandSpec<Args, Result>;

export const GENERATED_IPC_COMMANDS = {
  apply_central_repository_sync: command<{ decisions: CentralRepositorySyncDecisions }, CentralRepositorySyncApplyResult>(),
  apply_central_store_location_change: command<{ request: CentralStoreLocationApplyRequest }, CentralStoreLocationChangeResult>(),
  apply_local_remote_sync: command<{ request: LocalRemoteSyncApplyRequest }, LocalRemoteSyncApplyResult_Serialize>(),
  batch_install_central_skills: command<{ skillIds: string[]; agentIds: string[]; method: string | null; projectPath: string | null }, CentralBatchInstallResult>(),
  batch_install_collection: command<{ collectionId: string; agentIds: string[] }, BatchInstallResult>(),
  batch_install_to_agents: command<{ skillId: string; agentIds: string[]; method: string | null }, BatchInstallResult>(),
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
  delete_central_skill: command<{ skillId: string; removeAgentIds: string[] }, DeleteCentralSkillResult>(),
  delete_central_skills: command<{ requests: BatchDeleteCentralSkillRequest[] }, BatchDeleteCentralSkillResult>(),
  delete_collection: command<{ collectionId: string }, null>(),
  delete_skill_repository: command<{ repositoryId: string; requests: BatchDeleteCentralSkillRequest[] }, DeleteSkillRepositoryResult>(),
  force_mirror_central_repositories: command<{ request: ForceRepositoryMirrorRequest }, ForceRepositoryMirrorResult>(),
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
  "test_ai_connection",
  "test_github_pat",
  "unassign_skill_tags",
  "uninstall_skill_from_project",
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
};

export type BatchDeleteCentralSkillResult = {
	succeeded: BatchDeleteCentralSkillSuccess[],
	failed: FailedCentralSkillDelete[],
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

export type CentralRepositorySyncApplyResult = {
	keptSkillIds: string[],
	deleteResult: BatchDeleteCentralSkillResult,
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

export type DeleteSkillRepositoryResult = {
	repository: SkillRepository,
	deleted_repository: boolean,
	delete_result: BatchDeleteCentralSkillResult,
};

export type DeletedPlatformCopyGroup = {
	agentId: string,
	skillId: string,
	skillName: string,
	writablePaths: string[],
};

export type DuplicateResolution = "overwrite" | "skip" | "rename";

export type FailedCentralSkillDelete = {
	skill_id: string,
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

export type ForceRepositoryMirrorResult = {
	overwritten: ForceSkillUpdateSuccess[],
	imported: ImportedGitHubSkillSummary[],
	deleted: BatchDeleteCentralSkillResult,
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
export type IpcError = {
	code: string,
	message: string,
	retryable: boolean,
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
