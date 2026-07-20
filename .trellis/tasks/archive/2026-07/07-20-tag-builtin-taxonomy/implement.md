# Implement：内置 tag 分类集：升级迁移与 UI 可见策略

前置阅读：`.trellis/spec/backend/central-skill-tags.md`（本任务将重写它，先读旧契约）、
`.trellis/spec/quality/index.md`。

## 执行清单

### Step 1：seed 冲突迁移 + 内置 tag 集

- [x] `db/seed.rs`：实现 design.md §2 的单 tag 冲突处理辅助函数，替换现有
      无条件 upsert；扩充 `builtin_skill_tags()`（design.md §1 清单）。
- [x] `db/tests.rs`：
  - 新装：含全部新 tag（is_builtin=1）；重复 seed 幂等。
  - 升级 A（撞 id 的自定义 tag）：保留 custom、链接不变、未被劫持。
  - 升级 B（撞 name 的自定义 tag）：seed 不报错、custom 不变、内置未写入。
  - prune：不删新集合 tag、不删自定义 tag。
- 验证：`cd src-tauri && cargo test db::`

### Step 2：AI 候选放开

- [x] `services/ai_tagging/prompt.rs`：过滤改为仅排除 `UNCATEGORIZED_TAG_ID`。
- [x] `services/ai_tagging/tests.rs`：候选含内置 tag（抽查 2–3 个 id）、
      不含未分类。
- 验证：`cargo test ai_tagging`

### Step 3：UI 可见性

- [x] `src/lib/centralTags.ts`：新增 `getVisibleSkillTagsWithUsage`。
- [x] `CentralTopFilters.tsx` 切换；盘点其余 `getVisibleSkillTags` 调用点，
      管理/编辑类页面保持全量。
- [x] Vitest（`src/test/`）：空内置 tag 隐藏、有关联显示、自定义 tag 始终显示、
      特殊 filter id 与 sanitize 行为不变。
- 验证：`pnpm test -- src/test/centralTags.test.ts`（或相应文件）+ `pnpm typecheck`

### Step 4：spec 重写（必做）

- [x] 按 design.md §5 重写 `.trellis/spec/backend/central-skill-tags.md`
      （契约、矩阵、测试清单、Wrong/Correct 示例）。

### Step 5：门禁

- [x] `just ci` 全绿（fmt / clippy locked all-targets / cargo test /
      pnpm typecheck+lint+test / sizecheck / build，以 justfile 为准）。

## 回滚点

- Step 3 前端独立可回退。
- Step 1 发布后从内置集移除条目会经 prune 删除用户链接（parent D4）：
  **不作为常规回滚手段**；发布前回滚 = revert 提交。

## Review gate

- 完成后走 `trellis-check`；本任务归档是 child
  `07-20-ai-new-tag-review` 启动的前置条件。
