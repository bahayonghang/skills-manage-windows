import { getCurrentWindow } from "@tauri-apps/api/window";

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
