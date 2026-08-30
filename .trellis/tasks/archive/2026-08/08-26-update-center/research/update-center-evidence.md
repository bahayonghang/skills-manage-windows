# Update Center planning evidence

Date: 2026-08-26. This file records repository and primary-source facts used by this task.
It is not runtime or native-Windows verification.

## Verified repository facts

- `src-tauri/src/db/migrations/versions/mod.rs` currently declares six contiguous descriptors,
  v1 through v6. A new schema belongs in an appended immutable v7 descriptor; editing v1–v6 would
  violate `.trellis/spec/backend/database-migrations.md`.
- `src-tauri/src/commands/central_updates.rs` and
  `src-tauri/src/commands/skill_update_inventory.rs` load GitHub auth with
  `github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())` and build
  the shared client with `github_import::github_client()` before injecting both into services.
  `services/central_updates/repository_sync.rs` is a consumer, not the SecretStore/client factory.
- `src-tauri/src/services/github_import` already exposes crate-local source normalization, repository
  keys, full commit resolution, bounded pinned snapshot acquisition and per-candidate content digest
  helpers. Update detection should reuse these seams rather than add another HTTP authority/parser.
- `src-tauri/src/commands/skills_cli.rs` already owns Local gating, the `skills_cli_jobs` exclusive
  lease, cancel command and safe Operation Log boundary. `services/skills_cli/mod.rs` already acquires
  `acquire_target_mutation_guard` inside add/remove and passes cancellation to the supervised process.
- `.trellis/spec/frontend/job-correlation-cancellation.md` requires renderer-created job IDs,
  listen-before-invoke, stale event/promise rejection and exact cancellation correlation.
- The current Skills CLI lock parser only exposes source/sourceUrl/sourceType. The backend-contract
  prerequisite owns any new stable source/path/placement fields and real CLI capability evidence.
- Parent `../08-26-skills-cli-redesign/research/design-contract.md` is the task-tree authority for
  update states, placement semantics, drawer sizing, Base UI Escape, toast and evidence boundaries.

## Verified GitHub documentation facts

Primary sources:

- GitHub REST API rate limits:
  <https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api>
- GitHub REST API best practices:
  <https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api>

The documentation states that unauthenticated primary limits are associated with the originating IP
and are normally 60 requests per hour. This does not prove how much budget remains for this app.
Clients should use `x-ratelimit-remaining`, `x-ratelimit-reset` and `retry-after`; primary/secondary
limits can return 403 or 429. Authenticated conditional requests with a valid ETag can return 304
without consuming primary limit. Therefore “51 skills necessarily exceed 60 requests/hour” is not
a valid planning premise. The implementation must group by unique repository and treat headers plus
typed response classification as authoritative.

## Decisions derived from evidence

- Check one pinned repository snapshot per unique normalized source and derive all skill-path digests
  from that snapshot. Do not use one `commits?path=` request per skill.
- Persist installed baseline separately from observed/pending state. A legacy row without baseline is
  `baseline_required`, never `current`.
- Detect local modifications with an exact versioned content digest captured at baseline time, not
  mtime and not an unverified interpretation of the CLI's `skillFolderHash`.
- Check/apply/recovery extend the existing Skills CLI job family; apply additionally uses the Local
  mutation guard and a durable operation journal.
- Once v7 is released, rollback must retain the v7 descriptor and schema reader. Shipping a v6-only
  binary against a v7 user database would fail future-version preflight.

## Still UNVERIFIED

- Real `skills@1.5.23` support for `--force`, full-SHA pinned source syntax, copy refresh and cancellation.
- Windows junction/symlink privilege, native Tauri focus/Escape behavior and installer/WebView2 layout.
- Live PAT permissions, shared primary/secondary rate-limit behavior, private repositories and real
  repository data.
- Crash recovery against a real external Skills CLI process; design and fake-process tests are not
  native evidence.

