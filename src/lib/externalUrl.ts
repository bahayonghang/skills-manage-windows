import { open as openShellPath } from "@tauri-apps/plugin-shell";
import { isTauriRuntime } from "@/lib/tauri";

export async function openExternalUrl(rawUrl: string): Promise<void> {
  const url = new URL(rawUrl);

  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error("External URLs must use http(s).");
  }

  if (isTauriRuntime()) {
    await openShellPath(url.toString());
    return;
  }

  if (typeof window !== "undefined") {
    window.open(url.toString(), "_blank", "noopener,noreferrer");
  }
}
