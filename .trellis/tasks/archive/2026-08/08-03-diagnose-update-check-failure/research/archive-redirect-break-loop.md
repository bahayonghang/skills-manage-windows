# Bug Analysis: GitHub Archive Canonical Redirect

## 1. Root Cause Category

- **Category**: E - Implicit Assumption, with a D - Test Coverage Gap amplifier.
- **Specific Cause**: The first archive repair modeled GitHub archive delivery as
  only `302 -> codeload` with case-sensitive repository identity. Live GitHub also
  performs case-only normalization and, for renamed repositories, a trusted
  `301 -> /repositories/{numeric_id} -> 302 -> codeload` chain.

## 2. Why Earlier Fixes Failed

1. The PAT/error-envelope repair made the failure observable but did not change
   the archive protocol model.
2. The all-skill inventory repair restored scope and provenance, exposing all 24
   repositories and therefore the first incompatible live redirect.
3. The one-hop archive fix used synthetic 302 fixtures only, so it never tested
   GitHub case normalization or renamed repository canonicalization.

## 3. Prevention Mechanisms

| Priority | Mechanism | Specific Action | Status |
| --- | --- | --- | --- |
| P0 | Test coverage | Capture case-only and trusted numeric redirect shapes in production-policy and local transport tests | DONE |
| P0 | Architecture | Carry initial endpoint provenance into an archive-only finite state machine | DONE |
| P0 | Credential boundary | Rebuild each request; Bearer only on validated direct API hops, never mirror/codeload | DONE |
| P1 | Parser hardening | Reject backslash, encoded separators, userinfo, and dot segments before URL normalization | DONE |
| P1 | Live contract probe | Check all populated repository identities with automatic redirects disabled | DONE |

## 4. Systematic Expansion

- **Similar Issues**: Any external service redirect validator that compares only
  parsed paths can be vulnerable to parser normalization and authority confusion.
- **Design Improvement**: Treat redirect permission as state derived from the
  request origin, not from the destination URL alone.
- **Process Improvement**: For external HTTP contracts, pair synthetic hostile
  fixtures with a read-only live shape inventory; the live probe validates the
  model, while deterministic fixtures validate product behavior.

## 5. Knowledge Capture

- [x] Update the GitHub import archive redirect code-spec.
- [x] Update test ownership and required hostile matrices.
- [x] Preserve the probe and its unit tests under task research.
- [ ] Record final full-gate and application-level acceptance evidence.
