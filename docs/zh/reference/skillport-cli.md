# SkillPort CLI

`skillport-cli` 是 SkillPort 本机 Local target 的命令行入口。它与桌面端共用同一个
SQLite 数据库、中央技能库、受保护的 GitHub 凭据、安装服务和跨进程 mutation lock。

当前版本的 CLI 不管理 SSH 或 WSL target。`just ci` 等仓库开发命令另见
[CLI：just](./cli-just)。

## 运行或安装

在仓库 checkout 中直接运行，无需先安装：

```powershell
npm run cli -- skills list
```

构建 release binary：

```powershell
npm run build:cli
```

Windows 产物位于 `src-tauri/target/release/skillport-cli.exe`；macOS 或 Linux
产物位于 `src-tauri/target/release/skillport-cli`。

从当前 checkout 安装到 `PATH`：

```powershell
npm run install:cli
```

等价的 Cargo 命令是：

```powershell
cargo install --path src-tauri --bin skillport-cli --locked --force
```

桌面端 NSIS 安装器不会自动把 CLI 加入 `PATH`。桌面 binary 是 `skillport`，命令行
binary 是 `skillport-cli`。

## 命令结构

```text
skillport-cli [--json] [--lang en|zh] skills <command>
```

| 全局选项 | 含义 |
| --- | --- |
| `--json` | 为脚本输出带版本号的单行 JSON envelope。 |
| `--lang en\|zh` | 选择英文或中文 human output 标签，默认 `en`。 |
| `--help` | 显示当前命令的帮助。 |
| `--version` | 输出 CLI 版本。 |

Windows 上只要安装目录已经加入 `PATH`，`skillport-cli.exe` 与 `skillport-cli`
写法等价。

## 查看中央技能

### 列表

列出本机中央技能库中的所有技能：

```powershell
skillport-cli skills list
```

### 详情

查看一个中央技能：

```powershell
skillport-cli skills show <reference>
```

`<reference>` 按以下顺序解析：

1. 完全匹配不可变 `uid`；
2. 完全匹配 slug / skill id；
3. 唯一且区分大小写的技能名称。

如果同一名称匹配多个中央技能，命令以 code `3` 退出。此时使用 `list` 输出中的
`uid` 或 slug 消除歧义。

### 搜索

搜索远端 skills.sh catalog：

```powershell
skillport-cli skills search "react" --limit 10
```

`--limit <number>` 可选。搜索不会导入或安装技能。

## 导入技能

install 接受精确 skills.sh shorthand 或 GitHub URL：

```powershell
skillport-cli skills install vercel-labs/agent-skills@react-best-practices
skillport-cli skills install "https://github.com/openai/skills/tree/main/skills/docs"
```

支持的 source：

| Source | 行为 |
| --- | --- |
| `owner/repo@skill` | 解析并导入一个精确 skills.sh 技能。 |
| `https://github.com/...` | 预览并导入仓库或 tree URL 下发现的技能。 |

本地文件路径和非 GitHub URL 会被拒绝。GitHub 鉴权复用 SkillPort 中已经配置的
凭据；CLI 不提供 token 参数。

### 重复项安全规则

已有中央技能不会被静默覆盖：

```powershell
skillport-cli skills install owner/repo@skill --replace
```

- 未传 `--replace` 时，重复项以 code `3` 退出。
- `--replace` 显式允许覆盖。
- 一个 GitHub URL 发现多个技能时，批量覆盖还必须传入 `--yes`。

### 导入后同步

`--sync` 会先导入技能，再安装到 Agent 目录：

```powershell
skillport-cli skills install owner/repo@skill --sync --agent codex --method copy
```

`--agent <id>` 可以重复传入。使用 `--sync` 但未指定任何 `--agent` 时，会选择
除 Central 外所有已启用的本机 Agent。

## 同步中央技能

写入前先预览同步计划：

```powershell
skillport-cli skills sync <uid-or-slug> --agent codex --method copy --dry-run
```

应用同一计划：

```powershell
skillport-cli skills sync <uid-or-slug> --agent codex --method copy
```

同步多个 reference 或整个中央技能库：

```powershell
skillport-cli skills sync <ref-1> <ref-2> --dry-run
skillport-cli skills sync --all --dry-run
```

| 选项 | 含义 |
| --- | --- |
| `[REFERENCES]...` | 一个或多个中央技能 `uid`、slug 或唯一名称。 |
| `--all` | 选择全部中央技能，不能与 references 同时使用。 |
| `--agent <id>` | 限定为一个已启用的本机 Agent；可重复指定多个。 |
| `--method auto\|symlink\|copy` | 安装方式，默认 `auto`。 |
| `--dry-run` | 返回目标路径和安装方式，不修改数据库或文件系统。 |

未传 `--agent` 时，sync 会面向除 Central 外所有已启用的本机 Agent。执行范围较大的
`--all` 操作前应先使用 `--dry-run`。

## JSON 与退出码

自动化脚本使用 `--json`：

```powershell
$result = skillport-cli --json skills list | ConvertFrom-Json
if (-not $result.ok) { exit 1 }
```

成功命令把 JSON 写到 stdout：

```json
{"schemaVersion":1,"ok":true,"data":{},"warnings":[]}
```

错误把 JSON 写到 stderr：

```json
{"schemaVersion":1,"ok":false,"error":{"code":"skill.not_found","message":"...","details":{}}}
```

脚本应根据 `ok`、`error.code` 和进程退出码分支。面向人的 `message` 文案不是稳定的
机器契约。

| 退出码 | 含义 | 代表性 code |
| --- | --- | --- |
| `0` | 成功 | 无 |
| `1` | 内部 service 或数据库失败 | `internal.error` |
| `2` | source、method 或 sync scope 无效 | `input.invalid` |
| `3` | 技能缺失、有歧义或重复 | `skill.not_found`、`skill.ambiguous`、`skill.duplicate` |
| `4` | 另一个进程持有 Central mutation lock | `mutation.busy` |
| `5` | 批处理完成但有部分失败 | 含失败项的 success envelope |

## 与桌面端协作

桌面端和 CLI 可以同时运行。所有 mutation 共享 Central mutation lock；发生并发写入时，
CLI 会安全地以 code `4` 失败，而不是并发修改中央库。等待另一个操作结束后重试即可。

CLI mutation 不会向已经打开的桌面窗口推送实时事件。需要在对应桌面视图中手动刷新，
重新加载技能或安装状态。

## 当前限制

- 只支持 Local target，没有 SSH 或 WSL CLI 命令。
- 只接受 GitHub URL 或 `owner/repo@skill` 导入，不用本地路径猜测 source。
- 桌面安装器不会自动修改 `PATH`。
- CLI mutation 后不会实时刷新桌面窗口。
