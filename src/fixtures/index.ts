import { registerMiscFixtures, registerSettingsFixtures } from "./settings";
import { registerObsidianFixtures } from "./obsidian";
import { registerOperationLogFixtures } from "./operationLogs";
import { registerPlatformFixtures } from "./platform";
import { registerRuntimeLogFixtures } from "./runtimeLogs";
import { registerSavedViewFixtures } from "./savedViews";
import { registerSkillsFixtures } from "./skills";
import { registerTagGroupFixtures } from "./tagGroups";
import { registerTargetFixtures } from "./targets";
import { registerUsageFixtures } from "./usage";

/**
 * 浏览器演示态的 IPC fixture 总装：按命令名注册到 @/lib/ipc 的 fixture 侧。
 * main.tsx 在 !isTauriRuntime() 时调用（渲染前，静态 import 保证注册先于任何
 * store initialize）；Tauri 运行时不调用，真实 invoke 不受影响。
 * 未注册命令在演示态会 reject IpcFixtureMissingError（fail loud，缺口可定位）。
 */
export function installBrowserIpcFixtures(): void {
  registerSettingsFixtures();
  registerMiscFixtures();
  registerPlatformFixtures();
  registerSkillsFixtures();
  registerObsidianFixtures();
  registerUsageFixtures();
  registerTargetFixtures();
  registerOperationLogFixtures();
  registerRuntimeLogFixtures();
  registerTagGroupFixtures();
  registerSavedViewFixtures();
}
