# Implementation Plan

## 1. Add the canonical guard

- [ ] Add a Central Skills-owned remote path module with the existing lexical normalization helpers.
- [ ] Preserve raw path characters rather than trimming valid leading/trailing tab/newline data; keep explicit empty-path handling and existing `..`/backslash rejection.
- [ ] Add the portable GNU/Darwin `realpath` script, positional-argument contract, sentinel-safe capture and NUL-framed success protocol.
- [ ] Parse the protocol strictly and map stable SSH/WSL exit statuses into typed `CentralSkillsError` variants without raw stderr.

Review gate: confirm the helper returns only a canonical candidate that has passed canonical containment.

## 2. Integrate read and list entrypoints

- [ ] Replace lexical-only remote file-read resolution with the async canonical helper.
- [ ] Remove the final-symlink blanket refusal; inspect/read the returned canonical path and preserve file type, UTF-8 and resource-budget checks.
- [ ] Replace lexical-only remote directory-root resolution with the same canonical helper.
- [ ] Preserve non-recursive handling for symlink entries discovered during tree enumeration.

Review gate: no inspect/read/list call may use the original unresolved candidate after canonical validation.

## 3. Add focused regression coverage

- [ ] Extend lexical tests for relative/absolute inside paths, outside absolute path, `..`, backslash and prefix trap.
- [ ] Add protocol/error mapping tests for root equality, missing/broken path, root-not-directory, resolver unavailable, malformed output and escape.
- [ ] Add FakeRunner SSH/WSL parity tests asserting script stdin, raw positional args including tab/newline, parameter order and standard policy.
- [ ] Add local symlink matrix coverage for contained and escaping final/intermediate links plus a root install symlink where supported.
- [ ] Verify explicit contained symlink-to-directory access works while discovered symlink entries remain non-recursive.

Review gate: the bypass model `root/docs -> /etc`, request `root/docs/passwd`, must fail before any read/list call is issued.

## 4. Validate and review

Run the smallest checks first, then the full repository gate:

```powershell
cd src-tauri
cargo test central_skills --locked
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cd ..
just ci
```

- [ ] Inspect `git diff --check` and the final scoped diff.
- [ ] Confirm no dependency, database, IPC, frontend or unrelated Trellis changes entered the implementation diff.
- [ ] Re-evaluate whether the canonical remote-path contract should be captured in `.trellis/spec/backend/path-policy.md` during Phase 3.3.

## Risk / Rollback Points

- Shell portability is the highest-risk point; do not proceed if GNU and Darwin resolver branches cannot be distinguished deterministically.
- Protocol parsing must fail closed on extra output, missing NUL, invalid UTF-8 or unknown exit statuses.
- If integration reveals a targets-layer API change is required, return to planning rather than expanding scope.
