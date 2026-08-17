# 实施计划：Central 删除强制放弃陈旧 prepared journal

前置：阅读 `prd.md`、`design.md`、`.trellis/spec/backend/fs-db-operation-journal.md`、`skill-deletion-integrity.md`、`domain-error-enums.md`、`.trellis/spec/frontend/async-error-feedback.md`、`ipc-adapter.md`。

## 步骤

1. **资格判定**  
   在 `src-tauri/src/services/central_operation/` 增加 force-abandon 预览：复用 reconcile 的路径去重与 backup/marker 检查，跳过 fingerprint / owned-missing。覆盖 yao-meta 形态夹具（重复 `.agents` 路径、平台不存在、Central 存在）。

2. **删除编排**  
   `BatchDeleteCentralSkillRequest.force` + 单删 `force`。`delete_central_skills_under_guard` 在 recover 之前按技能放弃 eligible 的 `prepared` 行，再走现有 stage/commit/finalize。SSH/WSL 走同一编排，只复用存在性检查。

3. **预览字段**  
   `preview_delete_central_skill_*` 填入 `pending_recovery`。单删、批量、仓库预览共用。

4. **IPC / 文档**  
   删除命令用稳定 code 映射 `IpcError`，登记 `central_operation.delete_restore_collision` 与 `central_skills.force_delete_blocked`。更新 `commandMap` / 生成类型。运行 `pnpm docs:gen`。

5. **前端对话框**  
   单删改走 `preview_delete_central_skills`。`DeleteCentralSkillDialog`、`BatchDeleteCentralSkillsDialog`、仓库删除加入强制删除入口与 `formatBackendError`。store 传 `force`。

6. **i18n**  
   en/zh：`central.forceDelete*` + `backendErrors.central_skills.force_delete_blocked`。已有 `delete_restore_collision` 文案保持原句。

7. **测试**  
   - Rust：普通删除仍报 `delete_restore_collision`；force + 无 artifact + 指纹漂移成功；有 backup 拒绝且文件不变；phase ≠ prepared 拒绝；放弃后无 pending，新删除 `completed`。  
   - 前端：预览显示恢复文案；强制按钮仅 eligible 时出现；确认调用 `force: true`；错误不含路径 / token。  
   - IPC：单删碰撞不再变成 `internal.unexpected`。

## 验证命令（迭代期）

```bash
cd src-tauri && cargo test central_skills:: --locked -- --test-threads=1
cd src-tauri && cargo test central_operation:: --locked -- --test-threads=1
pnpm test -- --run src/test/stores/centralSkillsStore.test.ts src/test/components/central/
pnpm typecheck && pnpm lint
```

按需缩小到新增测试文件。

## 完成门

```bash
pnpm docs:gen
just ci
```

## 风险文件

- `src-tauri/src/services/central_skills/delete/batch.rs`：recover 顺序，避免 force 之后仍去 restore 旧行。
- `src-tauri/src/commands/skills.rs`：勿再 `to_string()` 丢掉稳定 code。
- `src/pages/centralSkillsDeleteWorkflow.ts`：单删预览从 `get_skill_detail` 迁走时保持 copy / symlink 分组。

## 回滚

单逻辑提交可 `git revert`。无数据库迁移。已放弃的 journal 保持 `rolled_back`。
