# Git And Release

## Branches And Pull Requests

- `dev` is the permanent daily-development branch.
- Short-lived task branches target `dev` and use squash merge; the task branch is deleted after
  merge according to repository policy.
- A `dev` to `main` promotion uses a merge commit. Refresh and verify the exact promotion merge
  SHA, then fast-forward `dev` to that SHA before writing Trellis bookkeeping or starting another
  task.
- Preserve `main` required `just-ci`, review, resolved-conversation, administrator, and
  no-force/no-delete protections. Show the target and read back actual values before changing
  remote rulesets or merge settings.

## Documentation And Release

- Keep local `just ci` checks aligned with GitHub Actions. CI is PR-targeted at `dev` or `main`;
  ordinary pushes do not trigger the required workflow.
- Docs deployment runs only for a published release or an explicitly authorized canonical `main`
  manual run. A single Pages artifact goes through deploy and online smoke; do not restore a
  second deploy build or recreate the legacy `gh-pages` branch without approval.
- For release reviews, protect the Windows x64 artifact set: signed NSIS, matching updater `.sig`,
  `latest.json`, MSI, and ZIP. Authenticode validation happens before updater signing; signing
  metadata must be generated from the final NSIS.

## Windows installer smoke and release-context toolchains

`release-context` pins Node from `package.json#engines.node` (`26.x`) and Rust from
`rust-toolchain.toml` (`1.98.0`) before resolver use:

checkout resolver → setup Node 26 → assert node 26.x → resolve-only checkout →
setup Rust 1.98.0 → assert rustc 1.98.0 → full resolver.

`windows-install-smoke` runs two isolated matrix cases (`nsis` and `msi`) with unique
`$RUNNER_TEMP` install roots. Each case uses `scripts/release/windows-installer-smoke.ps1`
for bounded install → verify unique installed `skillport.exe` → launch → stop → uninstall →
residue cleanup. Cleanup failure is an independent failure. Job `timeout-minutes` values
(15 for release-context, 20 for installer smoke) bound the outer wait; the helper owns
per-process 120s deadlines and process-tree kill. Stage logs are `{stage, outcome,
exitCode?, timedOut, cleanupOutcome}` plus digest/install root.

Authenticode on the installed exe follows `windows-signing.json` via the same policy as
`validateSigningState`: `authenticode=valid` requires Valid + signer + timestamp;
`authenticode=not-configured` must remain NotSigned and must not be reported as signed.
Publish still requires valid Authenticode through existing preflight. Installed-exe
Authenticode does not prove the inner exe was signed before bundle; that inner-exe-before-bundle
path and real user-machine compatibility remain **UNVERIFIED**. REL-001 and REL-002 stay open.

A controlled `windows-2022` NSIS/MSI rehearsal using final signed assets remains
**UNVERIFIED** until that remote run completes.

Remote writes, release publication, branch deletion, tag creation, and ruleset changes require
explicit user authorization and a final read-back of the affected ref or setting.
