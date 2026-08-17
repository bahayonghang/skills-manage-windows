# Implement: SSH leftover cleanup batching

## Ordered checklist

1. 在 `apply_remove_deleted_platform_copies_step` 把 SSH/WSL 从逐条 `remove_deleted_platform_copy_remote` 换成：一次校验 → 唯一路径脚本 → 按路径收尾 DB。
2. 抽出纯函数：规范化路径、按路径分组、解析脚本 stdout。不要把协议解析写进命令层。
3. 远端脚本只接收已通过 `ensure_remote_child_path` + `remote_join(slot)` 的路径。argv 不够则 stdin NUL 列表。
4. 成功/`MISSING`：删 `skill_installations` 与匹配 observation。共享同一路径的平台一起清。
5. 把 apply `cancel` 传入 runner。若实现分块，取消后不再开下一块。
6. Local 分支不改行为。能删掉的只是远端函数里「每条 `connect_remote_target`」。
7. FakeRunner 测试（SSH 与 WSL 至少一侧完整，另一侧协议/命令计数对齐）：
   - 10 个共享根组、1 条路径 → 1 次 runner 调用。
   - 3 条路径混合 OK/MISSING/ERR → 部分成功。
   - 守卫失败 → 0 次 runner 调用。
   - 成功后扫描不再返回该路径。
8. 保留现有 Local leftover 测试：托管副本删除、越权 agent、Central 重新出现。
9. 跑定向 Rust 测试 + leftover 前端测试。实现收尾跑 `just ci`。

## Validation commands

```bash
cd src-tauri && cargo test inventory::tests --locked
cd src-tauri && cargo test services::central_updates --locked
pnpm test -- src/test/components/central/updateCenter/UpdateCenterDialog.leftover-cleanup.test.tsx src/test/components/central/updateCenter/updateCenterDecisionAggregation.test.ts
just ci
```

无可用 SSH 主机时，不把实机墙钟当作门禁。用 FakeRunner 调用次数证明 R2。

## Risky files

| 文件 | 风险 |
| --- | --- |
| `src-tauri/src/services/central_updates/inventory/apply_steps.rs` | 批量 `rm` 的唯一写入点。守卫漏了就会扩大删除面。 |
| `src-tauri/src/services/central_updates/inventory/scan.rs` | 仅在需要补 observation 删除后的扫描断言时改；不要顺手改扫描语义。 |
| `src-tauri/src/db/repos/installations_repo.rs` | 若新增按路径删 installation+observation，必须走事务。 |
| `src-tauri/src/targets/exec.rs` | 优先复用 `run_script` / `run_command`。不要为 leftover 再开一种 spawn。 |

## Rollback

还原 `apply_remove_deleted_platform_copies_step` 远端循环即可。无迁移、无 IPC 变更。

## Follow-up (not this task)

- leftover 卡片按唯一路径折叠，修正 309 计数。
- russh 持久会话。
- leftover-only apply 跳过无用的 `CentralFs` open。
- 扫描停止对远端路径使用 `std::fs::symlink_metadata`。
