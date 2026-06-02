import type { AiSettings } from "@/stores/settingsStore";
import type { AiTagItemStatus, AiTagJob, SkillWithLinks } from "@/types";

export interface AiTagProgressItem {
  skillId: string;
  skillName: string;
  status: AiTagItemStatus;
}

export interface AiTagRateProfile {
  concurrency: number;
  intervalMs: number;
  stopOnRateLimit: boolean;
  isSafeProfile: boolean;
}

const DEFAULT_AI_TAG_CONCURRENCY = 1;
const DEFAULT_AI_TAG_INTERVAL_MS = 4000;

export function buildAiTagProgressItems(
  aiTagJob: AiTagJob,
  skills: readonly SkillWithLinks[]
): AiTagProgressItem[] {
  const skillNameById = new Map(skills.map((skill) => [skill.id, skill.name]));

  return Object.entries(aiTagJob.items).map(([skillId, status]) => ({
    skillId,
    skillName: skillNameById.get(skillId) ?? skillId,
    status,
  }));
}

export function buildAiTagRateProfile(
  settings: Pick<
    AiSettings,
    "tagConcurrency" | "tagIntervalMs" | "tagStopOnRateLimit"
  >
): AiTagRateProfile {
  const concurrency = parsePositiveInteger(
    settings.tagConcurrency,
    DEFAULT_AI_TAG_CONCURRENCY
  );
  const intervalMs = parsePositiveInteger(
    settings.tagIntervalMs,
    DEFAULT_AI_TAG_INTERVAL_MS
  );

  return {
    concurrency,
    intervalMs,
    stopOnRateLimit: settings.tagStopOnRateLimit,
    isSafeProfile:
      concurrency <= 1 &&
      intervalMs >= DEFAULT_AI_TAG_INTERVAL_MS &&
      settings.tagStopOnRateLimit,
  };
}

function parsePositiveInteger(value: string, fallback: number): number {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}
