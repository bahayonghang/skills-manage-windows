import type {
  SkillRefreshContext,
  SkillRefreshMode,
  SkillRefreshScope,
  SkillRefreshScopeKind,
} from "@/types/skillUpdateInventory";
import { normalizeUpdateCheckMode } from "@/lib/updateCheckMode";

export function normalizeRefreshContext(
  context?: Partial<SkillRefreshContext> | null,
): SkillRefreshContext {
  return {
    repositoryIds: uniqueNonEmpty(context?.repositoryIds ?? []),
    skillIds: uniqueNonEmpty(context?.skillIds ?? []),
    agentIds: uniqueNonEmpty(context?.agentIds ?? []),
  };
}

export function isRefreshScopeEnabled(
  kind: SkillRefreshScopeKind,
  context: Partial<SkillRefreshContext> | null | undefined,
): boolean {
  if (kind === "all") return true;
  const normalized = normalizeRefreshContext(context);
  if (kind === "repositories") return normalized.repositoryIds.length > 0;
  if (kind === "platform") return normalized.agentIds.length > 0;
  return normalized.skillIds.length > 0;
}

export function coerceRefreshScopeKind(
  kind: SkillRefreshScopeKind,
  context: Partial<SkillRefreshContext> | null | undefined,
): SkillRefreshScopeKind {
  return isRefreshScopeEnabled(kind, context) ? kind : "all";
}

export function buildRefreshScope(
  kind: SkillRefreshScopeKind,
  context: Partial<SkillRefreshContext> | null | undefined,
  mode: SkillRefreshMode = "sync",
): SkillRefreshScope {
  const normalized = normalizeRefreshContext(context);
  const effectiveKind = coerceRefreshScopeKind(kind, normalized);
  const normalizedMode = normalizeUpdateCheckMode(mode);
  if (effectiveKind === "repositories") {
    return {
      kind: "repositories",
      mode: normalizedMode,
      repositoryIds: normalized.repositoryIds,
    };
  }
  if (effectiveKind === "skills") {
    return { kind: "skills", mode: normalizedMode, skillIds: normalized.skillIds };
  }
  if (effectiveKind === "platform") {
    return { kind: "platform", mode: normalizedMode, agentIds: normalized.agentIds };
  }
  return { kind: "all", mode: normalizedMode };
}

function uniqueNonEmpty(values: readonly string[]): string[] {
  return Array.from(
    new Set(values.map((value) => value.trim()).filter(Boolean)),
  );
}
