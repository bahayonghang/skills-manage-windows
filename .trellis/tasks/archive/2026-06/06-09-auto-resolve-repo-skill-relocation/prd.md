# Auto-resolve moved repository skills

## Goal

When a repository moves a skill from an old source path to a new official source
path while keeping the same skill id, the Update Center should handle that case
automatically during refresh. The user should not have to manually combine an
Added row with a Removed row for the same repository skill.

Example: a repository previously tracked `teach` at `skills/in-progress/teach`
and later publishes the same `teach` skill at `skills/productivity/teach`.
Refresh should treat this as a source-path relocation, not as a separate
addition plus removal conflict.

## Requirements

- Detect repository skill relocations only when the evidence is unambiguous:
  same repository id, same skill id, different source paths, and exactly one
  matching remote-added candidate for exactly one remote-missing local skill.
- Resolve safe relocations during `refresh_skill_update_inventory` so the
  resulting inventory does not show both an Added item and a Removed item for
  the same relocated skill.
- Preserve normal manual handling for ambiguous or unsafe cases, including
  multiple candidates, cross-repository matches, different skill ids, skipped
  additions, and cases that cannot be recalculated.
- After relocation, recalculate the skill update state from the new source path:
  show it as Updatable only if the relocated remote content differs from the
  local Central skill content; otherwise keep it out of actionable tabs.
- Keep apply-stage semantics unchanged: refresh may repair source metadata and
  pending-state bookkeeping, but it must not import a new skill, overwrite
  Central files, or delete local skills automatically.
- Maintain existing Update Center abstractions and i18n conventions.

## Acceptance Criteria

- [ ] A regression test covers a skill moved from one repository source path to
      another with the same skill id, producing no Added/Removed conflict.
- [ ] If the moved skill has different remote content, refresh returns it as a
      normal `update_available` item after updating the stored source path.
- [ ] Ambiguous same-id additions do not auto-resolve and still surface for
      manual review.
- [ ] Frontend decision aggregation remains compatible with the backend output
      and does not select hidden paired Added/Removed rows.
- [ ] `just ci` passes before completion.

## Notes

- Confirmed evidence:
  - `RemoteAddedSkill` carries `repositoryId`, `sourcePath`, `skillId`, and
    `conflictExistingSkillId`.
  - `RemoteMissingSkill` carries `repositoryId`, `state.skill_id`, and
    `state.source_path`.
  - Pending additions are persisted in `skill_repository_pending_additions`.
  - Current refresh/apply semantics intentionally keep Added and Removed
    decisions independent, which creates the manual conflict for path moves.
