import { useEffect, useMemo, useRef, useState } from "react";

import { invoke } from "@/lib/ipc";
import type { SkillExplanationSummaryMap } from "@/types/skillExplanation";

function normalizeSkillIds(
  skillIds: readonly (string | null | undefined)[],
): string[] {
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
  lang = "zh",
): SkillExplanationSummaryMap {
  const [summaries, setSummaries] = useState<SkillExplanationSummaryMap>({});
  const normalizedLang = lang.trim() || "zh";
  const normalizedSkillIds = useMemo(
    () => normalizeSkillIds(skillIds),
    [skillIds],
  );
  const skillIdsKey = useMemo(
    () => normalizedSkillIds.join("\0"),
    [normalizedSkillIds],
  );
  const normalizedSkillIdsRef = useRef(normalizedSkillIds);

  useEffect(() => {
    normalizedSkillIdsRef.current = normalizedSkillIds;
  }, [skillIdsKey, normalizedSkillIds]);

  useEffect(() => {
    let cancelled = false;
    const requestSkillIds = normalizedSkillIdsRef.current;

    if (requestSkillIds.length === 0) {
      setSummaries({});
      return;
    }

    void Promise.resolve(
      invoke("get_skill_explanation_summaries", {
        skillIds: requestSkillIds,
        lang: normalizedLang,
      }),
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
  }, [skillIdsKey, normalizedLang]);

  return summaries;
}
