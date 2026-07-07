import { registerIpcFixtures } from "@/lib/ipc";
import {
  BROWSER_PLATFORM_PATHS,
  getPlatformSkillDir,
  getPlatformSkillFilePath,
} from "@/lib/platformPathPolicy";
import type { ScannedSkill } from "@/types";

const DESKTOP_ONLY_UNINSTALL_ERROR =
  "Uninstalling skills requires the Tauri desktop runtime.";

export const BROWSER_FIXTURE_SKILLS_BY_AGENT: Record<string, ScannedSkill[]> = {
  "claude-code": [
    {
      id: "fixture-central-skill",
      name: "fixture-central-skill",
      description:
        "Installed browser validation fixture for platform drawer flows.",
      file_path: getPlatformSkillFilePath(
        BROWSER_PLATFORM_PATHS,
        "claude-code",
        "fixture-central-skill",
      ),
      dir_path: getPlatformSkillDir(
        BROWSER_PLATFORM_PATHS,
        "claude-code",
        "fixture-central-skill",
      ),
      link_type: "symlink",
      symlink_target: getPlatformSkillDir(
        BROWSER_PLATFORM_PATHS,
        "central",
        "fixture-central-skill",
      ),
      is_central: true,
    },
  ],
  codex: [
    {
      id: "fixture-universal-skill",
      name: "fixture-universal-skill",
      description:
        "Installed browser validation fixture for Universal platform flows.",
      file_path: getPlatformSkillFilePath(
        BROWSER_PLATFORM_PATHS,
        "codex",
        "fixture-universal-skill",
      ),
      dir_path: getPlatformSkillDir(
        BROWSER_PLATFORM_PATHS,
        "codex",
        "fixture-universal-skill",
      ),
      link_type: "native",
      is_central: false,
    },
  ],
  cursor: [
    {
      id: "fixture-central-skill",
      name: "fixture-central-skill",
      description:
        "Installed browser validation fixture for platform drawer flows.",
      file_path: getPlatformSkillFilePath(
        BROWSER_PLATFORM_PATHS,
        "cursor",
        "fixture-central-skill",
      ),
      dir_path: getPlatformSkillDir(
        BROWSER_PLATFORM_PATHS,
        "cursor",
        "fixture-central-skill",
      ),
      link_type: "symlink",
      symlink_target: getPlatformSkillDir(
        BROWSER_PLATFORM_PATHS,
        "central",
        "fixture-central-skill",
      ),
      is_central: true,
    },
  ],
};

export function registerSkillsFixtures(): void {
  registerIpcFixtures({
    get_skills_by_agent: ({ agentId }) =>
      BROWSER_FIXTURE_SKILLS_BY_AGENT[agentId] ?? [],
    get_central_skills: () => [],
    // 卸载依赖真实文件系统，浏览器演示态保持「桌面运行时必需」语义：
    // reject 原字符串（Tauri 命令错误即 string），store catch 后 error 文案不变
    uninstall_skill_from_agent: () =>
      Promise.reject(DESKTOP_ONLY_UNINSTALL_ERROR),
    batch_uninstall_skills_from_agent: () =>
      Promise.reject(DESKTOP_ONLY_UNINSTALL_ERROR),
  });
}
