## User problem

<!-- What user or maintainer problem does this change solve? -->

## Scope

### In scope

<!-- List the behavior and files intentionally changed. -->

### Out of scope

<!-- List adjacent work that is deliberately excluded. Use `N/A` when there is none. -->

## Risk and rollback

<!-- Describe the main failure mode, data or compatibility risk, and the smallest rollback. -->

## Validation

<!-- List commands and focused tests that were actually run. Use `N/A` with a reason when a check does not apply. -->

- [ ] `just doctor` (when toolchain or environment behavior changes)
- [ ] Focused tests
- [ ] `just check` (fast feedback only)
- [ ] `just ci` (required before merge)
- [ ] `just audit` (required before merge)

## UI evidence

<!-- For UI changes, attach screenshots or a recording and describe the affected states. Use `N/A` with a reason otherwise. -->

## Packaging and release impact

<!-- State whether installers, updater metadata, signing, or release workflows are affected. Use `N/A` with a reason otherwise. -->

## Generated files

<!-- If Rust commands or schema changed, mention `pnpm docs:gen` and the generated files reviewed. Use `N/A` otherwise. -->

## Merge path

<!-- Task PRs target `dev` and use squash merge; do not use this section to request a promotion. -->
<!-- Promotion PRs target `main`, use a merge commit, and require an exact-head check immediately before merge. -->

- [ ] This is a task PR into `dev` (squash merge)
- [ ] This is a `dev` -> `main` promotion PR (merge commit)
- [ ] After promotion, `dev` will be fast-forwarded to the promotion merge SHA before Trellis bookkeeping or the next task
