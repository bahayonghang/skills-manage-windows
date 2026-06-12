# Implement：深度分析驱动的优化修复（父任务执行计划）

父任务自身不承载实现，本文档定义子任务的执行顺序、门禁与整合复核。

## 执行顺序与门禁

```
1. 06-11-eng-hygiene-quickfixes   → 门禁：just ci 绿 + eslint . 全仓 0 错误
2. 06-11-spawn-blocking-io        → 门禁：just ci 绿 + 必改 6 文件逐函数核对
   ── 以上两项完成后才允许进入 C 批次 ──
3. 06-11-thiserror-batch1-infra   → 门禁：just ci 绿 + 两域 grep 扫尾 0 命中 + 模板回写父 design.md
4. 06-11-thiserror-batch2-mid     → 门禁：just ci 绿 + 五域 grep 扫尾 0 命中
5. 06-11-thiserror-batch3-tail    → 门禁：just ci 绿 + 全局扫尾（String 错误仅存 commands 边界）
6. 06-11-claude-md-rewrite        → 门禁：与代码现状抽查一致 + 用户 review
```

每个子任务遵循标准 Trellis 流程：`task.py start` → 实现 → 检查 → spec 更新 → commit → 归档，然后才开始下一个。

## 关键回滚点

- 每个子任务独立 commit，revert 单批即可回滚。
- C1 是模式风险集中点：若 installation/scanner 改造中发现模板不可行（如错误上下文丢失、测试断言大面积脆裂），停止 C2/C3，回父任务重新设计，C1 单独回滚。

## 整合复核（全部子任务归档后）

复核日期：2026-06-12，全部 6 个子任务已归档。

- [x] 对照 `docs/reports/skills-manage-windows-deep-analysis-2026-06-11.md` 行动清单 #1–#8 逐项确认关闭（#8 圆角部分按裁定保持开放）：
  - #1 spawn_blocking：全仓唯一包装 `src-tauri/src/fs_util.rs`，抽查 projects/crud.rs 经 `run_blocking_fs_with` 包装 ✓
  - #2 CLAUDE.md 重写：旧漂移描述 grep 0 残留（40+ 命令/9 模块表/3 主题/db.rs/27 内置/我的源）✓
  - #3 thiserror：12 个 services 域全部有 error.rs；services 层 `Result<_, String>` 仅剩 tests 内 1 处（口径排除）✓
  - #4 ESLint：`.eslintrc.cjs` 已删除，根目录 `eslint .` 输出 "No issues found" ✓
  - #5 Sidebar：`usePlatformStore((s) => s.agents)` selector 模式 ✓
  - #6 data.json：仓库根目录已不存在 ✓
  - #7 set_setting：`let _ = db::set_setting` 裸忽略 grep 0 命中 ✓
  - #8 discoverDeprecationPreference.ts 已删除、`bg-black/20` 0 命中；圆角清扫按裁定保持开放 ✓
- [x] 全量门禁：`just ci` 绿（2026-06-12，Rust 699 passed + e2e 5 passed，"All checks passed"）。
- [x] 报告附录数字复核：生产 unwrap/expect 11 处（= 报告基线，未上升）、整 store 订阅 0（`use*Store()` 空参调用 0 命中）、`eslint .` 全仓 0 错误。
- [x] 父任务归档。
