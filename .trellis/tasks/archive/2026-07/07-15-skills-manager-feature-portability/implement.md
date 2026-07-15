# 父任务实施计划

> 父任务没有直接产品代码。只启动拥有下一项交付的子任务。

## 1. Planning Gate

- [x] 用户明确排除 Git 备份、快照、恢复和多设备合并。
- [x] 创建 `07-15-stable-skill-identity-mutation-lock`。
- [x] 创建 `07-15-shared-core-cli`。
- [x] 用户审查父子规划并批准实施（2026-07-15）。

## 2. Child Execution Order

1. 启动并完成 `07-15-stable-skill-identity-mutation-lock`。
   - 验证 `uid` backfill、兼容 resolver、现有 GUI mutation lock 接入和跨进程竞争。
2. 启动并完成 `07-15-shared-core-cli`。
   - 复用上一步的 identity/lock contract，交付 Local CLI 查询、搜索、安装和同步。
3. 返回父任务做跨子任务集成检查。

## 3. Parent Integration Review

- [x] CLI 没有调用 `commands::*` 或复制 marketplace/import/installation 编排。
- [x] 所有 CLI Central mutation 都通过共享 service 追踪到 `CentralMutationGuard`。
- [x] CLI JSON 中 `uid` 稳定、`id`/slug 兼容，Central resolver 行为一致。
- [x] 现有 Tauri UI、collections、tags、repository assignment、portable state 与扫描结果无回归。
- [x] 仓库中没有新增 Git backup/snapshot/merge 功能或占位代码。

## 4. Full Verification

```powershell
rtk python ./.trellis/scripts/task.py validate 07-15-skills-manager-feature-portability
rtk python ./.trellis/scripts/task.py validate 07-15-stable-skill-identity-mutation-lock
rtk python ./.trellis/scripts/task.py validate 07-15-shared-core-cli
rtk just ci
rtk pnpm tauri build
```

CLI 离线 E2E 使用临时 HOME、fixture GitHub snapshot/本地 HTTP stub 与测试 Agent 目录，覆盖：list/show/search → install preview → install → sync dry-run → sync → GUI/service 重新查询。

## 5. Rollback

- CLI 可独立移除，不回滚 `uid` 或 mutation lock。
- `uid` schema 为 additive；若未对外承诺前需要回滚，可停止读取但不删除列。
- lock 不允许通过 feature flag 绕过；若锁实现有问题，应回滚所有新 mutation 入口而不是无锁运行。

## 6. Integration Evidence (2026-07-15)

- 两个子任务均通过 Trellis validate 并归档；父任务未承载产品代码。
- `rtk just ci`: 123 frontend files / 1346 tests; Rust clippy and 781 tests passed。
- Windows `rtk pnpm tauri build` 通过；Tauri 通过 `mainBinaryName=skillport` 打包桌面主程序，并在 `beforeBundleCommand` 生成 CLI。
- 最终产物：`skillport.exe`、`skillport-cli.exe`、`SkillPort_0.10.13_x64-setup.exe`。
- `git diff --check` 通过；`ref/skills-manager` 无改动。
