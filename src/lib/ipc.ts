import type {
  DirectoryTreeEntry,
  ObsidianSkill,
  ObsidianVault,
  SkillDetail,
  SkillDetailRequest,
} from "@/types";
import { invoke } from "@/lib/tauri";

export interface SkillPathRequest extends SkillDetailRequest {
  path: string;
}

type IpcCommandSpec<Args, Result> = {
  args: Args;
  result: Result;
};

interface IpcCommandMap {
  get_skill_detail: IpcCommandSpec<SkillDetailRequest, SkillDetail>;
  read_file_by_path: IpcCommandSpec<SkillPathRequest, string>;
  list_directory_tree: IpcCommandSpec<SkillPathRequest, DirectoryTreeEntry[]>;
  open_in_file_manager: IpcCommandSpec<SkillPathRequest, void>;
  open_obsidian_path: IpcCommandSpec<{ path: string }, void>;
  get_obsidian_vaults: IpcCommandSpec<undefined, ObsidianVault[]>;
  get_obsidian_vault_skills: IpcCommandSpec<{ vaultId: string }, ObsidianSkill[]>;
}

type CommandArgs<K extends keyof IpcCommandMap> = IpcCommandMap[K]["args"];
type CommandResult<K extends keyof IpcCommandMap> = IpcCommandMap[K]["result"];

export function invokeCommand<K extends keyof IpcCommandMap>(
  command: K,
  ...args: CommandArgs<K> extends undefined ? [] : [CommandArgs<K>]
): Promise<CommandResult<K>> {
  return args.length === 0
    ? invoke<CommandResult<K>>(command)
    : invoke<CommandResult<K>>(command, args[0]);
}
