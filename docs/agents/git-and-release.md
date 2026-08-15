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

Remote writes, release publication, branch deletion, tag creation, and ruleset changes require
explicit user authorization and a final read-back of the affected ref or setting.
