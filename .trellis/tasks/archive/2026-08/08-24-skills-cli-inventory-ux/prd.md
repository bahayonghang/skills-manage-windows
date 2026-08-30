# Skills CLI 全局库存页与静默刷新

## Goal

把 Skills CLI 全局页改成「本机已装什么」的库存面：默认列出 lock 证明的 `npx skills add -g` 技能（含 PIN 默认 copy 安装），给出可扫读的统计，并把规范路径放到内容下方。刷新与 CLI 探测在后台进行，Windows 上不得把控制台闪到前台。doctor 失败或库存读取失败都不得再被伪装成「未安装」。

用户价值：打开页面立刻看见本机全局技能，而不是先面对安装表单和一句无法执行的报错。

## Background

当前页来自归档任务 `08-24-npx-skills-global-manage`。那次把列表绑定为冻结版本 `skills ls -g --json`。实现上 `loadAll` 用 `Promise.all` 绑死 doctor / list / targets（`src/stores/skillsCliStore.ts:65-69`）；list 非零映射成 `skills_cli.cli_unavailable`（`src-tauri/src/services/skills_cli/mod.rs:286-288`）。空态在 `skills.length === 0 && !isLoading` 时渲染（`src/pages/SkillsCliView.tsx:294-295`）。Refresh 走未隐藏窗口的 `ProcessRunner`（`src-tauri/src/targets/runner.rs:244-260`，`process_tree.rs:30` 的 `prepare` 不调用 `hide_child_window`）。

PIN `skills@1.5.23` 在单目标目录时默认 copy，且 copy **不写** `~/.agents/skills/<name>`（见 `research/copy-mode-ownership.md`）。现有 `classify_local_path_origin` 把这类副本标成 `Other`（`lock.rs:167-189`，`tests.rs:469-472`）。库存算法若只认 canonical/junction，会把合法 CLI 安装显示成「lock 有名、无路径、零平台」。

本次把**读路径**改为 lock + 文件系统（含 copy 副本）；**写路径**仍 spawn 官方 CLI。取代归档 R2 的「必须 spawn ls」，不推翻 Local-only、PIN、mutation guard、source 白名单。

## Decisions

- 成员资格只认 lock v3 名字。展示路径与平台归属见 `research/copy-mode-ownership.md`：canonical 优先，否则已映射已检测平台上的同名目录（copy），origin=`SkillsCli` 不是唯一归属条件。
- `skills_cli_list_global` 不为展示而 spawn。返回 snapshot：`{ skills, canonicalRoot, lockPath }`。
- doctor 与 inventory 分轨。`runtimeError` 与 `inventoryError` 都要进页面契约。首次库存失败禁止空态；刷新失败保留旧 `skills` 并显示刷新错误。
- 刷新 stale-while-revalidate。Windows 上 `ProcessTreeGuard::prepare` 必须给即将 spawn 的 `Command` 打 `CREATE_NO_WINDOW`；自动化必须覆盖 prepare 生产路径，不能只测常量。
- 统计是当前 snapshot 普查。`sourceType` 规范化桶见 copy-mode 调研。图表纯 SVG。
- 信息层级用 DOM 顺序验收，不验收视口像素。有库存时安装区默认折叠且位于库存区块之后。
- 本任务不强制 `--copy` / symlink。Local leftover 对 lock 名字下的 mapped agent 副本做最小保护扩展。
- 继续 Local-only、PIN `skills@1.5.23`、不默认 `--all` / `--agent '*'`、stderr 不进 IPC。启动器诊断只进 runtime log。
- IPC 形状变化必须跑 `pnpm ipc:codegen`（写 `src/lib/ipc/generatedCommandMap.ts`）和 `pnpm docs:gen`（写 `docs/architecture/_generated/`）。禁止手改生成物。

## Requirements

- R1: `SkillsCliView` 主栏 DOM 顺序为：页头（短标题 + 后台刷新）→ 运行时/库存错误区 → KPI 与统计图 → 已装列表（`data-testid="skills-cli-inventory"`，`UnifiedSkillCard` `variant="skillsCli"`）→ 次级安装区（`data-testid="skills-cli-install"`）→ 底部路径（canonical 根与 lock 文件，`text-ui-meta`）。文案走 `src/i18n/`。组件不直接 `invoke()`。
- R2: `skills_cli_list_global` 只读 lock + 文件系统 + `install_targets` 同款平台集合。缺 lock 或空 lock 返回空 snapshot（空态，不是错误）。每条含 name、path（canonical 或 copy 规则）、`installKind`（`canonical` | `copy` | `missing`）、lock 的 source / sourceUrl / 原始 sourceType、规范化 `sourceTypeBucket`、agents（displayName 列表，含 copy 目录命中）。Renderer 不读 lock。copy 归属不得要求 `classify_local_path_origin == SkillsCli`。
- R3: 统计只从本次 snapshot 派生。KPI：已装数 = `skills.length`；已链接平台技能数 = `agents.length >= 1` 的条数（copy 与 junction 都算链接）；来源种类数 = 规范化 `sourceTypeBucket` 去重个数。条形图：每个 detected∩mapped 平台上 `agents` 命中次数，零值桶仍绘制。来源图：每个规范化桶的技能数，含 `unknown`。SVG `role="img"`、aria 概述、每个数据点 `<title>`。不引入 recharts/d3，不加 `backdrop-filter`。
- R4: 有技能且无 `inventoryError` 时，安装区默认折叠，且在 DOM 中位于库存区块之后。空态（无错误、`skills.length === 0`）默认展开安装区。canonical 根与 lock 路径固定在主栏底部。不把「首屏可见性」绑定到窗口尺寸或缩放。
- R5: Refresh 与首次进入不因 doctor 失败而清空 `skills`。已有 `skills` 时 `isRefreshing` 为真不得拆掉列表。沿用 Skill Usage 式后台提示。
- R6: `ProcessTreeGuard::prepare` 在 Windows 上对传入 `Command` 设置 `CREATE_NO_WINDOW`（`0x08000000`）。`ProcessRunner::run` 必须经过该 prepare。Skills CLI doctor/preview/add/remove 禁止 `npx.cmd` / `cmd /c`。list 不 spawn。孙进程控制台列为人工验证，不作为自动化绿灯的唯一证据。
- R7: `cli_unavailable` 只表示 PIN 包/npx JS 无法作为写路径运行。list 读失败不得用该 code。store 分离 `runtimeError` 与 `inventoryError`。doctor 文案只出现一次。首次 `inventoryError` 且 `skills.length === 0` 必须渲染库存错误，禁止 `skillsCli.empty`。刷新失败保留旧列表并显示可重试库存错误。公开 message 走 `formatBackendError` / i18n。
- R8: preview/add/remove 仍走冻结 CLI、现有 source 白名单（`parse_skill_source`）、平台映射闭包、exclusive job + Local mutation guard。失败 toast + 内联错误；卸载确认失败时保持打开。成功后重读 lock snapshot。本任务不放宽白名单、不加 `--all` / `--agent '*'`。
- R9: Windows `npx-cli.js` 候选覆盖官方 Node 布局 `node_dir/node_modules/npm/bin/npx-cli.js`。shim canonicalize 后继续查已记录的全局 npm 根。失败时候选路径只写 runtime log，IPC `IpcError.message` 不得包含这些路径、HOME 或 URL。
- R10: 更新 `skills-cli-global.md` 与 `process-supervision.md`。IPC 变更后提交 `pnpm ipc:codegen` 与 `pnpm docs:gen` 的生成物，并跑对应 `:check`。`just ci` 通过。
- R11: Local leftover 在 lock 含 name 时，不得把 `{mapped_detected_agent.global_skills_dir}/<name>` 列为可删项。远程 leftover 仍 `cli_lock_protect=false`。无 lock 名的副本仍可清理。不得整棵排除 Universal 根。

## Acceptance Criteria

- [ ] AC1: [R1][R4] 当 fixture/store 有至少一条 skill 且无 `inventoryError` 时，`getByTestId("skills-cli-inventory")` 在文档顺序上先于 `getByTestId("skills-cli-install")`；安装区 `<details>`（或等价折叠控件）默认不 `open`。不依赖窗口宽高或 `scrollY`。
- [ ] AC2: [R2][R7] lock v3 含 N 个名字且对应 canonical **或** 单个已映射平台 copy 目录存在时，即使 doctor 返回 `cli_unavailable`，列表仍有 N 张卡；安装/卸载禁用；`cli_unavailable` 公开句只出现一次。
- [ ] AC3: [R2] 无 lock 或空 lock、且无 lock 名对应的 canonical/copy 目录时，显示 `skillsCli.empty`，不显示 `cli_unavailable`，安装区默认展开。
- [ ] AC4: [R2] fixture：lock 有 `demo`、无 `universal_skills_dir/demo`、仅 `cursor` 的 `global_skills_dir/demo` 为普通目录时，`installKind === "copy"`，`path` 为该 copy 路径，`agents` 含 Cursor 的 displayName。KPI「已链接」计 1。不得因 origin=Other 而 `agents` 为空。
- [ ] AC5: [R2] lock 有名、canonical 与所有 mapped 平台目录均无该文件夹时，仍列出该 name，`installKind === "missing"`，`path === null`，不当成空库存。
- [ ] AC6: [R2] lock 条目含 `source`、`sourceUrl`、`sourceType` 时，卡片/测试能读到这些字段；`sourceType` 为 `"not-a-real-type"` 或缺省时 `sourceTypeBucket === "unknown"`。
- [ ] AC7: [R3] `skills.length === 3` 且其中 2 条 `agents.length >= 1`、规范化桶为 `github`/`github`/`unknown` 时，KPI 渲染 3、2、2。有 mapped 平台零命中时条形图仍有该平台的零值桶（`title` 含 0）。空 snapshot 时图表走 empty 测试 id，不画假时间轴。
- [ ] AC8: [R3] 每个图表 SVG 具有 `role="img"`、非空 `aria-label` 或 `aria-labelledby`，且每个非 empty 数据点有 `<title>`。
- [ ] AC9: [R4] 主栏底部 `data-testid="skills-cli-paths"` 含 snapshot 的 `canonicalRoot` 与 `lockPath`；使用 `text-ui-meta`（或现有 meta token 类名），刷新成功后仍存在。
- [ ] AC10: [R5][R6] 已有列表时点 Refresh：列表节点在 loading 期间仍存在。Windows 自动化：`ProcessTreeGuard::prepare` 之后的 `Command` debug/记录的 creation flags 包含 `0x08000000`；`ProcessRunner::run` 测试路径调用 prepare（不得只断言 `hidden_child_creation_flags() == CREATE_NO_WINDOW`）。
- [ ] AC11: [R7] 首次 `skills_cli_list_global` 拒绝且 store `skills` 为空时，页面出现库存错误（`data-testid="skills-cli-inventory-error"`），**不**出现 `skillsCli.empty`，安装区不按「成功空库存」展开。
- [ ] AC12: [R7] 已有 skills 时 list 刷新失败：旧卡片仍在；出现库存错误；不把 `skills` 置空。
- [ ] AC13: [R7] list 读路径失败的 IPC code 不是 `skills_cli.cli_unavailable`。
- [ ] AC14: [R8] 现有 preview 白名单、空选择拒绝、add argv（`--yes`、PIN、`-g -y -a -s`、无 `--all`/`*`/`npx.cmd`）、卸载确认失败保持打开的测试保持有效或等价覆盖。本任务新增测试不得削弱这些断言。
- [ ] AC15: [R9] temp fixture：shim `node.exe` 旁无 npm、额外候选路径上有 `npx-cli.js` 时 resolve 成功；两处都没有时 `CliUnavailable`。失败 IPC message 不含候选绝对路径。对应测试可用 log sink 断言路径只出现在内部 log。
- [ ] AC16: [R11] Local leftover 扫描：lock 含 `demo` 且 Cursor 下为 copy 目录时，该路径不在可删列表；无 lock 名的 sibling copy 仍在。远程扫描不排除本机 lock 名。
- [ ] AC17: [R1][R10] 中英 i18n；`pnpm ipc:codegen` 后 `generatedCommandMap.ts` 中 `skills_cli_list_global` 的 Result 为 snapshot 而非 `SkillsCliGlobalSkill[]`；`pnpm ipc:codegen:check` 与 `pnpm docs:gen:check` 干净；`just ci` 通过。

## Out Of Scope

- 项目级（非 `-g`）Skills CLI。
- `find` / `use` / `init` / `update`。
- 导入或收编进 Central。
- SSH/WSL 上执行 `npx skills`。
- 在 Rust 中重实现 add/remove/lock **写入**。
- 捆绑 Node；自动升级 PIN。
- 一键 `remove --all` 或默认 `--all` / `--agent '*'`。
- 引入 recharts、d3。
- 时间序列安装趋势。
- 自动化证明 npx 孙进程（`npm.cmd`）窗口为零（人工 Windows 检查，见 implement.md）。
- 强制 PIN 改用 symlink（不添加 `--copy` 覆盖，也不删除 `-y`）。

## Notes

- 复杂任务。修订后的本摘要再次明确批准前不得 `task.py start`。
- 归档 leftover 的「不因位于 Universal 根整棵排除」仍然有效；R11 只增加 lock 名下的 mapped agent copy 排除。
- PIN 仍为 `skills@1.5.23`。领域词仍为 **Skills CLI global**。
- `sourceTypeBucket` 允许值：`github` | `gitlab` | `git` | `mintlify` | `huggingface` | `local` | `well-known` | `unknown`。
