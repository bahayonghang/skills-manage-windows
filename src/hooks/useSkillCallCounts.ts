import { useEffect, useRef, useState } from "react";

import { invoke, isTauriRuntime } from "@/lib/tauri";
import { useTargetStore } from "@/stores/targetStore";

/**
 * 给 PlatformView / CentralSkillsView 卡片注入「近 N 天 K 次」badge。
 *
 * 调用方传入要查的 skill 名列表（去重已交由后端处理），返回
 * `{ skill_name → count }` 的 map。返回 undefined 时表示尚在加载或失败，
 * 调用方应当不渲染 badge（与 count=0 行为一致）。
 *
 * 5 分钟模块级缓存（与后端的 5 分钟扫描缓存对齐）：相同
 * (activeTargetId, sortedNames, days) 重复调用直接命中，避免每次切换 Filter
 * 都打一次 IPC，同时确保 Local/SSH/WSL 之间不会复用徽章计数。
 *
 * 列表变化时自动重拉；isTauriRuntime=false 时直接返回空 map。
 */

const CACHE_TTL_MS = 5 * 60 * 1000;

type CacheEntry = { ts: number; data: Record<string, number> };
const cache = new Map<string, CacheEntry>();

function cacheKey(targetId: string, names: string[], days: number): string {
  return `${targetId}::${days}::${[...names].sort().join("|")}`;
}

export function useSkillCallCounts(
  skillNames: string[],
  days = 30
): Record<string, number> {
  const [counts, setCounts] = useState<Record<string, number>>({});
  const activeTargetId = useTargetStore((s) => s.activeTarget.id);

  // 把 names 序列化为 stable string，避免每次 render 都触发 effect
  const cacheId = cacheKey(activeTargetId, skillNames, days);
  const skillNamesRef = useRef(skillNames);

  useEffect(() => {
    skillNamesRef.current = skillNames;
  }, [cacheId, skillNames]);

  useEffect(() => {
    let cancelled = false;
    const requestSkillNames = skillNamesRef.current;

    if (requestSkillNames.length === 0) {
      setCounts({});
      return;
    }

    if (!isTauriRuntime()) {
      setCounts({});
      return;
    }

    // 缓存命中
    const hit = cache.get(cacheId);
    if (hit && Date.now() - hit.ts < CACHE_TTL_MS) {
      setCounts(hit.data);
      return;
    }

    void (async () => {
      try {
        const data = await invoke<Record<string, number>>("usage_get_skill_counts", {
          skills: requestSkillNames,
          days,
        });
        if (cancelled) return;
        cache.set(cacheId, { ts: Date.now(), data });
        setCounts(data);
      } catch {
        // 静默失败 —— badge 注入是辅助信号，不应阻塞主页面
        if (!cancelled) setCounts({});
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cacheId, days]);

  return counts;
}

/** 测试与显式刷新场景使用，让外部可以主动 evict 缓存。 */
export function clearSkillCallCountsCache(): void {
  cache.clear();
}
