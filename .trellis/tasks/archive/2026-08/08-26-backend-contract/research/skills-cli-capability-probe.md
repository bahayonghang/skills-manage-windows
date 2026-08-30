# Skills CLI 1.5.23 Capability Probe Protocol

状态：**P1 executed in isolated temp HOME; P2/P3 remain fail-closed.** 产品 argv 不得包含未验证 flag。

## Purpose

为 `08-26-backend-contract` 和 `08-26-update-center` 提供唯一、可审计的真实 CLI 能力证据。实现不得从
help 文案、源码印象或当前用户环境推断行为；每项能力只能标为：

- `VERIFIED_SUPPORTED`：隔离 probe 成功且原始证据完整；
- `VERIFIED_UNSUPPORTED`：隔离 probe 直接证明不支持或行为不满足安全契约；
- `UNVERIFIED`：未执行、下载/网络失败、证据不完整或结果歧义。

`VERIFIED_UNSUPPORTED` 与 `UNVERIFIED` 都必须在产品 capability plan 中 fail closed。

## Isolation and Safety

- 固定 package/version 为 `skills@1.5.23`，记录实际解析版本。
- 使用新建临时 HOME、临时 npm cache 和临时目标目录；禁止读取或修改真实用户 HOME、Skills CLI lock、
  SkillPort DB、credentials 或现有 agent directories。
- 使用公开、无凭据的受控 fixture repository；命令不得携带 PAT、URL credentials、query secret 或私有路径。
- 每次 probe 记录 UTC 时间、平台、命令（安全转义）、exit code、完整 stdout/stderr；若输出含临时绝对路径，
  发布前以稳定 `<TEMP_ROOT>` 代换，并保留原始本地证据位置而不提交秘密。
- 失败只记录失败，不重试到真实用户环境，也不以“看起来可行”升级结论。

## Required Evidence

### P1 — Help and flag surface

采集 pinned `skills@1.5.23` 的 add/remove `--help` 原始输出。逐项判定 `--force`、`--keep-links` 及更新
计划可能使用的其它 flag；未出现在原始 help 且未被行为 probe 直接证明的 flag 保持 `UNVERIFIED`。

### P2 — Pinned full-SHA source

在隔离 HOME 中以受控 repository 的完整 commit SHA 执行 preview/add；记录 lock/source identity、安装结果与
canonical content digest。必须证明执行内容与该 SHA 一致且 branch HEAD 后续漂移不会改变已固定结果，才能标
`VERIFIED_SUPPORTED`。

### P3 — Direct-copy refresh

在隔离 HOME 中建立一个 CLI-owned canonical、一个 managed link、一个 independent direct copy 与一个 conflict
fixture；执行候选 refresh/apply plan。必须证明 direct copy 是否被可靠刷新、managed link 是否保持指向 canonical、
conflict 是否零写入，并记录前后 digest/placement。任何不确定或覆盖 conflict 的行为标
`VERIFIED_UNSUPPORTED`；不得据此设计无 journal 的 remove+add fallback。

## Result Ledger

| Probe | Status | Evidence |
| --- | --- | --- |
| P1 add/remove help | VERIFIED_SUPPORTED (help surface) | Isolated npx `skills@1.5.23` on win32; version stdout `1.5.23`; add/remove help rc=0. Raw: `research/probe-evidence/probe-raw.json` |
| P1 `--force` documented | VERIFIED_UNSUPPORTED | Absent from add and remove help stdout |
| P1 `--keep-links` documented | VERIFIED_UNSUPPORTED | Absent from add and remove help stdout |
| P2 pinned full-SHA source | UNVERIFIED | GitHub commit lookup timed out; no preview/add with a full SHA was executed |
| P3 direct-copy refresh | UNVERIFIED | Not executed. `skills update` is documented but copy/link/conflict preservation is unproven |

完成 probe 后在本表追加命令、时间、解析版本、exit code、原始输出位置和逐项结论；禁止删除失败记录。

## P1 execution record

- Platform: `win32`
- Launcher: `node.exe` + npm `npx-cli.js --yes --package=skills@1.5.23 -- skills` (never `npx.cmd`)
- Isolated temp HOME / npm cache / XDG / TEMP: newly created, then deleted. Skill library under the real user HOME was not used as a target.
- Resolved package version: `1.5.23` (`skills --version`, rc=0)
- Node used to launch: `v26.7.0`
- Evidence file: `research/probe-evidence/probe-raw.json`

### Commands

| Id | UTC start | UTC end | Exit | Notes |
| --- | --- | --- | --- | --- |
| `node --version` | 2026-08-26T09:44:11Z | 2026-08-26T09:44:11Z | 0 | `v26.7.0` |
| `skills --help` | 2026-08-26T09:44:11Z | 2026-08-26T09:44:17Z | 0 | Full CLI help including Add/Remove options |
| `skills add --help` | 2026-08-26T09:44:17Z | 2026-08-26T09:44:18Z | 0 | Same top-level help as `skills --help` for this PIN |
| `skills remove --help` | 2026-08-26T09:44:18Z | 2026-08-26T09:44:19Z | 0 | Dedicated remove help |

### Documented add flags (from raw help)

`-g/--global`, `-a/--agent`, `-s/--skill`, `-l/--list`, `-y/--yes`, `--copy`, `--metadata`, `--subagent`, `--all`, `--full-depth`.

`--copy` is documented as “Copy files instead of symlinking to agent directories”. It is **not** a Skills CLI managed-link primitive and is not used by this child’s junction/symlink path.

### Documented remove flags (from raw help)

`-g/--global`, `-a/--agent`, `-s/--skill`, `-y/--yes`, `--all`.

Remove help states that omitting `-a` will “clean all agent links”. That is why product uninstall **must not** spawn `skills remove`.

### Isolation note (does not upgrade P2/P3)

npx stderr during P1 included `npm error config prefix cannot be changed from project config: <USER_PROFILE>\\.npmrc`. Help stdout was still captured (rc=0). No add/remove mutation was run, so the real skill library was not used as a write target. Subsequent P2/P3 installs stay UNVERIFIED rather than retrying against the real user profile.

## Product capability plan (fail closed)

- Do **not** put `--force` or `--keep-links` in product argv.
- Do **not** spawn `skills remove` for uninstall.
- Do **not** treat `skills update` as a proven copy-preserving refresh.
- Do **not** treat `owner/repo@<full-sha>` as a verified pin until P2 is executed with digest evidence.
- Windows junction create/inspect/remove is a separate native gate, not a CLI flag.
