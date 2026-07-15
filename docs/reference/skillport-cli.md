# SkillPort CLI

`skillport-cli` is the command-line interface for the Local SkillPort target. It uses
the same SQLite database, Central skill library, protected GitHub credentials,
installation services, and cross-process mutation lock as the desktop app.

The CLI does not manage SSH or WSL targets in this release. Repository development
commands such as `just ci` are documented separately in [CLI: just](./cli-just).

## Run or install

From a repository checkout, run the CLI without installing it:

```powershell
npm run cli -- skills list
```

Build the release binary:

```powershell
npm run build:cli
```

The output is `src-tauri/target/release/skillport-cli.exe` on Windows and
`src-tauri/target/release/skillport-cli` on macOS or Linux.

Install the binary on `PATH` from the checkout:

```powershell
npm run install:cli
```

The equivalent Cargo command is:

```powershell
cargo install --path src-tauri --bin skillport-cli --locked --force
```

The desktop NSIS installer does not add the CLI to `PATH`. The desktop binary is
`skillport`; the command-line binary is `skillport-cli`.

## Command shape

```text
skillport-cli [--json] [--lang en|zh] skills <command>
```

| Global option | Meaning |
| --- | --- |
| `--json` | Write a versioned, single-line JSON envelope for scripts. |
| `--lang en\|zh` | Select English or Chinese human-output labels. Default: `en`. |
| `--help` | Show help for the current command. |
| `--version` | Print the CLI version. |

On Windows, `skillport-cli.exe` and `skillport-cli` are interchangeable when the
installation directory is on `PATH`.

## Inspect Central skills

### List

List every skill in the Local Central library:

```powershell
skillport-cli skills list
```

### Show

Show one Central skill:

```powershell
skillport-cli skills show <reference>
```

`<reference>` resolves in this order:

1. exact immutable `uid`;
2. exact slug / skill id;
3. unique, case-sensitive skill name.

If a name matches more than one Central skill, the command exits with code `3`. Use
the `uid` or slug shown by `list` to disambiguate it.

### Search

Search the remote skills.sh catalog:

```powershell
skillport-cli skills search "react" --limit 10
```

`--limit <number>` is optional. Search does not import or install a skill.

## Import a skill

Install accepts either an exact skills.sh shorthand or a GitHub URL:

```powershell
skillport-cli skills install vercel-labs/agent-skills@react-best-practices
skillport-cli skills install "https://github.com/openai/skills/tree/main/skills/docs"
```

Supported sources:

| Source | Behavior |
| --- | --- |
| `owner/repo@skill` | Resolve and import one exact skills.sh skill. |
| `https://github.com/...` | Preview and import the skills found at the repository or tree URL. |

Local filesystem paths and non-GitHub URLs are rejected. GitHub authentication uses
the credential already configured in SkillPort; there is no token command-line flag.

### Duplicate safety

An existing Central skill is never overwritten implicitly:

```powershell
skillport-cli skills install owner/repo@skill --replace
```

- Without `--replace`, a duplicate exits with code `3`.
- `--replace` explicitly permits overwrite.
- Replacing multiple skills discovered from one GitHub URL also requires `--yes`.

### Import and sync

`--sync` imports the skill and then installs it to Agent directories:

```powershell
skillport-cli skills install owner/repo@skill --sync --agent codex --method copy
```

`--agent <id>` can be repeated. If `--sync` is used without any `--agent`, every
enabled Local Agent except Central is selected.

## Sync Central skills

Preview a sync before writing files:

```powershell
skillport-cli skills sync <uid-or-slug> --agent codex --method copy --dry-run
```

Apply the same plan:

```powershell
skillport-cli skills sync <uid-or-slug> --agent codex --method copy
```

Sync several references or the complete Central library:

```powershell
skillport-cli skills sync <ref-1> <ref-2> --dry-run
skillport-cli skills sync --all --dry-run
```

| Option | Meaning |
| --- | --- |
| `[REFERENCES]...` | One or more Central `uid`, slug, or unique-name references. |
| `--all` | Select every Central skill. Cannot be combined with references. |
| `--agent <id>` | Restrict the plan to an enabled Local Agent; repeat for multiple Agents. |
| `--method auto\|symlink\|copy` | Installation method. Default: `auto`. |
| `--dry-run` | Return target paths and methods without changing the database or filesystem. |

Without `--agent`, sync targets every enabled Local Agent except Central. Use
`--dry-run` before broad `--all` operations.

## JSON and exit codes

Use `--json` for automation:

```powershell
$result = skillport-cli --json skills list | ConvertFrom-Json
if (-not $result.ok) { exit 1 }
```

Successful commands write JSON to stdout:

```json
{"schemaVersion":1,"ok":true,"data":{},"warnings":[]}
```

Errors write JSON to stderr:

```json
{"schemaVersion":1,"ok":false,"error":{"code":"skill.not_found","message":"...","details":{}}}
```

Scripts should branch on `ok`, `error.code`, and the process exit code. Human-readable
`message` text is not a stable machine contract.

| Exit | Meaning | Representative code |
| --- | --- | --- |
| `0` | Success | none |
| `1` | Internal service or database failure | `internal.error` |
| `2` | Invalid source, method, or sync scope | `input.invalid` |
| `3` | Missing, ambiguous, or duplicate skill | `skill.not_found`, `skill.ambiguous`, `skill.duplicate` |
| `4` | Another process owns the Central mutation lock | `mutation.busy` |
| `5` | A batch completed with partial failures | success envelope with failed items |

## Desktop coordination

The desktop app and CLI can run at the same time. Mutating commands share the Central
mutation lock, so competing writes fail safely with exit code `4` instead of modifying
the library concurrently. Retry after the other operation finishes.

CLI changes do not push a live event into an already-open desktop window. Refresh the
relevant desktop view to reload updated skills or installation state.

## Current limits

- Local target only; no SSH or WSL CLI commands.
- GitHub URL or `owner/repo@skill` imports only; no local-path import classifier.
- No automatic PATH changes from the desktop installer.
- No live desktop refresh notification after a CLI mutation.
