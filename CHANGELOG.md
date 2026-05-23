# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- **Update Center**: a unified dialog that aggregates updatable skills, remote additions, remote deletions, and platform duplicates into one screen with per-item decisions. Replaces the standalone *Check for updates*, *Remote missing*, and *Repository sync* dialogs. See `docs/guide/update-center.md`.
- New Tauri commands `refresh_skill_update_inventory`, `apply_skill_update_decisions`, `clear_skill_update_inventory`, `get_skill_update_inventory`, `scan_platform_duplicate_skills`.
- New DB table `skill_repository_pending_additions` and column `skill_repositories.last_synced_at` to persist refresh results across sessions.
- Backend `SkillUpdateStatus` enum replaces the previous string constants.

### Changed

- PlatformView's *Scan duplicates* button now opens the Update Center on the *Platform duplicates* tab instead of a standalone dialog.
- `UnifiedSkillCard` shows additional badges for inventory-only signals (platform duplicate, orphan).
- `DuplicatePlatformSkillsDialog`, `CentralUpdateConfirmDialog`, `RemoteMissingSkillsDialog`, `CentralRepositorySyncDialog` are marked `@deprecated` and will be removed after one minor release.

## 0.11.0 - 2026-05-11

Security and Central Library V2 release for the 0.10.x → 0.11.0 line. This release keeps the Windows-first packaging contract unchanged while making Central Skills the default high-density workspace.

### Features

- Make Central Library V2 the default layout, with a classic-layout fallback link and `featureFlag.central.newLayout=off` override for troubleshooting.
- Add structured Central search syntax for `tag:`, `repo:`, `owner:`, `source:`, `has:`, `platform:`, `created:`, and `updated:` filters while preserving free-text search.
- Add URL-backed Central view state for query text, multi-select facets, sorting, grouping, view mode, and saved-view identity.
- Add local Saved Views for reusable Central queries, plus sidebar and command-palette entry points.
- Add Tag Groups and sidebar assignment so large tag sets can be grouped without moving skill files.
- Add group-by views for repository, owner, tag, and update status.
- Add Central V2 command-palette actions for saving the current view, creating tag groups, changing group-by mode, and switching back to the classic layout.

### Security

- Move GitHub PAT storage from plaintext SQLite settings into the shared SecretStore path with keyring, Windows DPAPI, and session fallback support.
- Move AI API Key storage into SecretStore while keeping non-sensitive AI provider, region, model, URL, and tagging options in normal settings.
- Add guarded one-time migrations for legacy `github_pat` and `ai_api_key` settings rows: old values are cleared only after the secret is written and read back successfully.
- Block generic settings IPC from reading or writing protected secret keys so new plaintext writes cannot be reintroduced through `get_setting`, `set_setting`, `get_settings`, or `set_settings`.

### Improvements

- Route the remaining Settings store actions through the shared `@/lib/tauri` wrapper so browser fixtures and Tauri runtime checks stay consistent across stores.
- Replace direct `serde_yaml` usage with `serde_norway` for SKILL.md frontmatter parsing after confirming the planned `serde_yml` alternative is itself RustSec-flagged.
- Harden the operation-log warning test so concurrent Rust test runs no longer depend on the old flaky SQL error path.
- Extend the Central Skills guide and README with V2 search, Saved Views, Tag Groups, group-by, command-palette, and rollback guidance.
- Keep the Windows packaging contract unchanged; no installer, signing, or bundle output path changes are included in this release.

### Known gaps

- Drag-and-drop reorder UI for Saved Views and Tag Groups is deferred; the backend IPC and store actions are already present.
- Saved View and Tag Group management still use lightweight prompt/confirm flows until a dedicated dialog primitive is introduced.
- Backend FTS5 search remains deferred until library size or p95 filter latency justifies it.
## 0.10.0 - 2026-05-03

Alignment release focused on upstream 0.10.0 packaging parity, Linux desktop bundles, and safer Discover installs for this Windows-first SkillPort fork.

### Features

- Add Linux desktop bundle metadata, templates, and GitHub Actions jobs for `.deb`, `.rpm`, and `.AppImage` artifacts under the SkillPort identity.
- Extend Discover platform installs so the existing install dialog now forwards `symlink` or `copy` mode all the way to the Rust backend.
- Restore Windows release artifacts to include NSIS `.exe`, MSI, and portable ZIP assets alongside macOS and Linux bundles.

### Improvements

- Add cross-platform bundle metadata such as publisher, homepage, license file, descriptions, and AppStream integration to the Tauri config.
- Deduplicate shared Discover platform directory patterns like `.agents/skills` so scans stop producing repeated platform matches for the same project path.
- Add release notes for the 0.10.0 line and refresh README download guidance for the expanded desktop package matrix.

### Fixes

- Delete tracked `.factory/` factory artifacts and ignore future `.factory/` output without hiding `AGENTS.md`.
- Keep Linux packaging branded as `SkillPort` / `skillport` / `com.bahayonghang.skillport` instead of reverting to upstream identifiers.

## 0.9.1 - 2026-04-23

Maintenance release focused on full-path display consistency and small README polish.

### Fixes

- Show full absolute paths in Central, Platform, Settings, Global Search, and platform-edit flows instead of collapsing paths to `~`.
- Render Windows paths with drive letters and backslashes in display-oriented UI surfaces.
- Keep auto-generated custom platform paths aligned with the detected home-directory style on each platform.

### Improvements

- Add a `Star History` section to the English and Chinese READMEs.
- Extend path helper tests and affected UI assertions to cover the new display rules.

## 0.9.0 - 2026-04-23

Cross-platform merge release focused on upstream 0.9.0 compatibility while keeping this fork's Windows installer contract.

### Features

- Merge upstream 0.9.0 Windows and macOS packaging work so release builds can produce Windows NSIS, Windows MSI, Windows ZIP, and macOS universal DMG/ZIP/TAR.GZ assets.
- Add source-aware Claude rows, detail loading, and explanation continuity across platform lists, drawers, and refresh flows.
- Add Windows-friendly path rendering helpers in both backend and frontend surfaces, including home-path compaction in UI.

### Fixes

- Keep this fork's `~/.agents/skills` Windows-first path rules while absorbing upstream home expansion and cross-platform path utilities into the existing `paths.rs` module.
- Add automatic copy fallback when Windows cannot create a symlink during platform install or import flows.
- Propagate full rescans across central, platform, and discover stores so counts and row state stay aligned after refresh.
- Preserve local bootstrap hydration, platform visibility toggles, and agent enablement while extending the data model with Claude source-specific row identity.

## 0.8.2 - 2026-04-23

Patch release focused on cached shell hydration, cheaper refreshes, and smoother large skill lists.

### Performance

- Hydrate the app shell from a cached bootstrap snapshot before running a background scan so platform counts, collection counts, and Discover totals appear immediately.
- Virtualize large central and platform skill card grids, and memoize repeated card/icon rendering to reduce scroll and filter cost.

### Fixes

- Normalize Windows home and central skill directory resolution so cached scans, installs, and imports stay aligned with `~/.agents/skills`.
- Deduplicate shared scan roots across platforms and update cached installation rows in one transaction to avoid stale counts after rescans.
- Add lightweight bootstrap and discover summary endpoints so the sidebar and refresh flows stop loading full datasets when only counters are needed.
- Load central skills lazily when the platform install dialog opens instead of preloading them on every platform page visit.

## 0.8.1 - 2026-04-23

Patch release to repair the GitHub release pipeline.

### Fixes

- Update the release workflow to use the published `tauri-apps/tauri-action@action-v0.6.2` tag so Windows and macOS release jobs can start correctly.

## 0.8.0 - 2026-04-20

First public release.

### Features

- Launch `skills-manage` as a Tauri desktop app for managing AI agent skills across built-in and custom platforms from one place.
- Add platform and central skill views with install, uninstall, symlink-aware status, and canonical skill management.
- Add a full skill detail experience with markdown preview, in-place drawer navigation, install actions, and collection-aware workflows.
- Add collections management, custom platform settings, configurable scan roots, onboarding, toast feedback, and a responsive sidebar.
- Add Chinese and English UI support, a Catppuccin multi-flavor theme system, accent color controls, and a global command palette.
- Add project-level Discover scanning with recursive search, cached results, stop-scan controls, import to central, and improved navigation context.
- Add marketplace browsing, preview drawers, auto-centralized installs, and AI-generated skill explanations.
- Add GitHub repository import with preview, mirror fallback retries, optional authenticated requests, selection persistence, and post-import platform install flows.

### Performance

- Improve global search, central search, and project skill browsing with deferred queries, lazy indexing, lighter search result cards, and list virtualization for large datasets.

### Fixes

- Harden AI explanation generation by rejecting blank cached content and re-generating corrupted empty explanations.
- Improve frontmatter handling by extracting structured metadata such as `name`, `description`, and `version` instead of leaking raw YAML into markdown previews.
- Show existing collection membership in skill details and preselect already-added collections in add-to-collection flows.
- Refine detail drawer, marketplace preview, and GitHub import layouts to preserve context and reduce navigation friction.
