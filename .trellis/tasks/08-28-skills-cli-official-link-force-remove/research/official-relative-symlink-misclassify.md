# Official relative symlink classified as wrong_link_target

Date: 2026-08-28
Target: SSH remote Linux (`/home/lyh`), Skills CLI global page, skill `ask-matt`.

## Observed UI

Uninstall dialog for `ask-matt`:

- 1 owned folder (canonical under Universal Agents)
- 0 managed links
- Independent copies retained on Codex CLI, Cursor, OpenCode, Amp, GitHub Copilot, Cline, Deep Agents, Firebender, Kimi Code CLI, Warp (`direct_copy`)
- Conflict: `ask-matt` on Claude Code: `wrong_link_target`
- Confirm disabled (`confirmable` requires empty conflicts)

`~/.claude/skills/<name>` entries are directory symlinks whose `readlink` text is relative:

```text
../../.agents/skills/<name>
```

which resolves on disk to `/home/lyh/.agents/skills/<name>`. This matches official Skills CLI **symlink** install (docs: symlink is recommended; default `npx skills add` without `--copy`).

One exception in the same directory (`trellis-plan-review`) pointed at `~/.skillsmanage/sk…` (Central), which is a true foreign target.

## Why SkillPort disagrees

Remote inventory uses `readlink` (not `readlink -f`) and string-equality after join:

- Probe script: `src-tauri/src/services/skills_cli/probe.rs:35` (`t=$(readlink "$p" …)`)
- Compare: `probe_resolves_to_canonical` at `probe.rs:182-209`
- Relative target is joined with `remote_parent` + `remote_join` (`paths.rs:438-447`)
- `remote_join` concatenates only; it does not collapse `.` / `..`
- `normalize_compare` (`probe.rs:212-214`) only unifies slashes and trailing `/`

Worked example:

| Piece | Value |
| --- | --- |
| slot | `/home/lyh/.claude/skills/ask-matt` |
| `readlink` | `../../.agents/skills/ask-matt` |
| joined | `/home/lyh/.claude/skills/../../.agents/skills/ask-matt` |
| expected canonical | `/home/lyh/.agents/skills/ask-matt` |

String compare fails → `REASON_WRONG_LINK_TARGET` (`directory_link.rs:26`, applied in `probe.rs:141-144`).

Local observe uses `paths_equivalent` (`directory_link.rs:101`) which runs `canonicalize_path_with_missing` (`paths.rs:314-336`) and therefore collapses `..` when the path exists. Same topology on Local would typically be `managed_link`; SSH remote mislabels it as `conflict`.

## Downstream mutation

- Remove plan treats `Conflict` as blocking (`remove.rs:305-324`, `confirmable: conflicts.is_empty()`).
- `remove_verified_directory_link` refuses Conflict/ordinary dirs (`directory_link.rs:204-214`).
- Remote unlink of a symlink is `rm -f` only when the slot is already classified as a managed link (`remote_scripts.rs:282-285`). Conflict never enters that path.

## Official CLI vs SkillPort PIN

- Official docs (vercel-labs/skills, 2026-08-28 fetch): `npx skills add …`; default method is **symlink**; `--copy` is opt-in. No separate `link` subcommand in current README; users may describe the symlink step as “link”.
- SkillPort PIN: `SKILLS_CLI_NPM_SPEC = "skills@1.5.23"` (`argv.rs:12`). PIN single-agent `-y` historically defaults to **copy** (archived `copy-mode-ownership.md`), which does not write Claude-style relative links.
- SkillPort remote `ln -s` uses the **absolute** canonical path (`remote_scripts.rs:260-263`), so SkillPort-created links would match string equality. Official relative links would not.

## Not yet verified on this machine

- Live `readlink` / `skills --version` on the SSH host (UNVERIFIED; UI + directory listing used).
- Whether latest unpinned `npx skills` lock schema remains v3.
- Whether `npx skills remove -g` would delete DirectCopy directories (must not be assumed).
