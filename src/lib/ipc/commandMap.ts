import type {
  DirectoryTreeEntry,
  ObsidianSkill,
  ObsidianVault,
  SkillDetail,
  SkillDetailRequest,
} from "@/types";

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
  get_skill_detail: command<SkillDetailRequest, SkillDetail>(),
  read_file_by_path: command<SkillPathRequest, string>(),
  list_directory_tree: command<SkillPathRequest, DirectoryTreeEntry[]>(),
  open_in_file_manager: command<SkillPathRequest, void>(),
  open_obsidian_path: command<{ path: string }, void>(),
  get_obsidian_vaults: command<undefined, ObsidianVault[]>(),
  get_obsidian_vault_skills: command<{ vaultId: string }, ObsidianSkill[]>(),
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
