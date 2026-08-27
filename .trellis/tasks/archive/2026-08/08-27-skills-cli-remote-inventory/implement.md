# 执行计划 — Skills CLI 远端只读列举

依据 `prd.md` 与 `design.md`。按段执行，每段结束跑该段验证命令再进入下一段。

**前置**：`08-27-skills-cli-remote-seam` 已合入 `dev`。

## 段 1 — 分类与观测分离（回滚单元 A，纯本机重构）

- [ ] 1.1 `placement.rs` 新增 `ObservedSlot` 枚举（design §2.2）：
      `Absent` / `ManagedLink { kind, resolves_to_canonical }` / `PlainDirectory` / `Conflict`。
- [ ] 1.2 新增 `classify_one_observed(canonical_owned: bool, slot: ObservedSlot, platform: &PlacementPlatform)`。
      把 `classify_one`（`:40-71`）的判定逻辑整体搬入，**不改任何判定顺序**。
- [ ] 1.3 `classify_absent`（`:73-110`）保持原样。四个 `reason_code` 的产生顺序
      （`canonical_missing` → `platform_unsupported` → `platform_not_detected` → `platform_disabled`）
      一个字不改——它是 `bulk-cleanup` 清理判据的依据。
- [ ] 1.4 `classify_one` 退化为薄封装：调 `observe_directory_slot` 得到 `ObservedSlot`，
      调 `canonical_is_owned_directory` 得到 `canonical_owned`，再转调 1.2。
- [ ] 1.5 `classify_placements`（`:27-38`）改为接受一个「已观测结果表」，
      本机调用方现场观测，远端调用方从段 2 的探测结果构造。

验证：`cargo test -p skillport skills_cli` — 既有 placement 用例应全绿且**无断言改动**。

## 段 2 — 远端探测脚本（回滚单元 B）

- [ ] 2.1 新建远端探测脚本构造器，路径清单以 **heredoc 内嵌进脚本体**，
      不走 argv（design §2.1）。输出格式 `路径\t类型\t链接目标`，
      类型取值 `link` / `dir` / `file` / `absent`。
      参照 `usage/fs_backend.rs:294-309` 的 marker 协议风格，但不复用其分块逻辑。
- [ ] 2.2 路径清单在 Rust 侧生成：全部技能的 canonical 目录
      + 「技能 × 平台」全部 slot 路径 + 各平台 `global_skills_dir` 本身（供 detected 判定）。
      顺序固定；输出按路径回填，缺行按 `absent` 处理。
- [ ] 2.3 解析器把每行转成 `ObservedSlot`。**分类判定不进 shell**：
      `remote_os()` 分支（design §2.3）在 Rust 侧完成。
      Windows 远端遇到 `dir` 信号时按 `PlainDirectory` 处理，不猜测 junction。
- [ ] 2.4 `SkillsCliFs` 增加 `probe_paths(&[String]) -> Result<Vec<PathProbe>, _>`。
      Local 实现逐路径走现有 syscall（保持本机行为不变）；
      Remote 实现走 2.1 的单次 `run_script`。

验证：`cargo test -p skillport skills_cli`

## 段 3 — 远端列举与错误映射（回滚单元 B 续）

- [ ] 3.1 `list_global` 改为两阶段（design §3）：
      RT1 读 lock（`read_file_bounded`），RT2 一次 `probe_paths`。
      **断言性注释**：这两次是全部远端往返，任何新增的 per-skill 远端调用都是回归。
- [ ] 3.2 lock 缺失 / 为空 → 返回空 `skills` 且携带 `canonicalRoot` / `lockPath`。
      `RemotePathMissing` 在此处吞掉转空，不上抛（R5）。
- [ ] 3.3 平台清单与 `enabled` 取 `TargetContext::db()`（已是远端 target 的 DB，
      见 `targets/registry.rs:420-427`）；`detected` 取 2.2 探测到的目录存在性。
      **不写任何「本机结果兜底」分支**（R4）。
- [ ] 3.4 新增 `SkillsCliError::RemoteUnavailable` 变体与
      `error.rs` 映射 `=> "skills_cli.remote_unavailable"`（design §2.5 的映射表）。
- [ ] 3.5 `ipc_error.rs` 增加 `skills_cli.remote_unavailable` 公开句；
      **不得含主机名、用户名、路径、stderr**。
- [ ] 3.6 i18n：`en.json` / `zh.json` 的 `backendErrors.skills_cli` 下成对新增
      `remote_unavailable`。
- [ ] 3.7 运行 `pnpm ipc:codegen` 刷新 `generatedCommandMap.ts` 的 reviewed codes；
      该文件是生成物，不手改。
- [ ] 3.8 能力矩阵翻闸：`ListGlobal` / `InstallTargets` / `ReadSkillMd` / `ExportInventory`
      改为远端支持。`RevealFolder` 保持永久不支持。

验证：`cargo test -p skillport && pnpm ipc:codegen && git diff --exit-code src/lib/ipc/`
（第三条应在重跑 codegen 后无 diff）

## 段 4 — 后端测试

- [ ] 4.1 AC1：fake 远端 runner 断言远端命令调用次数在「3 技能 × 4 平台」与
      「30 技能 × 4 平台」两种输入下**相同**（应为 2）。
- [ ] 4.2 AC2：断言列举路径上 argv 构造器未被调用（零 CLI spawn）。
- [ ] 4.3 AC3：远端分类覆盖五态各一例 + 四个 `reason_code`；
      同输入下与本机 `classify_one` 结果逐字段一致（用同一组 `ObservedSlot` 喂两边）。
- [ ] 4.4 AC4：`remote_os` 为 Unix 与 Windows 两种分支各一组用例；
      同时断言普通目录不被误判为 managed link。
      **真实远端 Windows 主机行为在结果中标注 `UNVERIFIED`**。
- [ ] 4.5 AC4b：构造「本机检测到平台 A、B，远端只有 A」的前置，
      断言平台 B 为 `platform_not_detected`；反向再构造一次（远端多于本机）。
- [ ] 4.6 AC4c：断言远端列举流程未读取本机 `resolve_home_dir()` 派生路径
      （用 fake 本机 FS 记录调用，或断言生成的路径清单全部以 `remote_home` 为前缀）。
- [ ] 4.7 AC5：远端 lock 缺失与 lock 为空两种情形都返回空 `skills` 且携带路径，无错误。
- [ ] 4.8 AC7b：连接失败映射 `skills_cli.remote_unavailable`；
      超时映射 `skills_cli.timeout`；两者可区分。
- [ ] 4.9 AC8：远端 stderr 植入哨兵 token，断言不出现在 `IpcError.message` 与操作日志。

验证：`cargo test -p skillport skills_cli`

## 段 5 — 前端解闸（回滚单元 C）

- [ ] 5.1 `src/components/layout/Sidebar.tsx:113-114`：移除非 Local 隐藏 `/skills-cli` 的条件。
- [ ] 5.2 `src/pages/SkillsCliView.tsx:210-217`：删除非 Local 的 `skillsCli.localOnly` 占位分支。
- [ ] 5.3 同文件 `:111-116`：移除「非 Local 不 `loadAll()`」的条件，改为无条件加载。
- [ ] 5.4 写操作按钮按能力矩阵禁用，tooltip 复用
      `skills_cli.local_target_only` 的既有公开句（design §2.6），不新增 i18n 键。
      清单随后续两个子任务合入而缩短。
- [ ] 5.5 确认切 target 时 `AppShell.tsx:91-127` 的 store 重置 + 重扫已覆盖远端场景，
      无需新增机制。

验证：`pnpm typecheck && pnpm lint`

## 段 6 — 前端测试

- [ ] 6.1 AC6：mock 非 Local target，断言侧边栏出现 `/skills-cli` 入口、
      页面渲染远端库存、未开闸的写操作显示本地化原因（而非静默 disabled）。
- [ ] 6.2 AC7：mock `skills_cli_list_global` 拒绝 `skills_cli.remote_unavailable`，
      断言展示可重试的库存错误且**已有列表不被清空**（stale-while-revalidate）。
- [ ] 6.3 更新既有断言「非 Local 渲染 localOnly」的用例为新契约，不得删除了事。

验证：`pnpm vitest run src/test/pages/SkillsCliView.test.tsx src/test/components`

## 段 7 — 收尾

- [ ] 7.1 AC9：i18n en/zh parity 检查通过（新增 `remote_unavailable` 一对）。
- [ ] 7.2 `.trellis/spec/backend/skills-cli-global.md` 能力矩阵翻四行，与代码同批。
- [ ] 7.3 确认 `generatedCommandMap.ts` 的改动全部来自 `pnpm ipc:codegen`。
- [ ] 7.4 全量：`just ci`。真实 SSH 主机端到端行为标记 `UNVERIFIED`。

## 风险文件与回滚点

回滚单元见 `design.md` §6。

| 文件 | 风险 | 回滚单元 |
| --- | --- | --- |
| `services/skills_cli/placement.rs` | 判定顺序若被无意改动，会连带影响 `bulk-cleanup` 的清理判据 | A |
| 远端探测脚本构造器（新） | 路径清单走 argv 会让往返退化为 `O(N/K)`，AC1 失败 | B |
| `services/skills_cli/error.rs` + `ipc_error.rs` + i18n | 新增码三处必须同批，漏一处会出现无公开句的裸码 | B |
| `pages/SkillsCliView.tsx` | 与 `08-27-skills-cli-bulk-cleanup` 冲突面，需错开工作树 | C |

## 前置检查

- [ ] `08-27-skills-cli-remote-seam` 已合入 `dev`，能力矩阵与 `SkillsCliFs` 可用。
- [ ] 确认 `08-27-skills-cli-bulk-cleanup` 未在同一工作树改 `SkillsCliView.tsx`。
- [ ] 工作树干净。
