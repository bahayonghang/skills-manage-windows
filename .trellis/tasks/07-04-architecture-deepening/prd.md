# 架构深化专项：9 个 deepening 候选的分析与优化

## Goal

消化 2026-07-04 `/improve-codebase-architecture` 架构评审产出的 9 个 deepening 候选：每个候选经过独立的深入分析（design）后实施优化，最终让 SkillPort 的关键 seam 各自收敛到一个 deep module。本父任务只承载需求源、任务地图、跨子任务约束与最终集成审查，不直接承载实现。

## 需求来源

- 评审方式：3 个只读走查代理（Rust 后端 / React 前端 / 测试面）+ 关键结论人工复核（passphrase 词表分叉、services/ 域清单、更新中心行数均已证实）。
- 评审报告存档：`research/architecture-review-20260704.html`（原件在系统临时目录，已拷贝存档）。
- 领域词汇与约束以 `CONTEXT.md` 为准；架构词汇用 module / interface / depth / seam / adapter / leverage / locality。

## 任务地图（9 个子任务）

| 子任务 | 候选 | 评审强度 |
| --- | --- | --- |
| `07-04-unify-redaction-policy` | 统一 Redaction policy，两个日志层共用一份敏感字段契约 | Strong |
| `07-04-central-updates-service-domain` | Update Center ≈4000 行业务从 commands 壳层落回 service 域 | Strong |
| `07-04-frontend-platform-module` | 前端 Platform management module（Universal Agents 分组 + 平台多选） | Strong |
| `07-04-typed-ipc-adapter` | 按命令名类型化的唯一 IPC adapter + fixture seam | Strong |
| `07-04-transport-seam` | Local/SSH/WSL transport seam，一份操作实现三个 adapter | Worth exploring |
| `07-04-path-policy-remote-half` | 补完 Path policy 的 remote 半边 | Worth exploring |
| `07-04-rust-test-support` | Rust test-support harness，收敛 24 份手抄测试 setup | Worth exploring |
| `07-04-skill-card-scenarios` | UnifiedSkillCard 显式场景 interface | Worth exploring |
| `07-04-unify-frontmatter-parsing` | 统一 SKILL.md frontmatter 解析（小改动） | Speculative |

## 执行顺序与依赖（写死的排序约束，其余建议）

1. **硬依赖：`central-updates-service-domain`（子 2）必须先于 `transport-seam`（子 5）**。两者触碰同一批命令文件（match active_target 分发的 14 个文件中多数属更新中心域）；先归位再收拢，否则 transport 收拢后代码又要整体搬家。
2. **强烈建议：`rust-test-support`（子 7）先于子 2、5、6 等 Rust 重构类任务**——重构前先有共享 harness，迁移中的行为验证成本大幅降低。
3. 建议整体顺序（先小后大、先基建后重构）：redaction（1）→ test-support（7）→ frontmatter（9）→ platform（3）→ ipc-adapter（4）→ path-remote（6）→ skill-card（8）→ update-center（2）→ transport（5）。
4. 除第 1 条硬依赖外，前端三项（3/4/8）与 Rust 各项可并行推进。

## 跨子任务约束（所有子任务共同遵守）

- `CONTEXT.md`「不要重复建议的方向」四条继续有效：不深化 SSH remote target lifecycle、不按文件大小机械拆分、不造 Operation Log DSL、不按文件数量拆 Settings store。
- 后端遵循 `.trellis/spec/backend/domain-error-enums.md`（一域一错误枚举、`#[error]` 文案逐字保留）与 `.trellis/spec/backend/spawn-blocking-io.md`（重 IO 走 `run_blocking_fs_with`）。
- 所有子任务目标是**收敛实现、不改变用户可见行为**（除非其 PRD 明确列出行为修正项）。
- 每个子任务按 Trellis 流程独立走完 plan → execute → finish；复杂子任务必须有 `design.md` + `implement.md` 才能 `task.py start`。

## Acceptance Criteria（父任务完成定义）

- [ ] 9 个子任务全部完成归档，或对放弃者记录了可复核的放弃理由（必要时按 `/improve-codebase-architecture` 的约定沉淀为「不要重复建议」条目）。
- [ ] `CONTEXT.md`「当前优先 deepening opportunities」清单按实际结果刷新：移除已落地项（含评审确认的 #5 central_skills 已拆分、#6 Settings store 证据不支持），登记新格局。
- [ ] 集成审查：抽查各新 seam 无跨域 reach-in、无第二套平行实现复活（redaction / path / frontmatter 各 grep 一次）。
- [ ] 全量门禁通过：`just ci`（Web + Rust 检查链）。

## Notes

- 本任务是父任务，正常情况下不应成为 `task.py start` 的实现目标；实现发生在子任务里。
- 评审对文档的两处勘误已分派：CLAUDE.md 的 InstallDialog 默认勾选描述修正归子任务 3；obsidian 域 0 测试的补齐演示归子任务 7。
