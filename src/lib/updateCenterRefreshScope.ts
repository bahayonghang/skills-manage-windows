import type {
  SkillRefreshContext,
  SkillRefreshScope,
  SkillRefreshScopeKind,
} from "@/types/skillUpdateInventory";

export function normalizeRefreshContext(
  context?: Partial<SkillRefreshContext> | null,
): SkillRefreshContext {
  return {
    repositoryIds: uniqueNonEmpty(context?.repositoryIds ?? []),
    skillIds: uniqueNonEmpty(context?.skillIds ?? []),
  };
}

export function isRefreshScopeEnabled(
  kind: SkillRefreshScopeKind,
  context: SkillRefreshContext | null | undefined,
): boolean {
  if (kind === "all") return true;
  const normalized = normalizeRefreshContext(context);
  if (kind === "repositories") return normalized.repositoryIds.length > 0;
  return normalized.skillIds.length > 0;
}

export function coerceRefreshScopeKind(
  kind: SkillRefreshScopeKind,
  context: SkillRefreshContext | null | undefined,
): SkillRefreshScopeKind {
  return isRefreshScopeEnabled(kind, context) ? kind : "all";
}

export function buildRefreshScope(
  kind: SkillRefreshScopeKind,
  context: SkillRefreshContext | null | undefined,
): SkillRefreshScope {
  const normalized = normalizeRefreshContext(context);
  const effectiveKind = coerceRefreshScopeKind(kind, normalized);
  if (effectiveKind === "repositories") {
    return { kind: "repositories", repositoryIds: normalized.repositoryIds };
  }
  if (effectiveKind === "skills") {
    return { kind: "skills", skillIds: normalized.skillIds };
  }
  return { kind: "all" };
}

function uniqueNonEmpty(values: readonly string[]): string[] {
  return Array.from(
    new Set(values.map((value) => value.trim()).filter(Boolean)),
  );
}
