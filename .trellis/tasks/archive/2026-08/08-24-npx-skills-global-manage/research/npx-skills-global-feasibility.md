# npx skills add 全局安装：可行性调研

日期：2026-08-24  
任务：`.trellis/tasks/08-24-npx-skills-global-manage`  
结论：可行。独立页面 spawn 官方 `npx skills` 管理全局（`-g`）安装；不并入 Central；leftover 不得删除 CLI canonical。

## 一手来源

| 来源 | URL | 用途 |
| --- | --- | --- |
| vercel-labs/skills README | https://github.com/vercel-labs/skills | CLI 命令、全局范围、73+ agent 路径表 |
| `src/installer.ts` | https://raw.githubusercontent.com/vercel-labs/skills/main/src/installer.ts | 全局 canonical 路径、symlink/copy、Windows junction |
| `src/skill-lock.ts` | https://github.com/vercel-labs/skills/blob/main/src/skill-lock.ts | `~/.agents/.skill-lock.json` 与 XDG 回退 |
| `src/constants.ts` | https://raw.githubusercontent.com/vercel-labs/skills/main/src/constants.ts | `AGENTS_DIR = '.agents'`，`SKILLS_SUBDIR = 'skills'` |
| `src/list.ts` | https://raw.githubusercontent.com/vercel-labs/skills/main/src/list.ts | `npx skills ls -g --json` 清单 |
| skills.sh | https://www.skills.sh | 目录与安装入口 |
| SkillPort 仓库 | `CONTEXT.md`、`paths.rs`、leftover scan | 现有 Central / Universal Agents / leftover 语义 |

仓库内旧文档 `docs/research-report.md` §4.1 与 `docs/desktop-design.md` 把 SkillPort 真实源写成 `~/.agents/skills/`。当前代码已经把 Central 迁到 `~/.skillsmanage/skills/`（见 `src-tauri/src/central_migration.rs`）。规划以当前代码与官方 CLI 源码为准。

## 1. `npx skills add -g` 实际落点

官方 CLI 包名是 `skills`，命令是 `npx skills`。仓库：`vercel-labs/skills`（约 29.5k stars，2026-08 观测）。

全局安装（`-g` / `--global`）走用户家目录，不走当前项目。

安装器把 **canonical** 固定为：

```text
join(homedir(), ".agents", "skills", sanitizeName(skillName))
```

即：

```text
~/.agents/skills/<skill-name>/
```

这与 README 里各 agent 的 `globalSkillsDir` 表不是同一件事：

- Canonical 始终是 `~/.agents/skills/<name>/`。
- Agent 专属全局目录（如 `~/.claude/skills/`、`~/.cursor/skills/`）在 **symlink 模式** 下指向 canonical。
- Universal agent 的全局安装 **不再** 写一份 agent 专属软链；文件已经在 canonical 里。
- `--copy` 跳过 canonical，直接复制到各 agent 目录。
- Windows 上软链使用 `junction`。

全局 lock 文件：

```text
$XDG_STATE_HOME/skills/.skill-lock.json
否则 ~/.agents/.skill-lock.json
```

schema `version = 3`。条目含 `source`、`sourceType`、`sourceUrl`、`ref`、`skillPath`、`skillFolderHash`、`installedAt`、`updatedAt`、可选 `pluginName`。旧 version 会被 CLI 当成空 lock（不兼容迁移）。

清单命令：

```text
npx skills ls -g --json
```

每项含 `name`、`path`（canonicalPath）、`scope`、`agents`、以及 lock 中的 `source` / `sourceUrl` / `sourceType`。

生命周期命令：`add`、`remove`、`list`/`ls`、`update`、`find`、`use`、`init`。`remove --global`、`update -g` 对应全局范围。

## 2. 与 SkillPort 路径语义的碰撞

SkillPort 当前权威路径（`CONTEXT.md`、`paths.rs`）：

| 名称 | 路径 | 角色 |
| --- | --- | --- |
| Central Skills | `~/.skillsmanage/skills/` | SkillPort 唯一真实源 |
| Universal Agents | `~/.agents/skills/` | 平台安装目标，不是 Central |
| Local DB | `~/.skillsmanage/db.sqlite` | 元数据 |

`npx skills add -g` 的 canonical 正好落在 SkillPort 的 **Universal Agents 平台目录**。

启动迁移 `migrate_legacy_central_skills_to_private_store` 曾经把旧 `~/.agents/skills` 复制进 Central，并 **保留** 旧目录，避免打断 Codex / Copilot。因此同一台机器上可以同时存在：

- SkillPort Central 副本
- `npx skills` 仍把 `~/.agents/skills` 当自己的真实源

两套工具写同一目录，但所有权模型相反。

## 3. SkillPort 今天会怎样对待这些技能

扫描器对平台目录里的真实目录记 `link_type = "copy"`，对软链记 `link_type = "symlink"`（`services/scanner/mod.rs` `detect_link_type`）。前端来源分类约定：

- `link_type === "symlink"` → 当作 SkillPort / Central 安装
- 其它 → standalone

因此：

- Universal Agents 上的 npx 全局技能是真实目录 → 显示为 **独立安装**。
- Claude / Cursor 等目录里指向 `~/.agents/skills/<name>` 的软链 → 显示为 **中央技能库**，即使目标不是 `~/.skillsmanage/skills/`。

Platform leftover 判定（`scan.rs` `is_deleted_observation_candidate`）：可写、非 plugin、非 native，且 `skill_id` 不在 Central。Update Center 一键清理会删除这些路径。

`npx skills add -g` 写入 `~/.agents/skills/<name>/` 的真实目录，通常不在 Central。它们满足 leftover 条件。一键清理会删掉 npx 的 canonical，并打断所有指向它的 agent 软链。

这是当前产品缺口，也是本任务的安全约束。

## 4. 三条实现路径

### A. 就地清点（推荐 MVP）

SkillPort 读取 lock + `~/.agents/skills/`，把这些技能标成独立来源（建议领域名：**Skills CLI global**），提供列表、详情、卸载、可选导入 Central。

优点：不要求运行时 Node；不改 CLI lock 协议；不把 Central 与 npx canonical 合并。  
代价：卸载/更新语义必须自己定义；不能完整覆盖 CLI 的 73 个 agent。

### B. 收编进 Central

把 npx 全局技能 `ensure_centralized` 进 `~/.skillsmanage/skills/`，之后按现有 Platform install 管理。

优点：复用现有安装、更新中心、卡片。  
代价：`npx skills update -g` 仍写 `~/.agents/skills`；两边会再分叉。与当年把 Central 迁出 `~/.agents/skills` 的决策冲突。

### C. 包装 `npx skills` 子进程

桌面端 spawn `npx skills ls/add/remove/update -g -y --json`。

优点：行为与官方 CLI 一致，含 lock 与 hash 更新。  
代价：运行时依赖 Node/npx；Windows 上 npx 交互与 PATH 不稳定；CLI 提示、telemetry、`gh auth token` 回退会进入桌面进程；官方 CLI 无稳定 Rust API。SkillPort 桌面运行时目前不依赖 Node。

## 5. 可行性结论

| 能力 | 判定 | 说明 |
| --- | --- | --- |
| 列出本机 `-g` 安装 | 可行 | lock + `~/.agents/skills` 扫描，或 `npx skills ls -g --json` |
| 在 UI 中区分来源 | 可行 | 现有 origin 分类不够；symlink≠SkillPort Central |
| 卸载全局 npx 技能 | 可行但有风险 | 必须同时处理 canonical、agent 软链、lock 条目 |
| 从 SkillPort 再执行 `npx skills add -g` | 可行，依赖 Node | MVP 采用：PIN `skills@1.5.23`，npx `--yes` 与 skills `-y` 分层 |
| 用现有 leftover 清理“管理”它们 | 不可行 | 会删除用户的 npx canonical |
| 把 npx 全局目录当作第二套 Central | 不建议 | 与 Path Policy 和 Central 迁移冲突 |

Windows-first：junction 与 Developer Mode 失败回退到 copy；路径比较必须走现有 `paths_equivalent`。不要在 leftover 清理里把 junction 目标当普通平台残留。

## 6. 界面执行官方 CLI：非交互命令契约

官方 README 与 `src/cli.ts` 把 **skills 包** 的非交互形态写死了。那一层的 `-y` **不是** npx 的 `--yes`。

[npx v11](https://docs.npmjs.com/cli/v11/commands/npx)：npx 自己的旗标必须出现在第一个位置参数之前。位置参数之后的 `-y` 会传给 `skills`。本机没有该包时，npx 会先装到 cache 并 **提示确认**，除非带 `--yes`。

冻结版本：npm `skills@1.5.23`（`gitHead` `435076e78988e1e6ec40d00b0b1d76bdbbc5419a`，`engines.node >= 22.20.0`）。parser fixture 锁这一版的 stdout。升级 PIN 是独立决策。

GUI argv 前缀（npx 层）：

```text
npx --yes --package=skills@1.5.23 -- skills <subcommand> ...
```

| 动作 | skills 子命令（接在前缀后） |
| --- | --- |
| 列出全局技能 | `ls -g --json` |
| 预览（不安装） | `add <source> --list` |
| 安装到全局 | `add <source> -s <name>... -g -a <agents...> -y` |
| 完整卸载 | `remove --global <name> -y` |

`--list` 是 Clack 人类输出，不是 JSON。解析器必须用 PIN 版本的 fixture；失败返回 typed 错误。

禁止默认发送：`--all`、`--agent *`、无 `-g` 的 add/remove、无 npx `--yes`、无 skills `-y` 的 add/remove。

Windows：不要把 `npx.cmd` 当作 `Command` 可执行文件（Rust 1.97 会经 `cmd.exe /c`）。解析 `node.exe`，把 npm 自带的 npx JS CLI 作为 `argv[1]`。source 走字符白名单。

SkillPort 现有 `CommandRunner` 只覆盖 SSH/WSL。本机 node/npx 是新的 Local 入口，必须复用 `ProcessRequest` / Job Object / 输出 cap / 取消。stdout/stderr 不得未脱敏进入日志或 IPC。

## 7. 建议的领域词

本任务若落地，应新增一个领域词，写入 `CONTEXT.md`（实施阶段，需用户同意）：

**Skills CLI global**：由 `npx skills add -g`（vercel-labs/skills）安装到 `~/.agents/skills/` 的技能。它不是 Central Skills，也不是 Platform leftover。

不要用 “Universal Agents canonical” 称呼这批技能。Universal Agents 在 SkillPort 里是安装目标。
