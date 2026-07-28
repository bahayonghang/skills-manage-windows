import { GENERATED_IPC_COMMANDS } from "./generatedCommandMap";

import type {
  AgentWithStatus,
  ArchiveFingerprint,
  BatchUninstallSkillRequest,
  BatchUninstallSkillResult,
  BootstrapSnapshot,
  CentralTopTag,
  CentralSkillUpdateResult,
  CentralSkillUpdateState,
  CreateSshTargetRequest,
  CreateWslTargetRequest,
  CustomAgentConfig,
  DailyOperationCount,
  DashboardCentralSummary,
  DirectoryTreeEntry,
  GitHubRepoImportResult,
  GitHubRepoPreview,
  GitHubRepoRef,
  GitHubSkillImportSelection,
  ObsidianSkill,
  ObsidianVault,
  OperationLogEntry,
  OperationLogFilter,
  OperationLogPage,
  PendingFsDbOperation,
  LocalArchiveImportResolution,
  LocalArchiveImportResult,
  LocalArchivePreview,
  PlatformPathMap,
  RuntimeLogClearRequest,
  RuntimeLogFile,
  RuntimeLogReadRequest,
  RuntimeLogReadResult,
  SavedView,
  ScanResult,
  ScannedSkill,
  SkillCountsSummary,
  SkillDetail,
  SkillDetailRequest,
  SkillportStateImportPreview,
  SkillportStateImportResolution,
  SkillportStateImportResult,
  SkillWithLinks,
  SshTargetTestResult,
  StartupStatus,
  TagGroup,
  TargetSummary,
  TargetConfigQuarantineStatus,
  TestSshTargetRequest,
  TestWslTargetRequest,
  UpdateCustomAgentConfig,
  UpdateSshTargetRequest,
  UpdateWslTargetRequest,
  WslDistributionSummary,
  WslTargetTestResult,
} from "@/types";
import type {
  CentralRepositorySyncPreview,
} from "@/types/centralRepositorySync";
import type {
  SkillUpdateApplyResult,
  SkillUpdateDecisions,
} from "@/types/skillUpdateInventory";
import type { SkillExplanationSummaryMap } from "@/types/skillExplanation";
import type {
  ProviderHealth,
  RecentSkillCall,
  SkillUsageDetail,
  UsageOverview,
  UsageRefreshResult,
  UsageScopeInfo,
} from "@/types/usage";

export interface SkillPathRequest extends SkillDetailRequest {
  path: string;
}

type IpcCommandSpec<Args, Result> = {
  args: Args;
  result: Result;
};

// 类型与运行时命令名单一来源：值本身是 phantom（永不读取），只承载类型信息，
// 同时让 Object.keys 可枚举已类型化命令（供 ipcCommandCoverage 测试使用）。
const command = <Args, Result>() => ({}) as IpcCommandSpec<Args, Result>;

function mergeCommandMaps<
  Generated extends Record<string, IpcCommandSpec<unknown, unknown>>,
  Handwritten extends Record<string, IpcCommandSpec<unknown, unknown>>,
>(
  generated: Generated,
  handwritten: Handwritten &
    Record<Extract<keyof Generated, keyof Handwritten>, never>,
) {
  return { ...generated, ...handwritten } as const;
}

export const HANDWRITTEN_IPC_COMMANDS = {
  // ── startup gate ─────────────────────────────────────────────────────────
  get_startup_status: command<undefined, StartupStatus>(),
  retry_startup: command<undefined, StartupStatus>(),
  rebuild_startup_database: command<undefined, StartupStatus>(),
  exit_startup: command<undefined, void>(),
  // ── skill detail / obsidian（旧 lib/ipc.ts 首批）───────────────────────────
  get_skill_detail: command<SkillDetailRequest, SkillDetail>(),
  read_file_by_path: command<SkillPathRequest, string>(),
  list_directory_tree: command<SkillPathRequest, DirectoryTreeEntry[]>(),
  open_in_file_manager: command<SkillPathRequest, void>(),
  open_obsidian_path: command<{ path: string }, void>(),
  get_obsidian_vaults: command<undefined, ObsidianVault[]>(),
  get_obsidian_vault_skills: command<{ vaultId: string }, ObsidianSkill[]>(),
  // ── platform / bootstrap / settings ──────────────────────────────────────
  get_bootstrap_snapshot: command<undefined, BootstrapSnapshot>(),
  get_setting: command<{ key: string }, string | null>(),
  set_setting: command<{ key: string; value: string }, void>(),
  list_platform_paths: command<undefined, PlatformPathMap>(),
  scan_all_skills: command<undefined, ScanResult>(),
  get_skill_counts_summary: command<undefined, SkillCountsSummary>(),
  get_dashboard_central_summary: command<undefined, DashboardCentralSummary>(),
  mark_import_intent_frontend_ready: command<undefined, void>(),
  preview_local_skill_archive: command<
    { archivePath: string },
    LocalArchivePreview
  >(),
  import_local_skill_archive: command<
    {
      archivePath: string;
      expectedFingerprint: ArchiveFingerprint;
      resolution: LocalArchiveImportResolution;
      renamedSkillId?: string;
    },
    LocalArchiveImportResult
  >(),
  // ── GitHub import: every content read is bound to a preview snapshot ─────
  preview_github_repo_import: command<{ repoUrl: string }, GitHubRepoPreview>(),
  fetch_github_skill_markdown: command<
    {
      previewId: string;
      repo: GitHubRepoRef;
      sourcePath: string;
    },
    string
  >(),
  import_github_repo_skills: command<
    {
      previewId: string;
      repoUrl: string;
      selections: GitHubSkillImportSelection[];
    },
    GitHubRepoImportResult
  >(),
  discard_github_repo_preview_snapshot: command<{ previewId: string }, void>(),
  set_agent_enabled: command<
    { agentId: string; isEnabled: boolean },
    AgentWithStatus
  >(),
  add_custom_agent: command<{ config: CustomAgentConfig }, AgentWithStatus>(),
  update_custom_agent: command<
    { agentId: string; config: UpdateCustomAgentConfig },
    AgentWithStatus
  >(),
  remove_custom_agent: command<{ agentId: string }, void>(),
  preview_skillport_state_import_file: command<
    { jobId: string; path: string },
    { json: string; preview: SkillportStateImportPreview }
  >(),
  export_skillport_state: command<
    { jobId: string; options: Record<string, never> },
    string
  >(),
  preview_skillport_state_import: command<
    { jobId: string; json: string },
    SkillportStateImportPreview
  >(),
  import_skillport_state: command<
    { jobId: string; json: string; resolutions: SkillportStateImportResolution[] },
    SkillportStateImportResult
  >(),
  cancel_skillport_state_portability: command<{ jobId: string }, void>(),
  check_central_skill_updates: command<
    { jobId: string; skillIds: string[] | null },
    CentralSkillUpdateState[]
  >(),
  check_central_repository_sync: command<
    { jobId: string; repositoryIds: string[]; skillIds: string[] | null },
    CentralRepositorySyncPreview
  >(),
  update_central_skills: command<
    { jobId: string; skillIds: string[] },
    CentralSkillUpdateResult
  >(),
  cancel_central_skill_updates: command<{ jobId: string }, void>(),
  apply_skill_update_decisions: command<
    { jobId: string; decisions: SkillUpdateDecisions },
    SkillUpdateApplyResult
  >(),
  save_skillport_state_export: command<{ path: string; json: string }, void>(),
  // ── platform skills ───────────────────────────────────────────────────────
  get_skills_by_agent: command<{ agentId: string }, ScannedSkill[]>(),
  get_central_skills: command<undefined, SkillWithLinks[]>(),
  get_central_top_tags: command<{ limit: number }, CentralTopTag[]>(),
  uninstall_skill_from_agent: command<
    { skillId: string; agentId: string; rowId?: string },
    void
  >(),
  batch_uninstall_skills_from_agent: command<
    { agentId: string; requests: BatchUninstallSkillRequest[] },
    BatchUninstallSkillResult
  >(),
  // ── usage ─────────────────────────────────────────────────────────────────
  usage_refresh: command<{ force: boolean }, UsageRefreshResult>(),
  usage_get_overview: command<
    { topSkillsLimit: number; source: string | null },
    UsageOverview
  >(),
  usage_get_recent: command<
    { limit: number; source: string | null },
    RecentSkillCall[]
  >(),
  usage_get_providers: command<undefined, ProviderHealth[]>(),
  usage_get_skill_detail: command<
    { skill: string; source: string | null },
    SkillUsageDetail
  >(),
  usage_get_scope_info: command<undefined, UsageScopeInfo>(),
  usage_resolve_skill_id: command<{ skillName: string }, string | null>(),
  usage_get_skill_counts: command<
    { skills: string[]; days: number },
    Record<string, number>
  >(),
  // ── central metadata ──────────────────────────────────────────────────────
  get_skill_explanation_summaries: command<
    { skillIds: string[]; lang: string },
    SkillExplanationSummaryMap
  >(),
  // ── targets ───────────────────────────────────────────────────────────────
  list_targets: command<undefined, TargetSummary[]>(),
  get_target_config_quarantine_status: command<
    undefined,
    TargetConfigQuarantineStatus
  >(),
  list_wsl_distributions: command<undefined, WslDistributionSummary[]>(),
  create_ssh_target: command<
    { request: CreateSshTargetRequest },
    TargetSummary
  >(),
  update_ssh_target: command<
    { request: UpdateSshTargetRequest },
    TargetSummary
  >(),
  test_ssh_target: command<
    { request: TestSshTargetRequest },
    SshTargetTestResult
  >(),
  create_wsl_target: command<
    { request: CreateWslTargetRequest },
    TargetSummary
  >(),
  update_wsl_target: command<
    { request: UpdateWslTargetRequest },
    TargetSummary
  >(),
  test_wsl_target: command<
    { request: TestWslTargetRequest },
    WslTargetTestResult
  >(),
  update_ssh_target_password: command<
    { targetId: string; password: string },
    SshTargetTestResult
  >(),
  delete_target: command<{ targetId: string }, void>(),
  set_active_target: command<{ targetId: string }, TargetSummary>(),
  // ── operation logs ────────────────────────────────────────────────────────
  list_operation_logs: command<
    { filter: OperationLogFilter },
    OperationLogPage
  >(),
  get_operation_log: command<{ logId: string }, OperationLogEntry | null>(),
  clear_operation_logs: command<{ filter: OperationLogFilter }, number>(),
  export_operation_logs: command<{ filter: OperationLogFilter }, string>(),
  get_daily_operation_counts: command<{ days: number }, DailyOperationCount[]>(),
  list_pending_fs_db_operations: command<undefined, PendingFsDbOperation[]>(),
  retry_fs_db_operation: command<
    { operationId: string },
    PendingFsDbOperation[]
  >(),
  // ── runtime logs ──────────────────────────────────────────────────────────
  list_runtime_log_files: command<undefined, RuntimeLogFile[]>(),
  read_runtime_log_file: command<
    { request: RuntimeLogReadRequest },
    RuntimeLogReadResult
  >(),
  clear_runtime_logs: command<{ request: RuntimeLogClearRequest }, number>(),
  export_runtime_log_file: command<{ fileName: string }, string>(),
  // ── tag groups ────────────────────────────────────────────────────────────
  list_tag_groups: command<undefined, TagGroup[]>(),
  create_tag_group: command<
    { input: { name: string; color: string | null } },
    TagGroup
  >(),
  update_tag_group: command<
    { id: string; input: { name?: string; color?: string | null } },
    TagGroup
  >(),
  delete_tag_group: command<{ id: string }, void>(),
  reorder_tag_groups: command<{ ids: string[] }, void>(),
  set_tag_group: command<{ tagId: string; groupId: string | null }, void>(),
  // ── saved views ───────────────────────────────────────────────────────────
  list_saved_views: command<undefined, SavedView[]>(),
  create_saved_view: command<
    {
      input: {
        name: string;
        query: string;
        icon: string | null;
        pinned: boolean;
      };
    },
    SavedView
  >(),
  update_saved_view: command<
    {
      id: string;
      input: {
        name?: string;
        query?: string;
        icon?: string | null;
        pinned?: boolean;
      };
    },
    SavedView
  >(),
  delete_saved_view: command<{ id: string }, void>(),
  reorder_saved_views: command<{ ids: string[] }, void>(),
} as const;

export const IPC_COMMANDS = mergeCommandMaps(
  GENERATED_IPC_COMMANDS,
  HANDWRITTEN_IPC_COMMANDS,
);

export type IpcCommandMap = typeof IPC_COMMANDS;

export type CommandArgs<K extends keyof IpcCommandMap> =
  IpcCommandMap[K]["args"];
export type CommandResult<K extends keyof IpcCommandMap> =
  IpcCommandMap[K]["result"];

export const TYPED_IPC_COMMAND_NAMES: readonly string[] =
  Object.keys(IPC_COMMANDS);
export const HANDWRITTEN_IPC_COMMAND_NAMES: readonly string[] =
  Object.keys(HANDWRITTEN_IPC_COMMANDS);

/**
 * 尚未进入 IPC_COMMANDS 的存量命令允许清单（ratchet：只减不增）。
 * ipcCommandCoverage 测试强制全仓 invoke 字面量 ∈ IPC_COMMANDS ∪ 本清单，
 * 新增命令必须显式登记（优先入 IPC_COMMANDS）。
 */
export const UNTYPED_IPC_COMMANDS: readonly string[] = [
  "accept_ai_tag_review",
  "add_project",
  "add_registry",
  "add_scan_directory",
  "add_skill_to_collection",
  "assign_skill_tags",
  "assign_skills_to_repository",
  "browse_skills_sh_directory",
  "bulk_suggest_skill_tags",
  "cancel_ai_tag_job",
  "create_collection",
  "create_or_update_skill_repository",
  "create_skill_tag",
  "explain_skill",
  "explain_skill_stream",
  "export_collection",
  "get_agents",
  "get_app_runtime_info",
  "get_collection_detail",
  "get_collections",
  "get_pending_ai_tag_reviews",
  "get_project_skills",
  "get_scan_directories",
  "get_settings",
  "get_skill_explanation",
  "get_skill_repositories",
  "get_skill_tags",
  "list_projects",
  "list_projects_using_skill",
  "list_registries",
  "pick_project_folder",
  "preview_delete_central_skills",
  "preview_delete_skill_repository",
  "read_skills_sh_file",
  "record_frontend_runtime_log",
  "refresh_skill_explanation",
  "rename_project",
  "rescan_project",
  "resolve_skills_sh_url",
  "search_marketplace_skills",
  "search_skills_sh",
  "set_project_pinned",
  "set_scan_directory_active",
  "set_settings",
  "set_skill_repository_pinned",
  "skip_ai_tag_review",
  "update_collection",
];
