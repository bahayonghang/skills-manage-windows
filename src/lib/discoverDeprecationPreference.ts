import { invoke, isTauriRuntime } from "@/lib/tauri";

const SETTING_KEY = "discover_deprecation_dismissed";

export async function loadDiscoverDeprecationDismissed(): Promise<boolean> {
  if (!isTauriRuntime()) {
    return true;
  }

  const value = await invoke<string | null>("get_setting", {
    key: SETTING_KEY,
  });
  return value === "true";
}

export async function saveDiscoverDeprecationDismissed(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  await invoke("set_setting", { key: SETTING_KEY, value: "true" });
}
