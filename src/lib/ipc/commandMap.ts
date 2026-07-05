import type {
  AgentWithStatus,
  BatchUninstallSkillRequest,
  BatchUninstallSkillResult,
  BootstrapSnapshot,
  CreateSshTargetRequest,
  CreateWslTargetRequest,
  CustomAgentConfig,
  DirectoryTreeEntry,
  ObsidianSkill,
  ObsidianVault,
  OperationLogEntry,
  OperationLogFilter,
  OperationLogPage,
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
  SkillCall,
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
  // ── platform skills ───────────────────────────────────────────────────────
  get_skills_by_agent: command<{ agentId: string }, ScannedSkill[]>(),
  get_central_skills: command<undefined, SkillWithLinks[]>(),
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
    SkillCall[]
  >(),
  usage_get_providers: command<undefined, ProviderHealth[]>(),
  usage_get_skill_detail: command<{ skill: string }, SkillUsageDetail | null>(),
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
export const UNTYPED_IPC_COMMANDS: readonly string[] = [];
