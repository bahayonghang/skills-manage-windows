import { useSyncExternalStore } from "react";

/**
 * 订阅 CSS media query 的匹配状态（响应式布局用）。
 *
 * - 订阅走 `change` 事件，窗口跨阈值时自动重渲染；
 * - server snapshot 取 `false`（不匹配的保守默认）；
 * - jsdom 由测试 setup 提供 matchMedia polyfill（默认 matches:false）——
 *   调用方表述查询时应让 false 落在期望的默认分支（如按"窄屏"表述）。
 */
export function useMediaQuery(query: string): boolean {
  return useSyncExternalStore(
    (onChange) => {
      const mql = window.matchMedia(query);
      mql.addEventListener("change", onChange);
      return () => mql.removeEventListener("change", onChange);
    },
    () => window.matchMedia(query).matches,
    () => false,
  );
}
