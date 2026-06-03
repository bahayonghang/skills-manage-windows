import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { FrontendRuntimeLogPayload } from "@/types";

type TauriWindow = Window & {
  __TAURI__?: unknown;
  __TAURI_INTERNALS__?: unknown;
};

export function isTauriRuntime(): boolean {
  if (typeof window === "undefined") {
    return false;
  }

  const tauriWindow = window as TauriWindow;
  return Boolean(tauriWindow.__TAURI__ || tauriWindow.__TAURI_INTERNALS__);
}

type IpcFailureRecorder = (
  command: string,
  args: unknown,
  error: unknown
) => void;

let ipcFailureRecorder: IpcFailureRecorder | null = null;

export function registerIpcFailureRecorder(recorder: IpcFailureRecorder | null): void {
  ipcFailureRecorder = recorder;
}

export function invoke<T>(command: string, args?: unknown): Promise<T> {
  const result =
    args === undefined
      ? tauriInvoke<T>(command)
      : tauriInvoke<T>(command, args as Parameters<typeof tauriInvoke>[1]);

  if (
    command !== "record_frontend_runtime_log" &&
    result &&
    typeof result.catch === "function"
  ) {
    result.catch((error) => {
      ipcFailureRecorder?.(command, args, error);
    });
  }

  return result as Promise<T>;
}

export function invokeRaw<T>(command: string, args?: unknown): Promise<T> {
  if (args === undefined) {
    return tauriInvoke<T>(command);
  }
  return tauriInvoke<T>(command, args as Parameters<typeof tauriInvoke>[1]);
}

export type RuntimeLogRecorder = (payload: FrontendRuntimeLogPayload) => void;
export const listen = tauriListen;

let hasRequestedMainWindowShow = false;

export async function showMainWindowWhenReady(): Promise<void> {
  if (hasRequestedMainWindowShow || !isTauriRuntime()) {
    return;
  }

  hasRequestedMainWindowShow = true;
  await getCurrentWindow().show();
}

export function __resetMainWindowReadyForTest(): void {
  hasRequestedMainWindowShow = false;
}
