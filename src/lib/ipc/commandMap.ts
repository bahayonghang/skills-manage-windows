import type {
  AgentWithStatus,
  ArchiveFingerprint,
  BatchUninstallSkillRequest,
  BatchUninstallSkillResult,
  BootstrapSnapshot,
  CentralTopTag,
  CreateSshTargetRequest,
  CreateWslTargetRequest,
  CustomAgentConfig,
  DailyOperationCount,
  DashboardCentralSummary,
  DirectoryTreeEntry,
  GitHubRepoRef,
  ObsidianSkill,
  ObsidianVault,
  OperationLogEntry,
  OperationLogFilter,
  OperationLogPage,
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
  SkillWithLinks,
  SshTargetTestResult,
  TagGroup,
  TargetSummary,
  TestSshTargetRequest,
  TestWslTargetRequest,
  UpdateCustomAgentConfig,
  UpdateSshTargetRequest,
  UpdateWslTargetRequest,
  WslDistributionSummary,
  WslTargetTestResult,
} from "@/types";
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

export const IPC_COMMANDS = {
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
  fetch_github_skill_markdown: command<
    {
      repo: GitHubRepoRef;
      sourcePath: string;
      previewWorkspaceId: string | null;
    },
    string
  >(),
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
    { path: string },
    { json: string; preview: SkillportStateImportPreview }
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

export type IpcCommandMap = typeof IPC_COMMANDS;

export type CommandArgs<K extends keyof IpcCommandMap> =
  IpcCommandMap[K]["args"];
export type CommandResult<K extends keyof IpcCommandMap> =
  IpcCommandMap[K]["result"];

export const TYPED_IPC_COMMAND_NAMES: readonly string[] =
  Object.keys(IPC_COMMANDS);

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
  "apply_central_repository_sync",
  "apply_central_store_location_change",
  "apply_local_remote_sync",
  "apply_skill_update_decisions",
  "assign_skill_tags",
  "assign_skills_to_repository",
  "batch_install_central_skills",
  "batch_install_collection",
  "batch_install_to_agents",
  "browse_skills_sh_directory",
  "bulk_suggest_skill_tags",
  "cancel_ai_tag_job",
  "cancel_central_skill_updates",
  "cancel_skillport_state_portability",
  "check_central_repository_sync",
  "check_central_skill_updates",
  "clear_ai_api_key",
  "clear_github_pat",
  "clear_skill_update_inventory",
  "create_collection",
  "create_or_update_skill_repository",
  "create_skill_tag",
  "delete_central_skill",
  "delete_central_skills",
  "delete_collection",
  "delete_skill_repository",
  "discard_github_repo_preview_workspace",
  "explain_skill",
  "explain_skill_stream",
  "export_collection",
  "export_skillport_state",
  "force_mirror_central_repositories",
  "force_update_central_skills",
  "get_agents",
  "get_ai_api_key_state",
  "get_app_runtime_info",
  "get_central_skill_update_states",
  "get_collection_detail",
  "get_collections",
  "get_github_pat",
  "get_pending_ai_tag_reviews",
  "get_project_skills",
  "get_scan_directories",
  "get_settings",
  "get_skill_explanation",
  "get_skill_repositories",
  "get_skill_tags",
  "get_skill_update_inventory",
  "import_collection",
  "import_github_repo_skills",
  "import_obsidian_skill_to_central",
  "import_obsidian_skill_to_platform",
  "import_skillport_state",
  "install_from_skills_sh",
  "install_marketplace_skill",
  "install_skill_to_agent",
  "install_skill_to_project",
  "keep_remote_missing_central_skills",
  "list_projects",
  "list_projects_using_skill",
  "list_registries",
  "pick_project_folder",
  "preview_central_store_location_change",
  "preview_delete_central_skills",
  "preview_delete_skill_repository",
  "preview_github_repo_import",
  "preview_local_remote_sync",
  "preview_skillport_state_import",
  "read_skills_sh_file",
  "record_frontend_runtime_log",
  "refresh_skill_explanation",
  "refresh_skill_update_inventory",
  "remove_project",
  "remove_registry",
  "remove_scan_directory",
  "remove_skill_from_collection",
  "rename_project",
  "rescan_project",
  "resolve_skills_sh_url",
  "scan_deleted_platform_copies",
  "scan_platform_duplicate_skills",
  "search_marketplace_skills",
  "search_skills_sh",
  "set_ai_api_key",
  "set_github_pat",
  "set_project_pinned",
  "set_scan_directory_active",
  "set_settings",
  "set_skill_repository_pinned",
  "skip_ai_tag_review",
  "test_ai_connection",
  "test_github_pat",
  "unassign_skill_tags",
  "uninstall_skill_from_project",
  "update_central_skills",
  "update_collection",
];
