import { useEffect, useMemo, useState } from "react";

import { invoke, isTauriRuntime } from "@/lib/tauri";
import type { SkillExplanationSummaryMap } from "@/types/skillExplanation";

function normalizeSkillIds(skillIds: readonly (string | null | undefined)[]): string[] {
  const seen = new Set<string>();
  const normalized: string[] = [];

  for (const skillId of skillIds) {
    const trimmed = skillId?.trim();
    if (!trimmed || seen.has(trimmed)) {
      continue;
    }
    seen.add(trimmed);
    normalized.push(trimmed);
  }

  return normalized;
}

export function useSkillExplanationSummaries(
  skillIds: readonly (string | null | undefined)[],
  lang = "zh"
): SkillExplanationSummaryMap {
  const [summaries, setSummaries] = useState<SkillExplanationSummaryMap>({});
  const normalizedLang = lang.trim() || "zh";
  const normalizedSkillIds = useMemo(() => normalizeSkillIds(skillIds), [skillIds]);
  const skillIdsKey = useMemo(() => normalizedSkillIds.join("\0"), [normalizedSkillIds]);

  useEffect(() => {
    let cancelled = false;

    if (!isTauriRuntime() || normalizedSkillIds.length === 0) {
      setSummaries({});
      return;
    }

    void Promise.resolve(
      invoke<SkillExplanationSummaryMap>("get_skill_explanation_summaries", {
        skillIds: normalizedSkillIds,
        lang: normalizedLang,
      })
    )
      .then((result) => {
        if (!cancelled) {
          setSummaries(result ?? {});
        }
      })
      .catch(() => {
        if (!cancelled) {
          setSummaries({});
        }
      });

    return () => {
      cancelled = true;
    };
    // reason: `skillIdsKey` is the stable dependency for content; the array is
    // rebuilt from it to keep IPC args deduplicated and in visible-list order.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [skillIdsKey, normalizedLang]);

  return summaries;
}
