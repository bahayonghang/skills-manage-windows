# Filter dev fixture skills from GitHub import discovery - design

## Architecture and Boundaries

The change belongs in the GitHub import discovery layer:

- `src-tauri/src/services/github_import/types.rs` owns the skipped discovery
  segment list.
- `src-tauri/src/services/github_import/source.rs` owns automatic manifest
  discovery for local archive preview/import and remote SSH workspace preview.
- `src-tauri/src/services/github_import/tests.rs` owns the focused regression
  coverage for repository snapshot discovery.

No frontend filtering is needed for the core fix. The UI should stop receiving
fixture candidates for new previews/imports because the candidates are filtered
before `RemoteSkillCandidate` values are built.

## Data Flow and Contracts

Current flow:

1. GitHub import downloads or inspects a repository snapshot.
2. `discover_skill_manifests_from_paths` normalizes repository paths.
3. Discovery checks direct root and priority roots.
4. If priority discovery finds nothing, recursive fallback scans bounded
   `SKILL.md` paths.
5. `build_remote_skill_candidate` parses frontmatter and emits candidates.

Planned flow:

1. Extend the existing skipped discovery segment contract to include only
   `test`, `tests`, `fixture`, and `fixtures`.
2. Keep segment matching case-insensitive, matching the existing
   `has_skipped_discovery_segment` behavior.
3. Let the existing immediate and recursive discovery checks reject paths
   containing those segments before candidate parsing.

This preserves existing behavior for root skills, priority skill roots, agent
specific roots, explicit source subpaths outside skipped segments, and recursive
fallback outside skipped segments.

## Compatibility and Migration

This is a forward-looking import/discovery fix. It does not delete or mutate
Central Skills rows already present in the user's database. Existing polluted
rows can still be deleted by the normal Central Skills delete workflow or by a
separate cleanup task if the user approves one.

The skip list should not include `sample`, `samples`, `example`, or `examples`
in this task. Those names may be used by legitimate publishable skills in some
repositories, and the user explicitly chose the narrower test-fixture scope.

## Trade-offs

- Adding only `test`, `tests`, `fixture`, and `fixtures` solves the observed
  fixture pollution while minimizing false positives.
- Broader sample/example filtering might catch more development content, but it
  risks hiding intentionally published demo or example skills.
- Keeping the fix in discovery rather than UI keeps update checks and import
  previews aligned with Central Skills data.

## Rollback

Rollback is limited to removing the newly added skip-list segments and the
associated tests. No schema migration or data migration is involved.
