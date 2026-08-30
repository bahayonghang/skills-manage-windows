# SSH doctor node vs login-shell Node

Date: 2026-08-28

## User evidence (login shell on remote host)

Interactive prompt on `dckj-MS-7E06`:

```text
which node
/home/linuxbrew/.linuxbrew/bin/node

node --version
v26.7.0
```

That Node is **Linuxbrew**, not `/usr/bin/node`.

## Why the previous “Node 20” claim is not a fact about this host

The earlier turn treated a Skills CLI page screenshot caption as `Node v20.x`. That caption was OCR of the desktop UI, never a command run on this host. It is **not** evidence that the user’s login Node is 20, and it must not drive Node-version product gates.

This host’s login Node is **v26.7.0** (≥ SkillPort `SKILLS_CLI_MIN_NODE` 22.20.0).

## How SkillPort actually probes remote Node

Remote doctor (`transport.rs` `DOCTOR_PROBE_SCRIPT`):

```sh
command -v node >/dev/null 2>&1
node --version
```

This runs over SSH `run_script`, **not** an interactive login shell. Non-interactive PATH is typically `/usr/bin:/bin` and does **not** load Linuxbrew (`~/.zprofile` / brew shellenv).

Remote launcher probe (`remote_scripts.rs` `build_remote_launcher_probe_script`) uses the same `command -v node`.

POSIX `npx-cli.js` well-known list (`argv.rs` `NPX_JS_POSIX_WELL_KNOWN`) includes `/opt/homebrew/lib/...` (Apple Silicon Homebrew) and **does not** include:

- `/home/linuxbrew/.linuxbrew/bin` on PATH
- `/home/linuxbrew/.linuxbrew/lib/node_modules/npm/bin/npx-cli.js`
- relative `../lib/node_modules/npm/bin/npx-cli.js` from `$PREFIX/bin` (Homebrew/Linuxbrew layout)

So even after `command -v node` finds Linuxbrew, the npx JS probe can still miss `npx-cli.js`.

## Implication for this task

Unpinning and spawning `npx skills` on Remote **fails closed** if doctor/launcher keep using the SSH default PATH. PATH augmentation for reviewed brew prefixes is in-scope, not a follow-up. Do **not** run unbounded `bash -lc` / `zsh -lic` as the Node finder.
