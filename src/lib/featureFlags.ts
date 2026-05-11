import { useSyncExternalStore } from "react";

/**
 * 功能开关：用于在 Central Skills 重构期间并存新旧 UX。
 *
 * 优先级（从高到低）：
 * 1. localStorage 中 `featureFlag.<name>` 的值（"on" / "off"）—— 用于运行时切换
 * 2. 环境变量 `import.meta.env.VITE_FLAG_<UPPER_NAME>` —— 启动时读取
 * 3. 内置默认值
 *
 * 在 DevTools Console 中调试：
 *   window.localStorage.setItem("featureFlag.central.newLayout", "on")
 *   window.dispatchEvent(new Event("feature-flag-change"))
 */

export type FeatureFlagName = "central.newLayout";

const DEFAULTS: Record<FeatureFlagName, boolean> = {
  // M6（Beta → GA）：默认开启 V2 布局，用户仍可通过 Beta 徽章旁的
  // "Switch to classic" 链接（CentralSkillsShellV2.onSwitchToClassic）
  // 或在 DevTools 中显式设为 "off" 切回 V1。
  "central.newLayout": true,
};

const ENV_KEY: Record<FeatureFlagName, string> = {
  "central.newLayout": "VITE_FLAG_NEW_CENTRAL",
};

const STORAGE_PREFIX = "featureFlag.";
const FLAG_CHANGE_EVENT = "feature-flag-change";

function getStorageKey(name: FeatureFlagName): string {
  return `${STORAGE_PREFIX}${name}`;
}

function parseBooleanValue(raw: string | undefined | null): boolean | null {
  if (raw === undefined || raw === null) return null;
  const value = raw.trim().toLowerCase();
  if (value === "1" || value === "true" || value === "on" || value === "yes") return true;
  if (value === "0" || value === "false" || value === "off" || value === "no") return false;
  return null;
}

function readEnvFlag(name: FeatureFlagName): boolean | null {
  const envName = ENV_KEY[name];
  const env = (import.meta as unknown as { env?: Record<string, string | undefined> }).env;
  return parseBooleanValue(env?.[envName]);
}

function readStorageFlag(name: FeatureFlagName): boolean | null {
  if (typeof window === "undefined") return null;
  try {
    return parseBooleanValue(window.localStorage.getItem(getStorageKey(name)));
  } catch {
    return null;
  }
}

/** 同步读取一次性 flag 值。useFeatureFlag 是更推荐的 React hook 入口。 */
export function getFeatureFlag(name: FeatureFlagName): boolean {
  const fromStorage = readStorageFlag(name);
  if (fromStorage !== null) return fromStorage;
  const fromEnv = readEnvFlag(name);
  if (fromEnv !== null) return fromEnv;
  return DEFAULTS[name];
}

/** 设置 flag。传 null 清除 localStorage 覆盖，回退到 env / 默认。 */
export function setFeatureFlag(name: FeatureFlagName, value: boolean | null): void {
  if (typeof window === "undefined") return;
  try {
    if (value === null) {
      window.localStorage.removeItem(getStorageKey(name));
    } else {
      window.localStorage.setItem(getStorageKey(name), value ? "on" : "off");
    }
    window.dispatchEvent(new CustomEvent(FLAG_CHANGE_EVENT, { detail: { name } }));
  } catch {
    // ignore quota / privacy errors
  }
}

function subscribe(name: FeatureFlagName, callback: () => void): () => void {
  function handleStorage(event: StorageEvent) {
    if (event.key === null || event.key === getStorageKey(name)) {
      callback();
    }
  }
  window.addEventListener(FLAG_CHANGE_EVENT, callback);
  window.addEventListener("storage", handleStorage);
  return () => {
    window.removeEventListener(FLAG_CHANGE_EVENT, callback);
    window.removeEventListener("storage", handleStorage);
  };
}

/**
 * React hook：订阅 flag 变化。
 *
 * 用 `useSyncExternalStore` 订阅 localStorage 与同会话 dispatch 事件，
 * 同步取值避免 tearing 与 act() 警告。
 */
export function useFeatureFlag(name: FeatureFlagName): boolean {
  return useSyncExternalStore(
    (callback) => subscribe(name, callback),
    () => getFeatureFlag(name),
    () => DEFAULTS[name]
  );
}
