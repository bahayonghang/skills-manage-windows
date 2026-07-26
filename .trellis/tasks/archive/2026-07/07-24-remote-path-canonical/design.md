# Design: Remote canonical path guard

## Boundary

实现归属 `services/central_skills`。新增一个聚焦的 sibling module（建议 `remote_path.rs`），负责远端 lexical normalization、canonical resolution protocol 和 `CentralSkillsError` 映射；`files.rs` 继续负责编排文件读取与目录树。

本任务不在 targets 层引入 filesystem abstraction。所有远端执行复用 `ConnectedRemoteTarget::run_script`，因此 SSH/WSL 自动共享现有 async supervision、timeout、bounded output 和 FakeRunner seam。

## Data Flow

```text
access_root + requested_path
  -> lexical normalize / containment (local Rust)
  -> run_script(CANONICAL_GUARD_SCRIPT, [root, candidate])
  -> remote canonicalize root and candidate
  -> canonical containment in the same script
  -> NUL-terminated canonical candidate
  -> strict Rust protocol parse
  -> existing inspect/read/list using canonical candidate
```

Lexical guard remains an early rejection and avoids unnecessary remote work. It is not the security boundary; canonical containment is.

## Remote Script Contract

Inputs are positional parameters: `$1` is lexical root and `$2` is lexical candidate. Neither value is interpolated into script text.

The script selects a supported resolver without mutating state:

1. Prefer GNU `realpath -e` when the probe succeeds.
2. Fall back to Darwin/BSD `realpath` without `-e` when it resolves an existing `/`.
3. If neither mode is usable, return the stable `resolver unavailable` status.

Both root and candidate must exist. The script captures resolver output with a sentinel so the utility's record-ending newline can be removed without stripping newline characters that belong to the path. It then requires the canonical root to be a directory and applies component-aware containment:

- `candidate_real == root_real`; or
- for a non-root skill root, `candidate_real` matches `root_real/*`;
- root `/` is handled explicitly rather than relying on the `//*` pattern.

On success, the script emits only `candidate_real` followed by a NUL byte. Rust requires exactly one terminal NUL and rejects malformed/extra protocol output. NUL framing allows embedded tab/newline characters.

The implementation will reserve stable non-zero statuses for:

- root resolution/missing failure;
- root not a directory;
- candidate resolution/missing/broken-link failure;
- canonical escape;
- resolver unavailable;
- malformed success output is detected locally as a protocol error.

The script redirects resolver diagnostics so stderr cannot include a sensitive remote path or tool detail. The Rust mapper matches SSH and WSL command-error exit codes and creates typed `CentralSkillsError` variants. Supervisor/transport failures remain `Remote(String)` because their existing typed text contains only transport/policy metadata; unknown command failures map to a generic canonical-resolution variant without forwarding raw stderr.

## Symlink Semantics

The selected A policy mirrors local `Path::canonicalize`:

- root may itself be an install symlink;
- final and intermediate symlinks are allowed when the resolved candidate stays within the resolved root;
- final and intermediate escapes are rejected;
- broken links fail closed because canonicalization requires existence.

The canonical candidate replaces the lexical candidate for the subsequent inspect/read/list call. This closes the check/use mismatch where validation resolves one object but the operation reuses an unresolved alias.

Directory-tree enumeration retains the current non-recursion rule for entries reported as `symlink`, preventing cycles. A caller may explicitly request a contained symlink-to-directory path; the entrypoint resolves it to its canonical directory before enumeration.

## Compatibility

- No database or IPC shape changes.
- No new dependency or targets API.
- Existing valid remote paths continue to work, including a Central skill reached through an agent install symlink.
- Existing remote final-symlink refusal is intentionally relaxed only when canonical containment proves the target is inside the skill root, aligning remote behavior with local.
- Missing paths may produce a more specific Central Skills domain error instead of the current inspect-based missing error; user-visible text remains generic and does not include stderr.

## Tests

Pure tests cover lexical normalization and strict NUL protocol parsing. FakeRunner-backed async tests cover the canonical result/error matrix and assert SSH/WSL parity for command arguments, script stdin, and process policy. Local filesystem tests retain the same policy matrix where platform symlink support is available.

High-value regressions are intermediate escape, contained final/intermediate symlink, root symlink, broken symlink, root equality, prefix trap, and tab/newline argument transport.

## Rollback

The change is isolated to the Central Skills remote guard and error variants. Rollback removes the sibling module and restores the two call sites to lexical candidates; no persisted state or migration is involved.
