/**
 * 前端平台登记表：Universal Agents 虚拟分组与默认可见平台的唯一登记点。
 *
 * 新增平台需要加入 Universal 分组或默认启用列表时，只改本文件；
 * 三份有序列表（global / project / install）由登记表推导，
 * 行为由 src/test/lib/platformRegistry.test.ts 锁定。
 */

export interface UniversalPlatformRegistration {
  id: string;
  /** 是否属于全局 Universal 分组（false = 仅项目场景成组） */
  globalGroup: boolean;
  /** install 代表选择偏好序（1 最优先）；undefined = 不作候选 */
  installPreference?: number;
}

/** 一行一个 universal 成员，数组顺序即展示顺序（project 场景全集）。 */
const UNIVERSAL_PLATFORM_REGISTRY_ENTRIES = [
  { id: "amp", globalGroup: true, installPreference: 7 },
  { id: "antigravity", globalGroup: false, installPreference: 4 },
  { id: "antigravity-cli", globalGroup: false, installPreference: 3 },
  { id: "cline", globalGroup: true },
  { id: "codex", globalGroup: true, installPreference: 1 },
  { id: "cursor", globalGroup: true, installPreference: 6 },
  { id: "deep-agents", globalGroup: true },
  { id: "firebender", globalGroup: true },
  { id: "gemini-cli", globalGroup: false, installPreference: 5 },
  { id: "copilot", globalGroup: true },
  { id: "kimi-code-cli", globalGroup: true },
  { id: "opencode", globalGroup: true, installPreference: 2 },
  { id: "warp", globalGroup: true },
] as const satisfies readonly UniversalPlatformRegistration[];

export const UNIVERSAL_PLATFORM_REGISTRY: readonly UniversalPlatformRegistration[] =
  UNIVERSAL_PLATFORM_REGISTRY_ENTRIES;

/** 项目场景 Universal 成员顺序 = 登记表顺序全集。 */
export const UNIVERSAL_PROJECT_AGENT_ID_ORDER: readonly string[] =
  UNIVERSAL_PLATFORM_REGISTRY.map((entry) => entry.id);

/** 全局场景 Universal 成员顺序 = 登记表顺序过滤 globalGroup。 */
export const UNIVERSAL_AGENT_ID_ORDER: readonly string[] =
  UNIVERSAL_PLATFORM_REGISTRY.filter((entry) => entry.globalGroup).map(
    (entry) => entry.id,
  );

/** install 代表偏好序 = 按 installPreference 升序。 */
export const UNIVERSAL_INSTALL_AGENT_ORDER: readonly string[] =
  UNIVERSAL_PLATFORM_REGISTRY.flatMap((entry) =>
    entry.installPreference === undefined
      ? []
      : [{ id: entry.id, preference: entry.installPreference }],
  )
    .sort((left, right) => left.preference - right.preference)
    .map((entry) => entry.id);

/** 默认启用平台（与 Universal 分组正交，同属新增平台登记项）。 */
export const DEFAULT_ENABLED_PLATFORM_IDS = [
  "claude-code",
  "codex",
  "grok",
  "antigravity",
  "antigravity-cli",
  "opencode",
  "kiro",
] as const;
