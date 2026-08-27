# Skills CLI doctor 门禁降级与警告去噪

父任务：`08-27-skills-cli-availability-remote`
源需求：U1

## Goal

移除 `/skills-cli` 页面上无解释力的 `cli_unavailable` 常驻告警，
并让一次 npx 探测失败不再锁死本不需要 CLI 的读写能力。

## Background

用户报告页面顶部常驻红色告警「The Skills CLI package could not be executed. / 无法执行 Skills CLI 软件包。」，
并要求删除。

仓库证据（详见父任务 `research/current-state-evidence.md`）修正了成因、放大了问题：

- 应用**从不**检测本地 `npm install` 的 `skills` 包。argv 固定为
  `<node> <npx-cli.js> --yes --package=skills@1.5.23 -- skills --help`（`argv.rs:286-290`）。
  探测失败的真实原因是网络/代理不可达、npx 缓存缺失、`npx-cli.js` 未解析到，或子进程超时。
- **过度封锁**：`runtimeBlocked = runtimeError !== null`（`SkillsCliView.tsx:151`）
  会禁用卡片操作、批量栏、卸载对话框与详情抽屉的 link/unlink，
  但按 spec `skills-cli-global.md:81-83,99-100,184`，list / remove / link / unlink **都不 spawn CLI**。
  只有 `add`（Install）与来源 preview 会 spawn。

## Decisions

- **D1（Q1 = 方案 B）**：**完全移除 doctor 的 `skills --help` 探测**。
  `doctor()` 只保留 Node 存在性与版本检测；不再为了「预先证明包能跑」而付出一次子进程 + 一次网络往返。
  安装失败在实际 `add` 调用时暴露。
- **D2**：与 D1 配套，页面级 `cli_unavailable` 告警横幅取消，
  `runtimeError` 不再无差别驱动禁用。禁用范围收敛到确实会 spawn CLI 的入口。

## Confirmed Facts

- `doctor()` 现有两段：Node 版本检测（`mod.rs:310-332`）与 PIN 包探测（`mod.rs:334-349`）。
  D1 只删除后者。
- `SkillsCliError::CliUnavailable` 的另一个产生点是 launcher 解析失败
  （`argv.rs:239-246`，node 旁找不到 `npx-cli.js`）。该分支是真正无法 spawn，需保留，
  但只应在 spawn 路径上暴露，不作为页面级横幅。
- **诊断性风险**：spec `skills-cli-global.md:156` 当前把「CLI non-zero (preview/add)」
  映射到 `internal.unexpected`。移除探测后，安装失败将只显示通用内部错误，
  比现状更难理解。本任务必须一并修正该映射，否则是 UX 回归。
- `runtimeError` 与 `inventoryError` 已在 store 中分轨（归档 `08-25-skills-cli-inventory-frontend` R2），
  doctor 失败不清空库存。本任务不改这一分轨。
- stderr / 路径 / URL 不得进入 `IpcError.message` 或未脱敏操作日志
  （`redaction-policy`，spec `skills-cli-global.md:109-111`）。
- 需要一并更新的既有测试：`SkillsCliView.test.tsx:300-308,993-1000`、
  `SkillsCliHeader.test.tsx:41-92`、`skillsCliStore.test.ts:106-122`、
  `src-tauri` `tests.rs:351-385`（`ac10_doctor_probe_failure_warns_without_stderr_and_keeps_public_message`
  所断言的探测已不存在，需改写或迁移到 add 路径）。
- 归档 PRD `08-25-skills-cli-inventory-frontend/prd.md:54` 的 R5 明文要求
  「`cli_unavailable` 出现时安装/**卸载**按钮禁用」。本任务显式推翻该条并在 spec 记录变更理由。

## Requirements

- R1：`doctor()` 移除 `build_probe_argv` 调用与随后的 `CliUnavailable` 返回；
  保留 Node 存在性与 `>= 22.20` 版本检测。`SkillsCliDoctorReport` 的对外形状不变。
- R2：页面移除 `cli_unavailable` 常驻 `role="alert"` 横幅。
  `runtimeError` 不再无差别禁用；卸载、link、unlink、导出、详情在任何 doctor 结果下保持可用。
- R3：Install 入口的可用性只由 doctor 结果决定，采用 **fail-closed**：
  doctor 成功才启用，doctor 以任何码失败都禁用并显示该码的公开句。
  `runtimeError` 不是单一 `node_missing`，而是
  `node_missing` / `timeout` / `cancelled` / `internal.unexpected` 的集合（TPR-05，design §2.4.1）；
  瞬时码的重试复用 header 既有的 Refresh，不新增控件。
  doctor 路径不得产生 `cli_unavailable`——node 二进制存在但无法启动时重映射为 `node_missing`。
- R4：修正安装失败的错误映射。现状把 `CliFailed`（CLI 已执行但请求失败）与
  `OutputLimitExceeded` / `ListUnparsed` 一并折叠成 `internal.unexpected`（`error.rs:169-171`）。
  拆出 `CliFailed` 并给它独立的稳定错误码，措辞区别于 `cli_unavailable`（环境无法执行）。
  同步 spec `skills-cli-global.md` 错误矩阵。公开句仍不得含 stderr、路径或 URL。
- R5：Rust 侧在两条失败路径上各产生一条结构化 `tracing warn`——add 非零退出、子进程无法 spawn。
  当前 add 路径**没有**任何日志（`mod.rs:572-574`），因此这是新增而非「保留」（TPR-04）。
  字段限于 design §2.5 的白名单：退出码、stdout/stderr 字节长度、技能与平台计数、
  静态 source 分类、spawn 的 `io_kind`。
  「摘要」不得是 stderr 的节选——没有任何字段派生自子进程输出，
  这样才能同时满足「warn 存在」与「stderr 不泄露」。日志与 IPC message 均不含 stderr、路径、URL。
- R6：`.trellis/spec/backend/skills-cli-global.md` 同步：
  doctor 契约、错误矩阵 `cli_unavailable` 行的适用范围、§5 Base case 中「doctor…may spawn」的表述、
  §6 中与探测相关的测试要求。
- R7：受影响的既有测试全部改写为新契约，不允许删除断言了事；
  新增覆盖「Node 正常但 npx 不可用时，卸载/解链仍可执行且 Install 在实际调用时失败」的用例。
- R8：新增或变更文案 en/zh 成对。

## Acceptance Criteria

- [ ] AC1 (R1)：`doctor()` 的 fake runner 测试断言只发生一次子进程调用（`node --version`），
      不再出现 `skills --help`。
- [ ] AC2 (R1,R3)：Node 缺失 / 版本过旧仍分别返回 `skills_cli.node_missing`；
      Node 正常时 `doctor()` 成功，即使 npx 包不可执行。
- [ ] AC3 (R2)：页面中不存在渲染 `skills_cli.cli_unavailable` 公开句的常驻横幅节点。
- [ ] AC4 (R2)：mock doctor 成功但 `skills_cli_add_global` 失败的场景下，
      卡片卸载按钮、批量栏 Unlink / Uninstall、卸载对话框均可交互并能成功调用对应 IPC。
- [ ] AC5 (R3)：mock `skills_cli_doctor` 拒绝 `skills_cli.node_missing` 时，
      Install 入口禁用并显示 Node 要求说明；其余操作不受影响。
- [ ] AC5b (R3)：mock doctor 分别以 `skills_cli.timeout` 与 `internal.unexpected` 拒绝时，
      Install 同样禁用、状态行显示对应公开句、Refresh 仍可点击，且库存不被清空。
- [ ] AC5c (R3)：doctor 的 node 版本探测遇到 spawn 失败时返回 `skills_cli.node_missing`，
      **不返回** `skills_cli.cli_unavailable`。
- [ ] AC6 (R4)：`skills_cli_add_global` 失败时按性质返回不同的码——
      launcher 无法解析 / 无法 spawn → `skills_cli.cli_unavailable`；
      CLI 非零退出 → 新增的 `CliFailed` 专属码，不再是 `internal.unexpected`。
      两者在 spec 错误矩阵中各有一行，且公开句措辞可区分。
- [ ] AC7 (R5)：add 非零退出时，断言 warn **存在**且携带 `operation` 与 `exit_code`；
      同一用例植入 `SECRET_STDERR_TOKEN` 哨兵，断言其既不出现在 tracing 输出，
      也不出现在 `IpcError.message`（沿用 `tests.rs:351-385` 手法，迁移到 add 路径）。
- [ ] AC7b (R5)：spawn 失败时，断言 warn 存在且只携带 `phase` 与 `io_kind`，
      断言日志中不出现 `source` 的 Display 文本。
- [ ] AC8 (R6)：`skills-cli-global.md` 中不存在与实现矛盾的探测描述；
      错误矩阵与 `SkillsCliView` 实际禁用面一致。
- [ ] AC9 (R7)：`SkillsCliHeader.test.tsx` / `SkillsCliView.test.tsx` / `skillsCliStore.test.ts`
      中原「doctor cli_unavailable → 卸载禁用」断言被替换为新契约断言，无遗留矛盾用例。
- [ ] AC10 (R8)：i18n en/zh parity 检查通过。
- [ ] AC11 (Completion Gate)：`just ci` 通过。
      来源是 `AGENTS.md` 的 Completion Gate 一节，不隶属本任务的任一 R；
      全树统一用 `(Completion Gate)` 标注，避免被严格 R→AC 追踪判为悬空子句（TPR-09）。

## Out of Scope

- 修复 npx 探测失败的根因（网络、代理、npx 缓存）。本任务只改产品行为。
- 改变 `SKILLS_CLI_NPM_SPEC` PIN 版本或 argv 结构。
- 远端目标的 doctor 语义（属 `08-27-skills-cli-remote-target` 树）。
- `inventoryError` 的呈现与重试逻辑。
- 为安装失败提供更详细的 stderr 诊断（受 redaction-policy 约束）。

## Ordering

本任务是 `08-27-skills-cli-bulk-cleanup` 与 `08-27-skills-cli-remote-seam` 的前置：
两者都依赖本任务定型后的 doctor 语义与 `runtimeBlocked` 传播面。
建议先合入 `dev` 再启动其余子任务，避免同一工作树并行写 `SkillsCliView.tsx`。
