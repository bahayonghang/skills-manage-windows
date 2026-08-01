# Windows updater staging runbook

This runbook is intentionally separate from a public release. It validates an update from the previous stable Windows NSIS installer to a candidate built by `Release Desktop`; it never uses the public `latest` channel for an unpublished candidate.

## Approval gate

Before enabling `run_updater_staging_smoke`, obtain explicit approval for the staging feed URL, isolated Windows runner/environment, credentials, candidate SHA, previous stable version, and rollback target. The workflow defaults to disabled and rejects `github.com` feeds. Do not create a tag, GitHub Release, Azure resource, secret, or environment as part of this runbook.

## Inputs and evidence

- Install the verified previous stable NSIS into an isolated directory.
- Host the candidate `latest.json`, final signed NSIS, updater `.sig`, and checksum on the approved HTTPS staging feed.
- Record the previous version, candidate frozen SHA, SHA256SUMS, Authenticode result, updater signature result, and staging URL.
- Use the manual `rehearsal` mode with the exact 40-character `rehearsal_ref`; leave `publish` unselected.

## Execution

1. Confirm the prior installer is Authenticode-valid and starts successfully.
2. Confirm candidate NSIS Authenticode, updater signature, `latest.json`, and checksum all verify against the staged bytes.
3. Start the prior application, request the update from the staging feed, and wait for the passive installer to finish.
4. Confirm the candidate version and executable path after restart, then retain install and updater logs as evidence.
5. Run the candidate uninstaller and verify the installation directory is removed.

## Rollback

Stop the staging run on any verification, launch, or version mismatch. Restore the saved previous installer and its known-good staging metadata; do not move the candidate tag, alter a public release, or rotate credentials. Record the failed step and preserve the candidate artifacts for diagnosis.
