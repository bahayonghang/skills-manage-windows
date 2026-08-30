# 日志治理与集成实施计划

## Steps

1. 收集六个前置子任务的 PRD、实现 diff、测试结果与未验证项；前置 gate 未通过时停止集成声明。
2. 从当前 registry/policy 生成覆盖矩阵，检查未分类、重复 owner/action、排除理由、lifecycle 与测试映射。
3. 运行静态入口与隐私扫描；只修复跨子任务集成缝，将 domain-owned 缺陷退回对应子任务处理。
4. 移除已无调用的兼容 adapter，补充穷尽性、嵌套去重、跨层 correlation、interrupted 收口和对抗种子测试。
5. 更新 canonical specs 与架构页面；命令/schema 实际变化后运行 `pnpm docs:gen`，再以 read-only gate 检查生成漂移。
6. 先运行聚焦检查，再运行 `just ci` 与 `pnpm docs:build`；保存 passed/failed/skipped/unverified 证据。
7. 在 Windows 原生 Tauri 完成居中 Dialog、键盘/窄窗、关联导航、clear/retention 和受控终止 smoke；回填父任务
   AC traceability，不推送、不发布、不删除用户现有日志。

## Validation

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --locked logging
cargo test --manifest-path src-tauri/Cargo.toml --locked ipc_registry
pnpm exec vitest run src/test/contracts src/test/runtime src/test/stores
pnpm docs:gen
pnpm docs:gen:check
pnpm docs:build
just ci
git diff --check
```

原生检查记录应用版本/commit、Windows 环境、复现动作、期望/实际结果、截图或日志 correlation ID。不能执行的
外部/provider/原生条件必须标记 UNVERIFIED，不得以单元测试替代。

## Rollback

集成失败时保留新字段的向后兼容读取和 backend Runtime authority，恢复最后一组集成 seam；不回滚已验证的
domain coverage，不删除日志数据库或 Runtime 文件。兼容 adapter 只有在调用仍存在时恢复。
