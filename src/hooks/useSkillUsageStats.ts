import { useEffect, useRef, useState } from "react";

import { invoke } from "@/lib/ipc";
import { useTargetStore } from "@/stores/targetStore";
import type { SkillUsageStat } from "@/types/usage";

/**
 * 给 PlatformView 排序 / 右下角名次注入全历史（或近 N 天）次数。
 *
 * `days === null` 表示全部已记录历史。`ready === false` 表示加载中或失败，
 * 调用方不得把空 map 当成「全部零次」。
 *
 * 5 分钟模块级缓存，key = `targetId::days|all::sortedNames`。
 * 只在 hook 内 invoke。
 */

const CACHE_TTL_MS = 5 * 60 * 1000;

type CacheEntry = { ts: number; data: Record<string, SkillUsageStat> };
const cache = new Map<string, CacheEntry>();

function cacheKey(
  targetId: string,
  names: string[],
  days: number | null,
): string {
  const window = days == null ? "all" : String(days);
  return `${targetId}::${window}::${[...names].sort().join("|")}`;
}

export function useSkillUsageStats(
  skillNames: string[],
  options: { days: number | null },
): { stats: Record<string, SkillUsageStat>; ready: boolean } {
  const [stats, setStats] = useState<Record<string, SkillUsageStat>>({});
  const [ready, setReady] = useState(false);
  const activeTargetId = useTargetStore((s) => s.activeTarget.id);
  const { days } = options;

  const cacheId = cacheKey(activeTargetId, skillNames, days);
  const skillNamesRef = useRef(skillNames);

  useEffect(() => {
    skillNamesRef.current = skillNames;
  }, [cacheId, skillNames]);

  useEffect(() => {
    let cancelled = false;
    const requestSkillNames = skillNamesRef.current;

    if (requestSkillNames.length === 0) {
      setStats({});
      setReady(true);
      return;
    }

    const hit = cache.get(cacheId);
    if (hit && Date.now() - hit.ts < CACHE_TTL_MS) {
      setStats(hit.data);
      setReady(true);
      return;
    }

    setReady(false);

    void (async () => {
      try {
        const data = await invoke("usage_get_skill_usage_stats", {
          skills: requestSkillNames,
          days,
        });
        if (cancelled) return;
        cache.set(cacheId, { ts: Date.now(), data });
        setStats(data);
        setReady(true);
      } catch {
        if (!cancelled) {
          setStats({});
          setReady(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [cacheId, days]);

  return { stats, ready };
}

/** 测试与显式刷新场景使用，让外部可以主动 evict 缓存。 */
export function clearSkillUsageStatsCache(): void {
  cache.clear();
}
