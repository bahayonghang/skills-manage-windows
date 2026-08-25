# Skills CLI 全局页：库存空白、控制台闪烁、cli_unavailable

日期：2026-08-24  
任务：`.trellis/tasks/08-24-skills-cli-inventory-ux`  
结论：三个用户问题是同一条错误数据流上的三层失败，不是三个无关缺陷。

## 用户观察到的现象

1. 页面默认被安装表单占据；已装技能区和路径信息沉底或空白。截图文案为 “No Skills CLI global skills are installed.”，同时顶部副标题把 `~/.agents/skills/` 和 PIN `skills@1.5.23` 挤在一起。
2. 点击 Refresh 时 Windows 控制台窗口闪到前台。
3. 主错误为 “The Skills CLI package could not be executed.”（中文：无法执行 Skills CLI 软件包）。同一句话出现两次：doctor 行和 `role="alert"`。

## 因果链

```
Refresh / 首次进入
    → skillsCliStore.loadAll Promise.all([doctor, list, install_targets])
        → list_global / doctor 都 spawn node.exe + npx-cli.js
            → ProcessRunner 不调用 hide_child_window
                → node.exe（CONSOLE 子系统）分配控制台并抢前台     ← 问题 2
            → list 非零被映射成 SkillsCliError::CliUnavailable     ← 问题 3 的误分类
        → Promise.all 任一拒绝
            → 丢弃全部结果，skills 保持 []
                → 空态 + 全页错误，安装表单仍在首屏               ← 问题 1 + 3
```

归档任务 `08-24-npx-skills-global-manage` 的 R2 **主动选择了** “列表必须走 `skills ls -g --json`；CLI 不可用则整页错误、不回退”。本机 lock 与 `~/.agents/skills/` 已足够展示库存，却被这条合同挡住。用户现在的产品意图是反过来的：默认看见本机已装什么。

可行性调研当时就写过另一条路：`research/npx-skills-global-feasibility.md` §4 路径 A「lock + `~/.agents/skills/` 扫描，不要求运行时 Node」。MVP 选了路径 C（包装 npx）。本次任务把**读路径**改回 A，**写路径**仍走 C。

## 问题 1 — 信息架构与数据源

| 现状 | 证据 |
| --- | --- |
| 安装表单是主栏第一个 `<section>`，库存卡片在其后 | `src/pages/SkillsCliView.tsx:212`–`:318` |
| 空态在 `skills.length === 0 && !isLoading` 时渲染 | `src/pages/SkillsCliView.tsx:294` |
| doctor 句与 `role="alert"` 重复 | `src/pages/SkillsCliView.tsx:193`–`:209` |
| lock 解析只保留名字 | `src-tauri/src/services/skills_cli/lock.rs:20`–`:23` |
| copy 目录 origin=Other | `src-tauri/src/services/skills_cli/lock.rs:167`–`:189`；`tests.rs:469`–`:472` |
| `loadAll` 用 `Promise.all` 绑死三条 IPC | `src/stores/skillsCliStore.ts:65`–`:69` |

空态文案在 `isLoading === false && skills.length === 0` 时渲染，与 `visibleError` 并存。doctor 失败时 list 的成功结果也被丢掉，因此即使用户机器上 lock 有条目，UI 仍显示「尚未安装」。

现有可复用图案：`UsageMetricStrip`（KPI）、Dashboard `ActivityPanel` 手绘 SVG 柱（`role="img"` + `<title>`）、Skill Usage 的 `backgroundScanning` 刷新提示。不要新开图表库。

## 问题 2 — 刷新把终端闪到前台

`hide_child_window` 只打在：

- `ConnectedSshTarget::base_command`（`src-tauri/src/targets/exec.rs:206`）
- `ConnectedWslTarget::base_command`（`src-tauri/src/targets/exec.rs:487`）
- WSL discovery（`src-tauri/src/targets/wsl_discovery.rs:37`）

Skills CLI 走 `NodeProcessRunner` → `ProcessRunner::run`（`src-tauri/src/targets/runner.rs:244-260`）。该路径：

- 不调用 `hide_child_window`
- Windows 上 `node.exe` 是 CONSOLE 子系统，从 GUI 父进程 spawn 会 `AllocConsole` 并激活
- Job Object（`process_tree.rs`）只负责杀进程树，**不隐藏窗口**
- `CREATE_NO_WINDOW` 不遗传给孙进程。`npx-cli.js` 仍可能再 spawn `npm.cmd` → `cmd.exe`，那些窗口也会闪

测试 `windows_hidden_window_flag_matches_create_no_window` 只断言常量等于 `0x08000000`，不断言 `ProcessRunner` 实际打了 flag。

进程监督契约 `.trellis/spec/backend/process-supervision.md` §1 范围写成「SSH、WSL 与 WSL discovery」，本地 `node` 是后加入口，隐藏窗口要求没有写进契约。这是契约缺口，不是「SSH 已经藏了所以 CLI 也藏了」。

刷新触发三次 spawn：doctor 的 `node --version`、doctor 的 `skills --help`、list 的 `skills ls -g --json`。用户每点一次 Refresh，最多闪三次。

## 问题 3 — 「无法执行 Skills CLI 软件包」

公开句来自 `ipc_error.rs` 的 `skills_cli.cli_unavailable`，对应 `SkillsCliError::CliUnavailable`。

该变体被用在三条完全不同的失败上：

1. 解析器找不到 `npx-cli.js`（`src-tauri/src/services/skills_cli/argv.rs` `resolve_node_launcher_from_dirs`）
2. doctor 的 PIN probe（`skills --help`）非零
3. **`list_global` 任何非零**（`src-tauri/src/services/skills_cli/mod.rs:286-288`）——这违反 `.trellis/spec/backend/skills-cli-global.md` 错误矩阵：list 非零 / 不可解析应走 `internal.unexpected`，不是 `cli_unavailable`

Windows `npx-cli.js` 候选几乎全是 Unix 路径（`/usr/lib/node_modules/...`）。官方 Node 安装器的 `node_dir/node_modules/npm/bin/npx-cli.js` 能命中。用户能看到控制台闪烁，说明 **node 和 npx-cli.js 已经被找到并 spawn 成功**。因此截图上的 `cli_unavailable` **不是**「找不到 npx」，而是 probe 或 list 的子进程退出码非零。

常见非零原因（本任务必须可诊断，不得把 stderr 送进 IPC）：

- 首次 `npx --yes --package=skills@1.5.23` 需要写 npm cache / 访问 registry，GUI PATH、代理、离线、或只读 cache 会导致失败
- 无 TTY + 未设 `npm_config_yes` / `CI=1` 时，部分 npm 版本仍交互失败
- `skills ls -g --json` 在 PIN 1.5.23 是合法命令（upstream `src/list.ts` 支持 `--json` 与 `-g`）；失败更可能是 npx 包装层而不是 flag 拼错

`Promise.all` 把上述失败升级成「整页不可用」。Vitest `shows npx missing as a localized doctor error` 只断言 alert 文案，不断言库存仍应渲染——它把错误合同冻成了当前错误 UX。

## 建议的产品合同（取代归档 R2 的列表条款）

| 操作 | 数据源 | 是否 spawn |
| --- | --- | --- |
| 默认库存、KPI、图表、路径 | v3 lock + canonical **或** mapped agent copy 目录 | 否 |
| doctor（安装/卸载是否可做） | node 版本 + PIN probe | 是，隐藏窗口，失败非阻塞 |
| preview / add / remove | 冻结 CLI | 是，隐藏窗口，失败 toast + 内联，不清空库存 |
| Refresh | 先重读 lock/FS；doctor 后台 | 库存路径无控制台；doctor 无可见窗口 |

## 明确不在本次范围

- 捆绑 Node 进安装包
- SSH/WSL 上跑 `npx skills`
- 把 CLI canonical 收编进 Central
- 升级 PIN `skills@1.5.23`
- 引入 recharts/d3
- 重写 add/remove/lock 写入
