# Harness Guide

Project contract: `AGENTS.md`. This file records five-tool discovery, accepted bootstrap
facts, task handoff, and evidence layers. It is not a product user guide and not a model
router.

Claude Code and Grok both load `CLAUDE.md`. That file imports `AGENTS.md` and stays thin so
the two files do not drift. Codex and OMP keep `AGENTS.md` as the project instruction path.
Live session loading for Claude, Codex, and OMP has not been run here and remains
**UNVERIFIED**. `grok inspect` is capability discovery, not an execution pass.

## Evidence layers

| Layer | What it can prove | What it cannot prove |
|---|---|---|
| Static files / imports / gitignore exceptions | Tracked paths, `@AGENTS.md` import, hook file bytes, skill text | That a session loaded them |
| CLI `--version` / `--help` / `codex features list` | Binary present; flag and feature names | Agent registry, hook firing, provider/model |
| `grok inspect --json` | Inspected projectRoot, instruction paths, hook/skill/agent lists, trust flags | Hook execution, provider availability, a passing session |
| Python / Vitest / `docs:gen:check` | Named tests and read-only docs checks on this machine | Hosted Linux/macOS runners, native WebView, installer |
| Five-tool live session | Only after an authorized real session | Must not be inferred from help text or inspect |

A documentation or contract-test PASS is not a Windows installer, Authenticode, or product
release result. A frontend-only build does not establish packaging acceptance.

## Model routing (prompt convention)

This is a prompt convention, not a harness capability and not a price table. Do not equate a
tool brand with model skill or cost.

- **Strong-model owner:** identity, credentials, filesystem + database, CI / release,
  capability disputes, source-of-truth decisions, and independent review.
- **Cheap-exec whitelist:** given files and assertions, fixtures, checklists, thin
  documentation, and command execution.
- **Escalate** when a new failure appears, the file or permission set would grow, an
  assertion would change, or the work crosses layer semantics.
- OMP `pi/task` is a config string. Availability and the resolved provider/model remain
  **UNVERIFIED**.

## Five-tool matrix

Permissions are marked **capability** (the tool can enforce it) or **prompt convention**
(role text; not OS path isolation). This repository does not claim OS-level directory
sandboxing from agent prompts.

### Claude Code

| Topic | Fact | Kind |
|---|---|---|
| Rules | Native `CLAUDE.md`; `@AGENTS.md` import | Capability |
| Subagents | `.claude/agents/trellis-{research,implement,check}.md`; no `model` field (request inherit) | Static files. Live load **UNVERIFIED** |
| Hooks | `.claude/settings.json` lists SessionStart, PreToolUse, PostToolUse, UserPromptSubmit. Tracked inject hook: `.claude/hooks/inject-subagent-context.py` | Config capability. Enable / trust / firing **UNVERIFIED** |
| Tools | Agent frontmatter `tools` can restrict calls | Capability |
| Write boundary | Research may persist under the current task `research/` only | Prompt convention |
| Native Windows sandbox | Not demonstrated here | **UNVERIFIED** |

Official: [memory/import](https://code.claude.com/docs/en/memory), [hooks](https://code.claude.com/docs/en/hooks), [subagents](https://code.claude.com/docs/en/sub-agents).

### Codex

| Topic | Fact | Kind |
|---|---|---|
| Rules | Native `AGENTS.md`; `.codex/config.toml` also lists `AGENTS.md` as fallback | Capability |
| Subagents | `.codex/agents/*.toml`; `sandbox_mode = "workspace-write"`; `agents.max_depth = 1`; model lines are comments (request inherit) | Sandbox mode is capability. Final model **UNVERIFIED** |
| Hooks | `.codex/hooks.json` lists SessionStart, UserPromptSubmit, SubagentStart, PreToolUse. Tracked inject hook: `.codex/hooks/inject-subagent-context.py`. Project config cannot set user feature flags | Config capability. User enable / `/hooks` review / firing **UNVERIFIED** |
| Write boundary | Research path limited to the current task `research/` | Prompt convention |

Official: [AGENTS.md](https://developers.openai.com/codex/guides/agents-md), [subagents](https://developers.openai.com/codex/subagents).

### Grok Build

| Topic | Fact | Kind |
|---|---|---|
| Rules | Inspect loads root `AGENTS.md` and `CLAUDE.md`; thin `CLAUDE.md` is the redundancy fix | Inspect capability, not session execution |
| Subagents | `.grok/agents/trellis-*.md`; `spawn_subagent` in agent text; no model/tool mode in frontmatter | Prompt convention. Live spawn **UNVERIFIED** |
| Hooks | Inspect is capability discovery, not an execution pass. This check's inspect: `projectTrusted` true; project `.claude` hooks listed but `disabled` / Claude-compat `hooks` surface off. `configWarnings` includes unknown-field `privacy` (not an empty-warning PASS). Enable / firing remain **UNVERIFIED** | Inspect observation ≠ session execution |
| Write boundary | Role text; official capability modes exist | Blocking behavior **UNVERIFIED** |

Official: [Grok Build](https://github.com/xai-org/grok-build), [subagents](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/16-subagents.md).

### Kimi Code

| Topic | Fact | Kind |
|---|---|---|
| Rules / skills | `.gitignore` exceptions for `.kimi-code/skills/trellis-{research,implement,check}/SKILL.md`. Not a live-load proof | Gitignore exceptions. Live skill load **UNVERIFIED** |
| Custom agents | Current Kimi Code documents project custom agents. This repo does **not** claim they are absent | Official docs. Live project-agent discovery **UNVERIFIED** |
| Research | Default dispatch is built-in read-only `explore`; the main session persists returned findings only under the known task `research/` path | `explore` read-only is capability. Persist-by-main is prompt convention |
| Implement / check | Until live discovery is verified, main session dispatches built-in `coder` with these skills and `Active task:` as the first prompt line; stay in approved scope | Prompt convention |
| Model pool | Official product supports model pool / secondary model | This repo's resolved model **UNVERIFIED** |

Do not use `kimi --agent*` as a read-only check; it starts a session.

Official: [custom agents](https://moonshotai.github.io/kimi-code/en/customization/agents).

### Oh My Pi (OMP) / Pi

| Topic | Fact | Kind |
|---|---|---|
| Rules | Root `AGENTS.md` is the project fact source; no separate OMP AGENTS file required | Context-file capability (docs). Live load **UNVERIFIED** |
| Subagents | `.omp/agents/trellis-{research,implement,check}.md`. Research/implement declare `model: pi/task`; check does not | Config string. `pi/task` availability **UNVERIFIED** |
| Agents CLI | `omp agents --help` exists; `omp agents list` has been observed exit 1; `omp agents unpack` has write side effects and is not a read-only check | Help is capability discovery. Unpack not run here |
| Tools | Frontmatter `tools` allowlist | Capability |
| Write boundary | Research allowlist still includes write/bash; directory limit is role text | Allowlist is capability. Path isolation is prompt convention |

Official: [context files](https://github.com/can1357/oh-my-pi/blob/main/docs/context-files.md), [task-agent discovery](https://github.com/can1357/oh-my-pi/blob/main/docs/task-agent-discovery.md).

## Task handoff

```text
Active task: .trellis/tasks/<mm-dd-name>
python ./.trellis/scripts/task.py create "<title>" --base-branch dev
```

- Prefer an explicit `Active task:` line. After session isolation, a determined `context_key`
  does not borrow or clear other sessions. `task.py current` on an invalid pointer exits
  non-zero and is not an executable task.
- **Research:** default read-only `explore` (Kimi) or the platform research agent. Results
  return to the main session. The main session persists only under that task's `research/`
  directory. Do not persist into other task directories, specs, or source.
- **Implement / check:** stay inside the approved file and permission scope. Do not spawn
  nested Trellis implement/check agents. Do not git commit, push, or amend from those roles.
- Write-path limits are prompt conventions unless the tool's allowlist or sandbox field
  actually omits write. Do not call prompt refusal a sandbox proof.

## Accepted evidence (archived children)

Write only these as accepted. Source commits and counts are frozen from those children.

### session-isolation

Determined `context_key` no longer borrows or clears other sessions. Isolation tests 17 OK;
full Python discover 54 ran / 4 POSIX skipped / exit 0. `task.py current` invalid pointer →
non-zero, not executable. Commits: `d52f0fb7`, `d73d18b2`, archive `c4a976ae`.

Applies to all five tools at the shared Python session layer. Five-tool live session smoke
remains **UNVERIFIED**.

### doctor-pnpm-readonly

Probe is `pnpm --version`, timeout 5s, pin 10.34.5, with child env
`pnpm_config_pm_on_fail=ignore` only. Match / mismatch / timeout tests 20 passed via
leftover-engine 10.34.5. Scoop PATH shim is still 12.3.4. Typecheck follow-up:
`doctor.test.ts` ES2020-safe (`6edb265d`). PATH canonical `pnpm exec` / `just ci` were **not**
that child's gate.

Applies to local doctor diagnostics. Does not prove the pin is on PATH for later commands.

### bootstrap-and-gates

`.gitignore` layered exceptions for two inject hooks and three Kimi skills. Hooks SHA256
`e4eee1adbebb5c45bd615254fc33e56f7b29dea841b372f4fb58aba6a33c355b` (both inject files
identical). Tracked. Required-hook absence fails closed. `rust-platform` lane is clippy →
cargo test → trellis-python (Windows `python` / POSIX `python3`). Isolated
`trellis init --skip-existing` left hook bytes unchanged. Python 54 / 4 skip. Vitest
`runCi` + contract 18 passed **direct**. Missing-hook fixture exit 1.

Linux and macOS hosted `rust-platform` runners remain **UNVERIFIED**. Five-tool live session
discovery, hook enable/trust, and hook firing remain **UNVERIFIED**.

### central-bulk-install

Skip replaced with ToolbarViewMenu → checkbox → BulkActionBar; `batchInstallSkills`
`["frontend-design"], ["kiro"], "symlink"`. `pnpm test` 2065 passed / 0 skipped. Native
WebView **UNVERIFIED**.

Applies to frontend Vitest. Not a WebView or installer result.

## UNVERIFIED (do not treat as passed)

- Linux and macOS hosted `rust-platform` runners
- Five-tool live session discovery, hook enable/trust, hook firing, provider/model
- OMP `pi/task` availability (config string ≠ proven)
- Grok inspect ≠ execution pass; this check's inspect had `projectTrusted` true and listed-but-disabled project `.claude` hooks; firing still unproven
- Native WebView / Windows installer / Authenticode inner-exe-before-bundle / real user-machine
- `just ci` until the parent runs it (parent release gate, not run in this child)

REL-001 and REL-002 are **contract-wontfix** with residual risk; they stay closed. See
`docs/agents/git-and-release.md` and the 2026-09-03 close contract. Do not reopen them from
this guide. Do not edit historical reports.

Read-only review entry: `docs/agents/build-and-test.md`.
