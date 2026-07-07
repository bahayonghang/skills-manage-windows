# 统一 SKILL.md frontmatter 解析

## Goal

把 SKILL.md frontmatter 解析收敛为一份实现，Discover（scanner）与 Marketplace import（github_import）共用，闭合已分叉的 BOM 边界行为。

## 背景与证据（2026-07-04 架构评审）

同一核心解析存在两三份 implementation，边界行为已分叉：

- `services/scanner/mod.rs:94` `parse_skill_md_content` — 剥 `---` 栅栏 + `serde_norway::from_str`，返回 `SkillInfo`，**不处理 BOM**。
- `services/github_import/source.rs:662` `parse_frontmatter` — 同样剥栅栏，但**去 BOM**（`\u{feff}`）、typed（`SkillFrontmatter`）、逐行找闭合栅栏。
- `services/scanner/ssh_batch.rs` — remote 变体（shell 侧实现，受限）。

后果：带 BOM 的 SKILL.md 走 Discover 与走 Marketplace import 解析结果不同（locality 失败 + 用户可见的不一致风险）。

## Requirements

1. 一份 frontmatter 解析 module（归属位置——scanner 域内共享还是独立小模块——由 design 或实现时裁决，改动小可以边做边定）。
2. scanner 与 github_import 两个入口迁移为调用方；BOM 处理统一为「去 BOM」（以更健壮的 github_import 行为为准）。
3. 补 BOM 用例测试：同一份带 BOM 的 SKILL.md 在两条入口解析结果一致。
4. `ssh_batch.rs` 的 remote shell 侧变体尽量共享常量/语义；若 shell 侧结构性无法复用，在代码注释与本任务记录边界即可。

## Constraints

- 解析产出的字段语义（name/description 等）零变化。
- 扫描是热路径：不引入可感知的性能回退（scanner 现有 45 条测试全绿即视为达标，无需专门基准）。

## Acceptance Criteria

- [x] scanner 与 github_import 的 frontmatter 剥离/解析指向同一实现（grep 验证仓库内栅栏剥离逻辑只此一处，ssh_batch shell 侧例外需注明）。
- [x] 新增 BOM 测试证明两条入口行为一致。
- [x] `cd src-tauri && cargo test` 全过（scanner 45 条 + github_import 61 条为重点回归面）；`cargo clippy -- -D warnings` 通过。

## 实施记录（2026-07-04）

- 归属裁决：新建 `services/scanner/frontmatter.rs`（scanner 域是 SKILL.md 解析的天然 owner；github_import 跨域 use 已有 resource_budget 先例）。`extract_frontmatter_block(content) -> Option<&str>` 只负责栅栏剥离，两个调用方各自保留 YAML→字段映射，字段语义零变化。
- 统一语义采 github_import 版（更严谨）：去 BOM + 容前导空白 + 开/闭栅栏 trim 后须恰为 `---`（scanner 原 `find("\n---")` 宽松闭合会被行内 `---` 误伤，一并闭合）。
- 需求 4（ssh_batch）实测为免费：shell 侧只 `cat` 内容，解析本就走 Rust 侧 `parse_skill_md_content`，统一后自动继承；`frontmatter.rs` 模块注释已注明。
- 测试：frontmatter.rs 单测 6 条（BOM/前导空白/CRLF/EOF 闭栅栏/行内 `---` 不闭合/缺失或带缀栅栏拒绝）+ scanner BOM 入口 1 条 + github_import 双入口一致性 1 条。scanner 45→52、github_import 61→62，全量 739 passed + 2 ignored，clippy(lib) 干净；`--all-targets` 的 14 个报错均为 usage/secrets 等存量，与本任务无关。
- Spec 契约登记：`.trellis/spec/backend/skill-frontmatter-parsing.md`（禁手抄栅栏剥离 + 巡检命令）。

## Notes

- 复杂度：lightweight~medium，**PRD-only 可开工**（若实现中发现归属争议再补简短 design.md）。
- 呼应 CONTEXT.md 优先方向 #4 的残余——scan core 本体（`scanner/mod.rs` 745 行）已存在，勿重建。
- 适合作为热身任务或穿插在大任务之间完成。
