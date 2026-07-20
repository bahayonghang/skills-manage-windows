# Implement：AI 新标签 proposal/review 流程

**前置**：child `07-20-tag-builtin-taxonomy` 已归档（候选规则定稿）。
前置阅读：`.trellis/spec/backend/central-skill-tags.md`（child A 更新后的版本）、
`.trellis/spec/backend/domain-error-enums.md`、`.trellis/spec/backend/spawn-blocking-io.md`（不涉及重 IO，确认即可）。

## 执行清单

### Step 1：schema 与 repo 读路径

- [ ] `db/schema/metadata.rs`：ensure_column 加 `proposed_name` /
      `proposed_description`。
- [ ] `db/repos/tags_repo.rs`：`get_pending_ai_tag_reviews` 改 LEFT JOIN +
      COALESCE + `is_proposal`；`replace_pending_ai_tag_reviews` 对 proposal
      行放开存在性校验；`db/types.rs` `SkillAiTagReview` 加 `is_proposal`。
- [ ] DB 单测：旧库升级加列；proposal 行写入/读取；孤儿行过滤。
- 验证：`cd src-tauri && cargo test db::`

### Step 2：create_skill_tag 原子化

- [ ] `tags_repo.rs`：ON CONFLICT(name) DO NOTHING + 按 name 回读；撞 id
      回退 UUID。
- [ ] 单测：并发/重复同名创建幂等；异名撞 id 回退。
- 验证：`cargo test db::`

### Step 3：接受/跳过语义

- [ ] `accept_ai_tag_reviews`：proposal 行接受时先创建 tag（design §4），链接
      挂实际 id；`skip` 保持无残留。
- [ ] 单测：接受创建 + 链接；同名重复接受复用；跳过零残留。
- 验证：`cargo test db::`

### Step 4：解析、归一化与流水线

- [ ] `types.rs`：`RawAiNewTagSuggestion` + envelope `new_tag` 字段 +
      `SkillTagProposal`；`SkillTagSuggestionResult`/progress payload 加可选
      `proposals`。
- [ ] `prompt.rs`：重写提示词（design §5）；`parse_ai_tag_suggestions` 返回
      envelope（兼容旧格式）；新增 `resolve_proposal` 纯函数（归一化、超长
      丢弃、撞名降级）。
- [ ] `mod.rs`：proposal 只进 pending review，永不自动应用；计入
      `low_confidence_count`。
- [ ] 单测：五种响应解析（复用/提议/混合/空/旧格式）；撞名降级；proposal
      不自动应用。
- 验证：`cargo test ai_tagging`

### Step 5：前端最小适配

- [ ] TS 类型 `isProposal`；review 面板 proposal badge「AI 新建标签」（i18n
      中英）。
- [ ] Vitest：proposal 展示与接受/跳过交互。
- 验证：`pnpm typecheck && pnpm test`

### Step 6：spec 同步（必做）

- [ ] `.trellis/spec/backend/central-skill-tags.md`：新增 proposal/review 契约
      与矩阵行（创建时机、跳过语义、归并、原子创建）。

### Step 7：门禁

- [ ] `just ci` 全绿。

## 回滚点

- Step 1 的 ensure_column 为增量可空列，回滚安全（旧代码忽略新列）。
- Step 4 解析层向后兼容，可单独回退提示词文案。
- 未接受的 proposal 仅存在于 review 表，回滚无 tag 数据残留。

## Review gate

- 完成后走 `trellis-check`；归档后 parent 做集成收口。
