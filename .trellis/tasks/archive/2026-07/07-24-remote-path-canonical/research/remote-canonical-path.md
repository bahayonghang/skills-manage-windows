# Remote canonical path research

## Confirmed repository evidence

- `services/central_skills/files.rs` performs local root/candidate canonicalization but remote access only has lexical normalization plus a final-object symlink check.
- `ConnectedRemoteTarget::run_script` dispatches the same script/argument interface to SSH and WSL and uses the supervised standard process policy.
- `FakeRunner` records program, args, stdin and policy, so command parity can be proven without live SSH/WSL.
- `TargetsError::RemoteCommandFailed` and `TargetsError::WslCommandFailed` retain `ExitStatus`; Central Skills can map stable script status codes without parsing stderr.
- Existing `CentralSkillsError::Remote(String)` is the approved transport-boundary mapping, while requirement-specific failures should use semantic variants.

## Security conclusion

Lexical containment is necessary but insufficient because filesystem lookup resolves every intermediate symlink. The safe operation target is the canonical candidate returned by the same remote script that canonicalizes the root and checks component-aware containment.

The original unresolved path must not be reused after the check. Otherwise an intermediate alias remains available to the later inspect/read/list operation.

## Portability and path framing

- GNU supports `realpath -e`; Darwin/BSD `realpath` resolves existing paths without that option. Probe the GNU form first, then the plain form, and fail closed if neither works.
- Root and candidate are passed as positional arguments. This avoids shell-source interpolation and preserves whitespace.
- Newline-delimited output is ambiguous for valid newline-containing paths. A single NUL-terminated success value gives the Rust parser an unambiguous boundary.
- Command substitution strips trailing newlines. Appending a sentinel before assignment and removing only the sentinel plus the resolver's one record terminator preserves newline characters belonging to the path.
- Resolver diagnostics should be redirected; typed exit codes are sufficient and prevent raw remote paths/tool output from reaching user-visible errors.

## Scope conclusion

This is a Central Skills access-policy fix, not a general remote filesystem layer. A small sibling module keeps `files.rs` focused and stays within the current source-size budget.
