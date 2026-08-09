# Live GitHub Archive Redirect Shapes

## Reproduction

A read-only diagnostic loaded the 24 populated GitHub repositories from the current database and issued unauthenticated archive GETs with automatic redirects disabled. The feedback loop completed in under 10 seconds and failed when any response did not match the current production policy.

Observed results:

- 18 repositories: exact `302 -> codeload.github.com/{owner}/{repo}/legacy.tar.gz/refs/heads/{branch}`.
- 4 repositories: the same safe 302 shape, but GitHub canonicalized only ASCII case in owner and/or repo.
- 2 repositories: `301 -> api.github.com/repositories/{numeric_id}/tarball/{branch}`, followed by `302 -> codeload.github.com/{renamed_owner}/{renamed_repo}/legacy.tar.gz/refs/heads/{branch}`.
- No query, fragment, userinfo, non-HTTPS authority, nonstandard port, branch change, or non-GitHub host was observed.

This reproduces the user's `github_import.archive_redirect_rejected` result without a PAT and separates it from database, scope, and persistence behavior.

## Root Cause

The first archive fix encoded a narrower model than GitHub's live contract:

- it accepted only initial `302 Found`, not GitHub's permanent numeric repository canonicalization;
- it compared codeload owner/repo segments case-sensitively;
- it had no explicit state proving a numeric canonicalization was authorized by a response from the trusted direct API rather than a mirror.

The update run is fail-fast at repository snapshot acquisition, so any one of these repositories makes a full 141-skill check end in an error after earlier repositories have already consumed network time.

## Safe Fix Boundary

- Preserve the global no-redirect client.
- Treat case-only owner/repo changes as the same GitHub identity.
- Permit numeric canonicalization only from the trusted direct API response and validate its exact authority/path/ref.
- Keep Bearer on trusted API hops only; create the final codeload request without auth.
- After numeric canonicalization, validate canonical owner/repo as safe components and the ref as exact, but do not require renamed identity to equal the stale input.
- Reject the same shapes from mirrors and reject every additional redirect after codeload.
