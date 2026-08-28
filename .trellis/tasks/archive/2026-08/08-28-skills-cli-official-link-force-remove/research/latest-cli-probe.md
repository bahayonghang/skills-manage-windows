# npm `skills` latest CLI probe (segment 0)

Date: 2026-08-28  
Host: local Windows, Node `v26.7.0`, npm `12.0.2`  
Method: `npm view skills@latest`, `npm pack skills@1.5.23`, run packed `bin/cli.mjs --help` / `add --help`, read packed `dist/cli.mjs`.  
Live `npx skills add` install on disk: **UNVERIFIED** (not spawned).

## Registry

| Field | Value |
| --- | --- |
| `dist-tags.latest` | `1.5.23` |
| `dist-tags.snapshot` | `1.5.23-snapshot.0` |
| Published | `2026-08-18T20:31:50.817Z` |
| `engines.node` | `>=22.20.0` (unchanged vs SkillPort `SKILLS_CLI_MIN_NODE`) |
| Packed tarball | `skills-1.5.23.tgz` |

Current SkillPort PIN `skills@1.5.23` **is** today's latest. Unpinning to `--package=skills` does not change the resolved version on this date; it follows the `latest` dist-tag thereafter.

## Help surface (packed CLI, `--help` = `add --help`)

Add options include:

- `-g, --global` — install globally
- `-a, --agent <agents>` — agents (`*` = all; SkillPort must not default this)
- `-s, --skill <skills>`
- `-y, --yes` — **Skip confirmation prompts**
- `--copy` — **Copy files instead of symlinking to agent directories**
- `--all` — shorthand for `--skill '*' --agent '*' -y` (SkillPort must not default)

No `--symlink` / `--link` flag is documented. Symlink is the documented default; `--copy` is opt-in.

README Installation Methods: Symlink (Recommended) vs Copy. Option table: `--copy` = copy instead of symlink.

## Default install mode (source, not a live add)

Packed `dist/cli.mjs` (both well-known and git add paths):

```js
let installMode = options.copy ? "copy" : "symlink";
// prompt only when !copy && !yes && uniqueDirs.size > 1
} else if (uniqueDirs.size <= 1) installMode = "copy";
```

| Argv | unique agent `skillsDir` count | Mode |
| --- | --- | --- |
| `--copy` | any | copy |
| `-y`, no `--copy` | `> 1` | **symlink** (prompt skipped) |
| `-y`, no `--copy` | `<= 1` | **copy** (undocumented shortcut; no `--symlink` flag) |
| interactive, no `--copy` | `> 1` | prompt (symlink recommended) |
| interactive, no `--copy` | `<= 1` | copy |

This is the historical “single-agent `-y` copies” behavior of **the same 1.5.23 binary**, not a new latest regression. SkillPort typically passes multiple `-a` agents; when their `skillsDir` values differ, `-y` without `--copy` stays symlink.

There is **no** official flag to force symlink. Inventing one is forbidden. Omitting `--copy` is the documented way to get the default.

## Lock schema

`CURRENT_VERSION = 3` (`.skill-lock.json`). `readSkillLock` rejects `version < 3` by returning an empty lock. Ownership remains lock v3 names.

## Gate vs R6

| R6 expectation | Probe |
| --- | --- |
| default symlink | Documented + initial `installMode`; `--copy` opt-in |
| `-y` skips prompts | Help text + source (`if (!options.yes) confirm`) |
| lock v3 | `CURRENT_VERSION = 3` |
| fail-closed if `-y` always copies with no symlink flag | **Does not apply.** `-y` + multiple unique dirs = symlink. No new flag invented. |

**Decision: PIN change to `--package=skills` is allowed.** Do not add `--copy`. Do not invent `--symlink`. Single-agent `-y` copy shortcut is upstream; SkillPort still must not pass `--copy`.

## Not verified

- Live `npx skills add … -g -y -a … -s …` creating a relative Claude Code symlink.
- Future `latest` after 1.5.23 (lock schema or default mode could change).
