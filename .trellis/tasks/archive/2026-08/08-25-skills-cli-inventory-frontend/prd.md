# Skills CLI 库存优先页面前端落地与 doctor 非阻塞

## Goal

用户打开 Skills CLI 全局页立即看到本机 `npx skills add -g` 安装的全部技能与统计图（本机 lock v3 实测 51 条），而不是先面对安装表单和一句「无法执行 Skills CLI 软件包」。doctor 探测失败只降级写操作可用性，不再清空已成功读取的库存、不再整页报错。

## Background

### 归档任务只交付了后端

归档任务 `08-24-skills-cli-inventory-ux`（completed 2026-08-25）的 PRD 定义了库存优先页面（其 R1/R3/R4/R5/R7），但实际只落地了后端：

| 交付物 | 证据 |
| --- | --- |
| lock+FS 库存投影，list 不 spawn | `bffffc09`；`src-tauri/src/services/skills_cli/inventory.rs`；`mod.rs:269-297` |
| npx 子进程非交互环境（CI=1 等） | `2a28ce5b`；`runner.rs:88-91` |
| spawn 隐藏窗口 CREATE_NO_WINDOW | `process_tree.rs:158-159` + `process_tree.rs:182-192` 测试 |
| spec 契约更新 | `d73958a0`；`.trellis/spec/backend/skills-cli-global.md` |

前端从未实现：`src/pages/SkillsCliView.tsx` 仅有初始提交 `436e6c9b`，仍是「安装表单在首屏、库存在底部」布局，无 KPI、无图表、无 `skills-cli-inventory` / `skills-cli-install` / `skills-cli-paths` 测试锚点；`src/stores/skillsCliStore.ts:62-82` 仍用 `Promise.all` 绑死 doctor/list/targets 且只有单一 `error` 字段。

### 截图错误的因果链（当前 HEAD 仍可复现）

```
skills_cli_doctor probe 非零（离线 / 代理 / npm cache 冷启动失败等）
    → SkillsCliError::CliUnavailable（mod.rs:246-248）
    → loadAll 的 Promise.all 整体拒绝，丢弃 list/targets 已成功的结果
      （skillsCliStore.ts:65-81，skills 保持 []）
    → 同一句错误渲染两次：doctor 行（SkillsCliView.tsx:193-202）+ role="alert"（:206-210）
    → 空态「No Skills CLI global skills are installed.」（:294-295）
```

### 本机实测（2026-08-25，排除后端数据问题）

- `~/.agents/.skill-lock.json` 存在：version 3、51 条，含 `source`/`sourceType`/`sourceUrl`；`~/.agents/skills/` 有 55 个目录。`skills_cli_list_global` 数据路径健康。
- launcher 解析成功：node v26.7.0 位于 `D:\GreenSoftware\node\node.exe`，`npx-cli.js` 命中第一候选 `node_dir/node_modules/npm/bin/npx-cli.js`（argv.rs:182-201）。
- doctor probe `node <npx-cli.js> --yes --package=skills@1.5.23 -- skills --help`（含 CI=1 等 env）冷跑 7.4s exit 0、温跑 5.0s exit 0。
- 结论：probe 在 shell 上下文成功；应用内失败只发生在 probe 非零的运行环境（网络/缓存差异），且每次 Refresh 都重新 spawn doctor 两次（`node --version` + probe，温态约 5s）。错误 UX 放大了偶发 probe 失败的破坏面。

## Decisions

- 后端 IPC 形状不变（snapshot 已就位），本任务只改前端 store/view/组件/i18n 与 doctor 失败日志；不跑 `ipc:codegen`，无 Tauri 命令变更则不触发 `docs:gen`。
- doctor 与库存分轨：`runtimeError`（doctor/写路径）与 `inventoryError`（list/targets 读取）分离；`cli_unavailable` 公开句只出现一次，只禁用安装/卸载按钮。
- 统计是当前 snapshot 普查，图表纯 SVG（复用 Dashboard `ActivityPanel` 手绘柱图模式），不引入 recharts/d3。
- probe 失败原因写 runtime log（tracing），stderr/URL 不进 IPC message（延续 redaction-policy）。
- 继续 Local-only、PIN `skills@1.5.23`、stderr 不进 IPC；领域词 **Skills CLI global**。

## Requirements

- R1: `SkillsCliView` 主栏 DOM 顺序：页头（标题 + Refresh 后台刷新）→ 错误区 → KPI 与统计图 → 已装列表（`data-testid="skills-cli-inventory"`，`UnifiedSkillCard` `variant="skillsCli"`）→ 次级安装区（`data-testid="skills-cli-install"`，`<details>` 折叠）→ 底部路径（`data-testid="skills-cli-paths"`，canonical 根 + lock 文件）。文案走 `src/i18n/`；组件不直接 `invoke()`。
- R2: store 分离 `runtimeError` 与 `inventoryError`。doctor 失败不清空 `skills`；list/targets 失败不清空 `skills` 且保留旧数据可重试；刷新期间已有列表不拆除（stale-while-revalidate，沿用 Skill Usage 后台提示形态）。首次 `inventoryError` 且无数据时渲染库存错误，禁止 `skillsCli.empty`。
- R3: 统计只从本次 snapshot 派生。KPI：已装数 = `skills.length`；已链接平台技能数 = `agents.length >= 1` 条数；来源种类数 = `sourceTypeBucket` 去重个数。平台条形图：每个 detected∩mapped 平台 `agents` 命中次数，零值桶仍绘制。来源图：各规范化桶技能数，含 `unknown`。SVG `role="img"`、aria 概述、每数据点 `<title>`；空 snapshot 走 empty 测试 id，不画假轴。
- R4: 有技能且无 `inventoryError` 时安装区默认折叠且位于库存之后；空态（无错误）默认展开。canonical 根与 lock 路径固定主栏底部。
- R5: `cli_unavailable` 只表示 PIN 包/npx JS 无法作为写路径运行：出现时安装/卸载按钮禁用并给出一行原因，库存列表照常渲染；doctor 状态行只出现一次该句。
- R6: doctor probe 非零时在 Rust 侧写 tracing warn（退出状态 + stderr 截断摘要），IPC message 不含 stderr/路径/URL；launcher 解析失败维持现有 warn（argv.rs:241-243）。
- R7: 中英 i18n 同步；`just ci` 通过。

## Acceptance Criteria

- [ ] AC1: [R1][R4] store 有 ≥1 条 skill 且无 `inventoryError` 时，`skills-cli-inventory` 在文档顺序上先于 `skills-cli-install`；安装区 `<details>` 默认不 `open`。不依赖窗口宽高。
- [ ] AC2: [R2][R5] doctor 拒绝 `skills_cli.cli_unavailable` 而 list 成功返回 N 条时，页面渲染 N 张卡，该公开句只出现一次，安装/卸载按钮禁用。
- [ ] AC3: [R2] 无 lock（空 snapshot）且无错误时显示 `skillsCli.empty`，不显示 `cli_unavailable`，安装区默认展开。
- [ ] AC4: [R2] 已有 skills 时点 Refresh：loading 期间列表节点仍存在（stale-while-revalidate）。
- [ ] AC5: [R2] 首次 `skills_cli_list_global` 拒绝且 skills 为空时，出现 `data-testid="skills-cli-inventory-error"`，不出现 `skillsCli.empty`，安装区不按成功空库存展开。
- [ ] AC6: [R2] 已有 skills 时 list 刷新失败：旧卡片仍在、出现库存错误、`skills` 不被置空。
- [ ] AC7: [R3] `skills.length === 3` 且 2 条 `agents.length >= 1`、桶为 `github`/`github`/`unknown` 时 KPI 渲染 3、2、2；有平台零命中时条形图仍有零值桶（`title` 含 0）。
- [ ] AC8: [R3] 每个图表 SVG 有 `role="img"`、非空 aria 标注、每个非 empty 数据点有 `<title>`；空 snapshot 走 empty 测试 id。
- [ ] AC9: [R1] `data-testid="skills-cli-paths"` 含 snapshot `canonicalRoot` 与 `lockPath`，刷新成功后仍存在。
- [ ] AC10: [R6] probe 非零时 tracing warn 含退出状态；`IpcError.message` 仍为 `skills_cli.cli_unavailable` 公开句，无 stderr 内容（Rust 测试断言 log sink / error payload）。
- [ ] AC11: [R7] 现有 preview 白名单、空选择拒绝、add payload、卸载确认失败保持打开、非 Local 侧边栏隐藏的测试保持有效；doctor-error 旧用例改为断言「库存仍渲染 + 错误一次」。

## Out Of Scope

- 后端 list/inventory/lock 投影改动（`bffffc09` 已交付，本任务只加 doctor 失败日志）。
- PIN 升级、捆绑 Node、SSH/WSL 执行、项目级（非 `-g`）技能。
- 引入 recharts/d3、时间序列安装趋势、`backdrop-filter`。
- 重写 add/remove/lock 写入路径；放宽 source 白名单；`--all` / `--agent '*'`。
- 自动化证明 npx 孙进程（`npm.cmd`）窗口为零（人工 Windows 检查，R6 已有进程级测试覆盖 prepare 路径）。

## Notes

- 复杂任务：`design.md` 与 `implement.md` 已建。Inline 工作流，`implement.jsonl` / `check.jsonl` 门禁按 inline 规则跳过。
- 本 PRD 为最终收敛版；实现前需用户对规划摘要明确批准。
