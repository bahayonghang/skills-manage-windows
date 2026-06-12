# PRD：深度分析驱动的优化修复

> 任务类型：父任务（持有任务地图与跨子任务验收，自身不承载实现）
> 来源：`docs/reports/skills-manage-windows-deep-analysis-2026-06-11.md`（2026-06-11 深度分析报告）

## 目标与用户价值

依据深度分析报告的行动清单，系统性消除已确认的性能隐患、架构债务与工程配置问题，使代码现状与项目约定/文档重新对齐。

## 已确认事实（来自报告，均已逐项核查）

1. **同步 IO 隐患（中）**：17 个文件在 async fn 中直接调用同步 `std::fs` 且无 `spawn_blocking`，高密度项：`commands/central_updates_fs.rs`（17 处）、`services/github_import/import.rs`（15 处）、`commands/central_store_location.rs`（10 处）、`services/projects/crud.rs`（10 处）等；installation 域已有现成包装 `services/installation/fs_util.rs:14`。
2. **字符串化错误处理（中）**：后端 925 处 `Result<_, String>`，无 thiserror/anyhow；存在 `error.contains("timed out")` 脆弱判断（`commands/scanner.rs:100`）。
3. **CLAUDE.md 漂移（中）**：实际 171 个 IPC 命令/24 文件 vs 文档「40+」；后端已是 commands→services→db/repos 三层；6 套主题 vs 文档「3 种」。
4. **ESLint 配置（低）**：legacy `.eslintrc.cjs` 与 flat `eslint.config.cjs` 共存；flat config 全局 ignores 缺 `src-tauri/target` 等，根目录 `eslint .` 产生 738 个幻影错误（实测复现）。
5. **Sidebar 整 store 订阅（低）**：`src/components/layout/Sidebar.tsx:107`，全项目唯一违反 selector 模式处。
6. **data.json 位置（低）**：144.8 KB 测试夹具位于仓库根目录且被 git 跟踪（提交 82d8cd8 引入）。
7. **best-effort 静默失败（低）**：`let _ = db::set_setting(...)` 裸忽略，失败无日志（`commands/scanner.rs:49,66-67,92` 等）。
8. **其他打磨项（低）**：`src/lib/discoverDeprecationPreference.ts` 废弃残留；3 处 `bg-black/20` 遮罩未 token 化。

## 任务地图（已定稿，2026-06-11 用户认可）

| 顺序 | 子任务目录 | 内容 | 规模 |
|------|-----------|------|------|
| 1 | `06-11-eng-hygiene-quickfixes` | 工程卫生快修包（ESLint / Sidebar / data.json / set_setting / discover 残留 / 遮罩 token） | 小 |
| 2 | `06-11-spawn-blocking-io` | 重 IO 路径 spawn_blocking 改造 | 中 |
| 3 | `06-11-thiserror-batch1-infra` | thiserror 基建 + installation + scanner | 中 |
| 4 | `06-11-thiserror-batch2-mid` | thiserror 中批：central_skills / github_import / projects / marketplace / local_remote_sync | 中 |
| 5 | `06-11-thiserror-batch3-tail` | thiserror 尾批：usage / obsidian / ai_provider / ai_tagging / portable_state + db/repos 透传 + 全局扫尾 | 中 |
| 6 | `06-11-claude-md-rewrite` | CLAUDE.md 架构章节重写（记录改造后现状） | 小 |

依赖关系已写入各子任务 `prd.md`：串行执行，2 依赖 1 完成（建议）、3 依赖 2（必须）、4 依赖 3（必须）、5 依赖 4（必须）、6 依赖 2–5 全部完成（必须）。

## 验收标准（父任务级，跨子任务）

- 每个子任务交付后 `just ci` 全绿（typecheck / lint / 前端测试 / cargo test / clippy）。
- 行为不回归：现有 709 个 Rust 测试 + 1214 个前端用例全部通过（错误断言允许调整，用例不允许删除）。
- 分析报告条目 #1–#8（除圆角）逐条关闭，关闭证据可 grep / 命令验证。
- 最终整合复核：全部子任务归档后，对照报告「建议行动清单」逐项确认状态。

## 已决定

1. ✅ thiserror 改造：**全量改造全部 12 个 services 域**（用户选定选项 c，2026-06-11），拆为 C1/C2/C3 三批交付。
2. ✅ 圆角规格清扫：**不纳入**（用户选定选项 a，2026-06-11）。
3. ✅ 子任务结构与顺序：D→A→C1→C2→C3→B，用户认可（2026-06-11）。

## 范围外

- 圆角规格清扫（控件层 md/lg 离散度保持现状）。
- 「pages/ 目录按 feature 收敛」的组织性重构（单独立项）。
- 新功能开发。

## 待决问题

无——规划决策已全部关闭，等待用户 review 规划产物后启动子任务 1。
