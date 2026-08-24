# Skills CLI global 页面设计

## 1. Architecture

```text
Sidebar /skills-cli  (hidden unless ActiveTarget::Local)
  SkillsCliView
    skillsCliStore  ──invoke──►  commands/skills_cli.rs
                                   │ reject if not Local
                                   ▼
                                 services/skills_cli/
                                   doctor / list / preview / add / remove
                                   source grammar + argv builder
                                   agent_map (complete builtin closure)
                                   lock_ownership (leftover + origin)
                                   │
              ┌────────────────────┼─────────────────────┐
              ▼                    ▼                     ▼
        LocalNodeRunner      acquire_target          leftover scan
        node.exe + npx JS    mutation_guard          Local-only exclude
        ProcessRequest       (Local, add/remove      by lock evidence
        Job Object           and leftover local)
```

## 2. Boundaries

| 层 | 职责 | 禁止 |
| --- | --- | --- |
| `commands/skills_cli.rs` | TargetContext、Local 门闩、jobId、operation log、`IpcResult` 映射 | 业务逻辑、直接 `std::process`、`active_db()` 与 `active_target()` 混用 |
| `services/skills_cli` | argv、白名单、解析 PIN 版本输出、lock 所有权、域错误 | `Result<T, String>`、改 lock schema、Central 安装器 |
| Local node runner | 监督 `node.exe` + npx JS | `npx.cmd` 作为 program、`cmd /c` 字符串、无界 stdout |
| leftover scan | Local 且 lock 命中才排除 | 用本机 lock 保护远程 leftover；按目录根一刀切 |
| 前端 store | 唯一 `invoke` 调用方 | 组件直接 `invoke` |

## 3. Local-only and TargetContext

每个 `skills_cli_*` command 先 `state.resolve_target_context()`。

| ActiveTarget | 行为 |
| --- | --- |
| Local | 继续 |
| SSH / WSL | 立即 `skills_cli.local_target_only`；不检测远程平台、不 spawn、不读本机 lock |

前端：`activeTarget.kind !== "local"` 时侧栏不展示或禁用入口。

leftover：`scan_deleted_platform_copies_with_pool` 增加显式 `ActiveTarget`（或 `cli_lock_protect: bool`）。仅 `Local` 时读取本机 lock 做排除。SSH/WSL leftover 扫描零 CLI 保护。测试：同一 skill id 在远程 leftover 中仍可删除。

## 4. Mutual exclusion

Exclusive job family **不能** 阻止 leftover apply（`central_update_jobs`）或 Central install。不同 family 允许并行（`.trellis/spec/backend/exclusive-job-lifecycle.md`）。

共享文件系统互斥：Local `acquire_target_mutation_guard`（`.trellis/spec/backend/central-mutation-lock.md`）。

| 操作 | exclusive job | target mutation guard |
| --- | --- | --- |
| doctor / list / preview | 否（短读）或可选 | 否 |
| CLI add / remove | `skills_cli` family + renderer `jobId` | **是**，覆盖整个 node 进程 |
| Central install / uninstall | 否 | 已有 |
| leftover 本地 apply | `central_update_jobs` | **本任务补上**，在删除循环前获取、结束后释放 |
| leftover 远程 apply | `central_update_jobs` | 获取对应 remote target guard |

锁顺序（两端一致，避免死锁）：

1. 该操作的 exclusive job lease（若有）
2. `acquire_target_mutation_guard`
3. 写盘 / spawn
4. Drop guard，再 Drop lease

获取等待沿用 `DEFAULT_CENTRAL_MUTATION_TIMEOUT`（10 s）。持有时间可以覆盖 BulkTransfer（15 min）。对方等待超时 → `skills_cli.busy` 或现有 Central mutation Busy/Timeout。

测试：假 runner 卡住 add 时，install_skill 与 leftover 本地 apply 必须 Busy/Timeout，不得删除同一路径。

## 5. IPC

| 命令 | 作用 |
| --- | --- |
| `skills_cli_doctor` | Node ≥ 22.20.0 且能跑 PIN 包 |
| `skills_cli_list_global` | `skills ls -g --json` |
| `skills_cli_install_targets` | Local 检测 ∩ 已映射；标记 defaultSelected |
| `skills_cli_preview_source` | `skills add <source> --list` |
| `skills_cli_add_global` | jobId + source + skillNames + skillPortAgentIds |
| `skills_cli_remove_global` | jobId + skillName |
| `cancel_skills_cli_job` | 取消 family 作业 |

`add_global` 在 service 内映射 `--agent` id。前端不传 argv。

`SkillsCliError` → 固定信封（登记 `legacy_code_message` / mapper）：

| variant | code | retryable | public message（英文权威，i18n 另译） |
| --- | --- | --- | --- |
| 非 Local | `skills_cli.local_target_only` | false | Skills CLI is available only on the Local target. |
| 无 Node | `skills_cli.node_missing` | false | Node.js 22.20 or later is required. |
| PIN 失败 | `skills_cli.cli_unavailable` | false | The Skills CLI package could not be executed. |
| source 非法 | `skills_cli.source_invalid` | false | The skill source is not allowed. |
| 预览解析失败 | `skills_cli.preview_unparsed` | false | The skill preview could not be parsed. |
| 空选择 | `skills_cli.selection_empty` | false | Select at least one skill and one platform. |
| agent 无映射 | `skills_cli.agent_unmapped` | false | That platform cannot be targeted by Skills CLI. |
| 忙 | `skills_cli.busy` | true | Another skill operation is using this target. |
| 超时 | `skills_cli.timeout` | false | The Skills CLI command timed out. |
| 取消 | `skills_cli.cancelled` | false | The operation was cancelled. |
| 其它 | `internal.unexpected` | false | 固定摘要；无 stderr |

stdout/stderr/URL 只进已 redaction 的 runtime log，不进 IPC message。

## 6. Argv and Windows launcher

PIN：`skills@1.5.23`。常量 `SKILLS_CLI_NPM_SPEC`。

Program：解析 `node.exe`（Windows）或 `node`（POSIX）。`argv`：

```text
<node> <npx-cli.js> --yes --package=skills@1.5.23 -- skills add <source> -s … -g -a … -y
```

npx-cli.js 来自同机 npm 安装（例如 Node 安装目录下 `node_modules/npm/bin/npx-cli.js`）。找不到则 `skills_cli.cli_unavailable`。

禁止 `Command::new("npx.cmd")` 与 `cmd.exe /c` 拼接。

source 语法（拒绝即 `skills_cli.source_invalid`）：

- 允许：`^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$`；可选 `@` + skill 名 `[A-Za-z0-9_.: -]+`
- 允许：`https://github.com/`、`https://gitlab.com/` 后跟无查询串的 path
- 允许：`git@github.com:owner/repo.git`（无空格）
- 拒绝：`& | ^ % ! < > " \n \r`、`-c`、空格开头的 shell

测试输入：合法 GitHub/SSH/HTTPS；对抗 `& | ^ % ! "`。

list/preview policy = Standard（120 s / 8 MiB）。add/remove = BulkTransfer（15 min / 32 MiB）。stderr cap 保持 1 MiB。

`--list` parser 用 `skills@1.5.23` 录制的 stdout fixture。失败 → `skills_cli.preview_unparsed`。

## 7. Agent mapping closure

候选：`is_detected && mapped`。默认勾选：`is_detected && is_enabled && mapped`。表驱动测试：seed 中每个 builtin id 必须出现在下表。

已映射（SkillPort id → CLI `--agent`）：

| SkillPort id | CLI `--agent` |
| --- | --- |
| `claude-code` | `claude-code` |
| `codex` | `codex` |
| `grok` | `grok` |
| `cursor` | `cursor` |
| `gemini-cli` | `gemini-cli` |
| `trae` | `trae` |
| `factory-droid` | `droid` |
| `junie` | `junie` |
| `qwen` | `qwen-code` |
| `trae-cn` | `trae-cn` |
| `windsurf` | `windsurf` |
| `qoder` | `qoder` |
| `augment` | `augment` |
| `opencode` | `opencode` |
| `kilocode` | `kilo` |
| `amp` | `amp` |
| `kiro` | `kiro-cli` |
| `codebuddy` | `codebuddy` |
| `hermes` | `hermes-agent` |
| `copilot` | `github-copilot` |
| `antigravity` | `antigravity` |
| `antigravity-cli` | `antigravity-cli` |
| `zed` | `zed` |
| `cline` | `cline` |
| `deep-agents` | `deepagents` |
| `firebender` | `firebender` |
| `kimi-code-cli` | `kimi-code-cli` |
| `warp` | `warp` |
| `aider` | `aider-desk` |
| `reasonix` | `reasonix` |
| `openclaw` | `openclaw` |

明确不支持（选择器隐藏；测试锁原因）：

| SkillPort id | 原因 |
| --- | --- |
| `ob1` | 官方 CLI 无对应 `--agent` |
| `qclaw` | 龙虾衍生，无独立 CLI id |
| `easyclaw` | 同上 |
| `autoclaw` | 同上 |
| `workbuddy` | 同上 |
| `central` | 不是平台目标 |

不传 Universal 虚拟组 id。选中的 universal 成员映射后去重。零候选：安装禁用。

自定义平台：无映射则不出现。不在本任务扩展自定义映射 UI。

## 8. Origin and leftover ownership

证据优先级（Local）：

1. `.skill-lock.json`（或 XDG 路径）version 3 中存在该 sanitized name → **CLI 拥有**。
2. symlink/junction 解析后的目标（`paths_equivalent`）落在 `~/.agents/skills/<sanitized>/` **且** 步骤 1 命中 → CLI 平台链接，排除 leftover，origin = skills-cli。
3. 目标落在 Central 根内 → origin = central（现有 SkillPort 安装）。
4. 其它可写平台副本 → standalone / leftover 候选。

禁止：仅因路径前缀是 `~/.agents/skills` 就排除。无 lock 的 Universal 根内副本必须仍能作为 leftover。

Windows junction：用 `symlink_metadata` + `read_link`；比较走 `paths_equivalent`。补 fixture：junction 指向 CLI canonical 且 lock 命中则保护；无 lock 的 junction 不保护。

更新 `.trellis/spec/frontend/platform-origin-classification.md`：`link_type === "symlink"` 不再等于 Central。

## 9. Frontend

- 路由 `/skills-cli`。
- 侧栏：Central 与 Marketplace 之间；非 Local 隐藏。
- `skillsCliStore`。
- 安装：source → preview 技能多选 → 平台多选（默认 enabled）→ 确认。
- `UnifiedSkillCard` variant `skillsCli` + 负例测试。
- 卸载确认（完整删除）。
- `formatBackendError`；fixture 注册全部新命令。

## 10. Compatibility and rollback

- 不迁移 lock schema。
- 不改 Central 路径常量。
- leftover 本地 apply 增加 mutation lock 是行为收紧：与 CLI 并发时更快 Busy，不改变无并发时的删除集合（除 lock 排除外）。
- 回滚页面可保留 leftover lock 排除（更安全）。若回滚排除，必须同时回滚，否则可能再删 CLI canonical。
- README / README_CN / CONTEXT.md 增加 Skills CLI global。
