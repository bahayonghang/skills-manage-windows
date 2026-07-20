import type { SkillTag } from "@/types";

export const UNCATEGORIZED_TAG_ID = "uncategorized";
export const UPDATE_TAG_FILTER_ID = "updates";
export const AI_REVIEW_TAG_FILTER_ID = "ai-review";

const SYSTEM_TAG_IDS = new Set([UNCATEGORIZED_TAG_ID]);
const SPECIAL_TAG_FILTER_IDS = new Set([
  UNCATEGORIZED_TAG_ID,
  UPDATE_TAG_FILTER_ID,
  AI_REVIEW_TAG_FILTER_ID,
]);

export function isSystemTagId(tagId: string): boolean {
  return SYSTEM_TAG_IDS.has(tagId);
}

export function isSpecialTagFilterId(tagId: string): boolean {
  return SPECIAL_TAG_FILTER_IDS.has(tagId);
}

export function getVisibleSkillTags<T extends Pick<SkillTag, "id">>(
  tags: readonly T[],
): T[] {
  return tags.filter((tag) => !isSystemTagId(tag.id));
}

export function getVisibleSkillTagsWithUsage<
  T extends Pick<SkillTag, "id" | "is_builtin">,
>(tags: readonly T[], counts: Readonly<Record<string, number>>): T[] {
  return getVisibleSkillTags(tags).filter(
    (tag) => !tag.is_builtin || (counts[tag.id] ?? 0) > 0,
  );
}

export function sanitizeSelectedTagIds(
  selectedIds: readonly string[],
  tags: readonly Pick<SkillTag, "id">[],
): string[] {
  const knownTagIds = new Set(tags.map((tag) => tag.id));
  return selectedIds.filter(
    (id) => isSpecialTagFilterId(id) || knownTagIds.has(id),
  );
}
