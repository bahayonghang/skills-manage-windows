import { registerIpcFixtures } from "@/lib/ipc";

/**
 * 设置键值 + 杂项命令的浏览器 fixture：
 * - get_setting 一律返回 null（各调用方自行回落默认值，与旧 store 分支语义一致）
 * - set_setting 静默成功（偏好写入在演示态不落盘）
 */
export function registerSettingsFixtures(): void {
  registerIpcFixtures({
    get_setting: () => null,
    set_setting: () => undefined,
  });
}

export function registerMiscFixtures(): void {
  registerIpcFixtures({
    get_skill_explanation_summaries: () => ({}),
    open_obsidian_path: () => undefined,
  });
}
