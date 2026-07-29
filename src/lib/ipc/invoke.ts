import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  listen as tauriListen,
  type EventCallback,
  type EventName,
  type Options,
  type UnlistenFn,
} from "@tauri-apps/api/event";
import type { CommandArgs, CommandResult, IpcCommandMap } from "./commandMap";
import { normalizeIpcRejection, sanitizeIpcFailureArgs } from "./errors";
import { dispatchIpcFixture } from "./fixtures";
import { isTauriRuntime } from "./runtime";

type IpcFailureRecorder = (
  command: string,
  args: unknown,
  error: unknown,
) => void;

let ipcFailureRecorder: IpcFailureRecorder | null = null;

export function registerIpcFailureRecorder(
  recorder: IpcFailureRecorder | null,
): void {
  ipcFailureRecorder = recorder;
}

type TauriInvokeArgs = Parameters<typeof tauriInvoke>[1];

function dispatch<T>(command: string, args: unknown): Promise<T> {
  if (!isTauriRuntime()) {
    return dispatchIpcFixture<T>(command, args);
  }
  return args === undefined
    ? tauriInvoke<T>(command)
    : tauriInvoke<T>(command, args as TauriInvokeArgs);
}

/**
 * 全仓唯一 IPC 入口（例外：runtimeLogger 走 invokeRaw，见下）。
 * 命令名 ∈ IPC_COMMANDS 时按名推导 args/result；未类型化命令走第二个
 * overload（`invoke<T>("cmd", args)`），入 map 后删掉显式泛型即获得类型。
 * Tauri 运行时直连后端；浏览器演示态路由到 fixture 注册表。
 */
export function invoke<K extends keyof IpcCommandMap>(
  command: K,
  ...args: CommandArgs<K> extends undefined ? [] : [CommandArgs<K>]
): Promise<CommandResult<K>>;
export function invoke<T = unknown>(
  command: string,
  args?: unknown,
): Promise<T>;
export function invoke<T>(command: string, args?: unknown): Promise<T> {
  const result = dispatch<T>(command, args);

  // 测试里 invoke 可能被 mock 成返回 undefined 的 vi.fn，需守住 .catch 可用
  if (!result || typeof result.catch !== "function") {
    return result;
  }

  return result.catch((error) => {
    const normalized = normalizeIpcRejection(error);
    if (command !== "record_frontend_runtime_log") {
      ipcFailureRecorder?.(
        command,
        sanitizeIpcFailureArgs(args),
        normalized,
      );
    }
    throw normalized;
  });
}

/**
 * 裸通道：绕过 fixture 路由与 failure recorder，直连 Tauri。
 * 仅供 runtimeLogger 日志自举使用（recorder 落盘自身失败会递归）。
 */
export function invokeRaw<T>(command: string, args?: unknown): Promise<T> {
  if (args === undefined) {
    return tauriInvoke<T>(command);
  }
  return tauriInvoke<T>(command, args as TauriInvokeArgs);
}

/** 浏览器演示态返回 no-op unlisten，调用方无需再包 isTauriRuntime guard。 */
export function listen<T>(
  event: EventName,
  handler: EventCallback<T>,
  options?: Options,
): Promise<UnlistenFn> {
  if (!isTauriRuntime()) {
    return Promise.resolve(() => undefined);
  }
  return tauriListen(event, handler, options);
}
