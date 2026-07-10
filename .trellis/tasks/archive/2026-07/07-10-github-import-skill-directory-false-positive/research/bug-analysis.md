# Bug Analysis: GitHub import rejects repository-level skill directory

## 1. Root Cause Category

- **Primary category**: E - Implicit Assumption
- **Specific cause**: Candidate filtering assumed every non-root directory whose normalized ID is `skill` is a generic packaging artifact. That assumption did not distinguish a repository-level `skill/SKILL.md`, which upstream projects use as a complete distributable skill container.
- **Secondary category**: B - Cross-Layer Contract
- **Specific cause**: The backend preserves typed `RateLimited` and `AccessDenied` errors only until the Tauri command boundary, then the frontend reconstructs authentication meaning from strings. A broad `pat` substring matched the unrelated word `subpaths`.
- **Contributing category**: D - Test Coverage Gap
- **Specific cause**: Existing tests covered deep generic `.../skill/` rejection but no positive top-level `skill/SKILL.md` case, and UI tests covered positive PAT guidance but no non-auth messages containing misleading substrings.

## 2. Why Earlier Fixes Failed

1. The original generic-filter change fixed the observed `agent_reach/skill` symptom at the candidate ID layer. It encoded the sample path's basename as global intent, so later repositories with a valid top-level `skill/` were indistinguishable.
2. The original PAT helper optimized for catching many historical string messages with a single broad expression. Bare `github`, `settings`, and `pat` terms increased recall but had no token boundaries or domain-specific negative cases.
3. The test support layer copied the production regex instead of reusing the helper, allowing future behavior drift between real UI and marketplace integration tests.

## 3. Prevention Mechanisms

| Priority | Mechanism | Specific Action | Status |
| --- | --- | --- | --- |
| P0 | Test coverage | Add a `kill-ai-slop`-shaped snapshot and root/subpath parity regression | DONE |
| P0 | Domain contract | Specify top-level `skill/` identity and deep generic-wrapper filtering separately | DONE |
| P0 | UI regression | Test `subpaths`, URL validation, rate limiting, and configured-token messages in the real wizard | DONE |
| P1 | Code reuse | Make marketplace test support call the production authentication classifier | DONE |
| P1 | Search audit | Confirm no second generic `skill_id == "skill"` filter or copied auth regex remains | DONE |
| P2 | Architecture | Consider structured GitHub import error codes across IPC when this error surface next expands | DEFERRED; explicitly out of scope |

## 4. Systematic Expansion

- **Similar issues**: Other path-quality heuristics can become false positives when basename is treated as intent without depth or discovery-origin context. The current search found no second generic-skill filter.
- **Design improvement**: Candidate identity and content scope must remain distinct. This fix changes identity to repository-derived while preserving `sourcePath=skill` for copying and update metadata.
- **Process improvement**: Every negative heuristic needs a neighboring positive fixture at the closest valid boundary, not only examples it is expected to reject.
- **Cross-layer improvement**: When typed backend errors are stringified, frontend recovery must use explicit phrases backed by positive and negative examples. Bare product names or short substrings are not sufficient classifiers.

## 5. Knowledge Capture

- [x] Updated `.trellis/spec/backend/github-import-preview-contract.md` with candidate and error-guidance contracts.
- [x] Added backend and real-component regression tests.
- [x] Removed the duplicate classifier from marketplace test support.
- [x] Checked for a template spec mirror; this application repository has no `src/templates/markdown/spec/` tree, so no sync target exists.
- [x] Committed code/tests as `3bc3b6c` and the durable spec as `33dcad2` after the full Phase 2.2 quality gate passed; task artifacts are carried by the archive commit.
