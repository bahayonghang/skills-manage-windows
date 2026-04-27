// ─── Agent Types ─────────────────────────────────────────────────────────────

export interface AgentWithStatus {
  id: string;
  display_name: string;
  category: string;
  global_skills_dir: string;
  project_skills_dir?: string;
  icon_name?: string;
  is_detected: boolean;
  is_builtin: boolean;
  is_enabled: boolean;
}

export interface CustomAgentConfig {
  id?: string;
  display_name: string;
  category?: string;
  global_skills_dir: string;
}

export interface UpdateCustomAgentConfig {
  display_name: string;
  category?: string;
  global_skills_dir: string;
}

// ─── Scan Types ───────────────────────────────────────────────────────────────

export interface ScanResult {
  total_skills: number;
  agents_scanned: number;
  skills_by_agent: Record<string, number>;
}

export type ScanState = "idle" | "refreshing" | "error";

export interface SkillCountsSummary {
  cachedSkillCounts: Record<string, number>;
  lastScanAt: string | null;
  scanState: ScanState;
}

export interface BootstrapSnapshot {
  agents: AgentWithStatus[];
  cachedSkillCounts: Record<string, number>;
  collectionCount: number;
  discoveredCount: number;
  lastScanAt: string | null;
  scanState: ScanState;
}
export type ClaudeSourceKind = "user" | "plugin";

export interface ScannedSkill {
  id: string;
  row_id?: string;
  name: string;
  description?: string;
  file_path: string;
  dir_path: string;
  link_type: string;
  symlink_target?: string;
  is_central: boolean;
  source_kind?: ClaudeSourceKind | null;
  source_root?: string | null;
  is_read_only?: boolean;
  conflict_group?: string | null;
  conflict_count?: number;
}

// ─── Skill Types ──────────────────────────────────────────────────────────────

export interface Skill {
  id: string;
  name: string;
  description?: string;
  file_path: string;
  canonical_path?: string;
  is_central: boolean;
  source?: string;
  content?: string;
  scanned_at: string;
}

export interface SkillInstallation {
  skill_id: string;
  agent_id: string;
  installed_path: string;
  link_type: string;
  symlink_target?: string;
  /** ISO 8601 timestamp of when the skill was first installed. */
  installed_at?: string;
}

export interface SkillDetail extends Omit<Skill, "content"> {
  row_id?: string;
  dir_path?: string;
  source_kind?: ClaudeSourceKind | null;
  source_root?: string | null;
  is_read_only?: boolean;
  conflict_group?: string | null;
  conflict_count?: number;
  installations: SkillInstallation[];
  /** Collections this skill currently belongs to. */
  collections?: Collection[];
  repository?: SkillRepository;
  tags?: SkillTag[];
  source_path?: string;
  is_source_unknown?: boolean;
}

export interface SkillDetailRequest {
  skillId: string;
  agentId?: string;
  rowId?: string;
}

export interface SkillWithLinks {
  id: string;
  name: string;
  description?: string;
  file_path: string;
  canonical_path?: string;
  is_central: boolean;
  source?: string;
  scanned_at: string;
  created_at?: string;
  updated_at?: string;
  /** Agent IDs that currently have this skill installed (symlink or copy). */
  linked_agents: string[];
  /** Agent IDs that share the Central skills directory. */
  shared_root_agents: string[];
  repository?: SkillRepository;
  tags?: SkillTag[];
  source_path?: string;
  is_source_unknown?: boolean;
}

export interface BatchInstallResult {
  succeeded: string[];
  failed: Array<{ agent_id: string; error: string }>;
}

export interface CentralBatchInstallSuccess {
  skill_id: string;
  agent_id: string;
  target_path: string;
}

export interface CentralBatchInstallFailure {
  skill_id: string;
  agent_id: string;
  error: string;
}

export interface CentralBatchInstallResult {
  succeeded: CentralBatchInstallSuccess[];
  failed: CentralBatchInstallFailure[];
}

export interface DeleteCentralSkillPreview {
  skill_id: string;
  skill_name: string;
  central_path: string;
  copy_installations: SkillInstallation[];
  auto_removed_agent_ids: string[];
}

export interface FailedCentralSkillDelete {
  skill_id: string;
  error: string;
}

export interface BatchDeleteCentralSkillPreviewResult {
  previews: DeleteCentralSkillPreview[];
  failed: FailedCentralSkillDelete[];
}

export interface BatchDeleteCentralSkillRequest {
  skill_id: string;
  remove_agent_ids: string[];
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

// ─── Collection Types ─────────────────────────────────────────────────────────

export interface Collection {
  id: string;
  name: string;
  description?: string;
  created_at: string;
  updated_at: string;
}

export interface CollectionWithSkills extends Collection {
  skill_ids: string[];
}

export interface CollectionDetail extends Collection {
  /** Full skill objects that are members of this collection. */
  skills: Skill[];
}

export interface CollectionBatchInstallResult {
  succeeded: string[];
  failed: Array<{ agent_id: string; error: string }>;
}

// ─── Repository and Tag Metadata Types ───────────────────────────────────────

export interface SkillRepository {
  id: string;
  name: string;
  source_type: string;
  owner?: string;
  repo?: string;
  branch?: string;
  url?: string;
  is_unknown: boolean;
  created_at: string;
  updated_at: string;
}

export interface SkillRepositoryWithStats extends SkillRepository {
  skill_count: number;
  unknown_skill_count: number;
}

export interface SkillTag {
  id: string;
  name: string;
  description?: string;
  color?: string;
  is_builtin: boolean;
  created_at: string;
  updated_at: string;
}

export interface SkillTagSuggestion {
  skill_id: string;
  tag: SkillTag;
  confidence: number;
  reason: string;
}

export interface SkillTagSuggestionResult {
  skill_id: string;
  skill_name?: string;
  suggestions: SkillTagSuggestion[];
  succeeded?: boolean;
  error?: string;
  low_confidence_count?: number;
}

export interface SkillAiTagReview {
  skill_id: string;
  skill_name: string;
  tag: SkillTag;
  confidence: number;
  reason: string;
  suggested_at: string;
  updated_at: string;
}

export type AiTagItemStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";

export type AiTagJobStatus = "idle" | "running" | "completed" | "failed" | "cancelled";

export interface AiTagJob {
  jobId: string | null;
  status: AiTagJobStatus;
  total: number;
  completed: number;
  succeeded: number;
  failed: number;
  lowConfidenceCount: number;
  currentSkillName?: string;
  error?: string;
  items: Record<string, AiTagItemStatus>;
}

export interface AiTagProgressPayload {
  jobId: string;
  skillId?: string;
  skillName?: string;
  status: "started" | "running" | "succeeded" | "failed" | "completed" | "cancelled";
  total: number;
  completed: number;
  succeeded: number;
  failed: number;
  lowConfidenceCount?: number;
  suggestions?: SkillTagSuggestion[];
  error?: string;
}

// ─── Settings Types ───────────────────────────────────────────────────────────

export type CentralSkillUpdateStatus =
  | "up_to_date"
  | "update_available"
  | "unsupported"
  | "remote_missing"
  | "error";

export interface CentralSkillUpdateState {
  skill_id: string;
  source_type: string;
  source_url?: string | null;
  ref?: string | null;
  source_path?: string | null;
  last_remote_hash?: string | null;
  latest_remote_hash?: string | null;
  last_checked_at?: string | null;
  last_updated_at?: string | null;
  status: CentralSkillUpdateStatus;
  error?: string | null;
}

export type CentralSkillUpdateItemStatus =
  | "queued"
  | "running"
  | "succeeded"
  | "failed"
  | "skipped";

export type CentralSkillUpdateJobStatus = "idle" | "running" | "completed" | "failed";

export interface CentralSkillUpdateJob {
  phase: "checking" | "updating" | null;
  status: CentralSkillUpdateJobStatus;
  total: number;
  completed: number;
  succeeded: number;
  failed: number;
  skipped: number;
  currentSkillName?: string;
  error?: string;
  items: Record<string, CentralSkillUpdateItemStatus>;
}

export interface CentralSkillUpdateProgressPayload {
  phase: "checking" | "updating";
  skillId?: string;
  skillName?: string;
  status:
    | "started"
    | "running"
    | "completed"
    | CentralSkillUpdateStatus;
  total: number;
  completed: number;
  succeeded: number;
  failed: number;
  skipped: number;
  error?: string;
}

export interface CentralSkillUpdateResult {
  succeeded: string[];
  failed: Array<{ skillId: string; error: string }>;
  skipped: Array<{ skillId: string; reason: string }>;
  states: CentralSkillUpdateState[];
}

export interface ScanDirectory {
  id: number;
  path: string;
  label?: string;
  is_active: boolean;
  is_builtin: boolean;
  added_at: string;
}

export type TargetKind = "local" | "ssh";
export type SshAuthMethod = "key" | "password";
export type TargetCredentialStatus = "stored" | "session" | "missing" | "unreadable";

export interface TargetSummary {
  id: string;
  kind: TargetKind;
  label: string;
  host?: string | null;
  username?: string | null;
  port?: number | null;
  authMethod?: SshAuthMethod | null;
  remoteHome?: string | null;
  remoteOs?: string | null;
  cacheDbPath?: string | null;
  hasStoredPassword?: boolean | null;
  credentialStatus?: TargetCredentialStatus | null;
  credentialError?: string | null;
  symlinkEnabled?: boolean | null;
  isActive: boolean;
}

export interface CreateSshTargetRequest {
  label: string;
  host: string;
  username: string;
  port?: number | null;
  authMethod?: SshAuthMethod | null;
  keyPath?: string | null;
  password?: string | null;
  passphrase?: string | null;
}

export interface TestSshTargetRequest {
  id?: string | null;
  label?: string | null;
  host?: string | null;
  username?: string | null;
  port?: number | null;
  authMethod?: SshAuthMethod | null;
  keyPath?: string | null;
  password?: string | null;
  passphrase?: string | null;
}

export interface SshTargetTestResult {
  ok: boolean;
  remoteHome?: string | null;
  remoteOs?: string | null;
  credentialStatus?: TargetCredentialStatus | null;
  credentialError?: string | null;
  message: string;
}

export interface GitHubPatTestResult {
  configured: boolean;
  ok: boolean;
  status?: number | null;
  message: string;
}

// ─── Discover Types ───────────────────────────────────────────────────────────

export interface ScanRoot {
  path: string;
  label: string;
  exists: boolean;
  enabled: boolean;
}

export interface DiscoveredSkill {
  id: string;
  name: string;
  description?: string;
  file_path: string;
  dir_path: string;
  platform_id: string;
  platform_name: string;
  project_path: string;
  project_name: string;
  is_already_central: boolean;
}

export interface DiscoveredProject {
  project_path: string;
  project_name: string;
  skills: DiscoveredSkill[];
}

export interface DiscoverResult {
  total_projects: number;
  total_skills: number;
  projects: DiscoveredProject[];
}

export interface DiscoveredSummary {
  totalSkillsFound: number;
  totalProjectsFound: number;
}

export interface DiscoverProgressPayload {
  percent: number;
  current_path: string;
  skills_found: number;
  projects_found: number;
}

export interface DiscoverFoundPayload {
  project: DiscoveredProject;
}

export interface DiscoverCompletePayload {
  total_projects: number;
  total_skills: number;
}

export type ImportTarget =
  | { type: "central" }
  | { type: "platform"; agent_id: string };

export interface DiscoverImportResult {
  skill_id: string;
  target: string;
}

// ─── Marketplace Types ───────────────────────────────────────────────────────

export interface SkillRegistry {
  id: string;
  name: string;
  source_type: "github" | "http_json";
  url: string;
  normalized_url?: string | null;
  is_builtin: boolean;
  is_enabled: boolean;
  last_synced: string | null;
  last_attempted_sync?: string | null;
  last_sync_status?: "never" | "success" | "error";
  last_sync_error?: string | null;
  cache_updated_at?: string | null;
  cache_expires_at?: string | null;
  etag?: string | null;
  last_modified?: string | null;
  created_at: string;
}

export interface MarketplaceSkill {
  id: string;
  registry_id: string;
  name: string;
  description?: string;
  download_url: string;
  is_installed: boolean;
  synced_at: string;
  cache_updated_at?: string | null;
}

// ─── Portable State Types ───────────────────────────────────────────────────

export type SkillportStateSourceStatus = "exists" | "will_add";
export type SkillportStateSkillStatus = "ready" | "conflict" | "missing" | "unrestorable";
export type SkillportStateImportResolutionType = "overwrite" | "skip" | "rename";

export interface SkillportStateImportPreviewSummary {
  sourcesToAdd: number;
  sourcesExisting: number;
  ready: number;
  conflicts: number;
  missing: number;
  unrestorable: number;
}

export interface SkillportStateSourcePreview {
  name: string;
  url: string;
  status: SkillportStateSourceStatus;
}

export interface SkillportStateSkillPreview {
  id: string;
  name: string;
  sourcePath?: string | null;
  status: SkillportStateSkillStatus;
  existingSkillId?: string | null;
  reason?: string | null;
}

export interface SkillportStateImportPreview {
  githubSources: SkillportStateSourcePreview[];
  skills: SkillportStateSkillPreview[];
  summary: SkillportStateImportPreviewSummary;
}

export interface SkillportStateImportResolution {
  skillId: string;
  sourcePath?: string | null;
  resolution: SkillportStateImportResolutionType;
  renamedSkillId?: string | null;
}

export interface SkillportStateImportedSkill {
  sourcePath: string;
  importedSkillId: string;
  skillName: string;
}

export interface SkillportStateImportFailure {
  skillId: string;
  sourcePath?: string | null;
  error: string;
}

export interface SkillportStateImportResult {
  sourcesAdded: number;
  sourcesSkipped: number;
  importedSkills: SkillportStateImportedSkill[];
  skippedSkills: string[];
  failedSkills: SkillportStateImportFailure[];
  tagsRestored: number;
}

export interface GitHubRepoRef {
  owner: string;
  repo: string;
  branch: string;
  normalizedUrl: string;
}

export interface GitHubSkillConflict {
  existingSkillId: string;
  existingName: string;
  existingCanonicalPath?: string | null;
  proposedSkillId: string;
  proposedName: string;
}

export interface GitHubSkillPreview {
  sourcePath: string;
  skillId: string;
  skillName: string;
  description?: string | null;
  rootDirectory: string;
  skillDirectoryName: string;
  downloadUrl: string;
  conflict?: GitHubSkillConflict | null;
}

export interface GitHubRepoPreview {
  repo: GitHubRepoRef;
  skills: GitHubSkillPreview[];
  previewWorkspaceId?: string | null;
}

export type DuplicateResolution = "overwrite" | "skip" | "rename";

export interface GitHubSkillImportSelection {
  sourcePath: string;
  resolution: DuplicateResolution;
  renamedSkillId?: string | null;
}

export interface ImportedGitHubSkillSummary {
  sourcePath: string;
  originalSkillId: string;
  importedSkillId: string;
  skillName: string;
  targetDirectory: string;
  resolution: DuplicateResolution;
}

export interface GitHubRepoImportResult {
  repo: GitHubRepoRef;
  importedSkills: ImportedGitHubSkillSummary[];
  skippedSkills: string[];
}

export type GitHubImportProgressPhase = "preparing" | "writing" | "finalizing";

export interface GitHubImportProgressPayload {
  phase: GitHubImportProgressPhase;
  currentSkill?: string | null;
  currentPath?: string | null;
  completedFiles: number;
  totalFiles: number;
  completedBytes: number;
  totalBytes: number;
}
