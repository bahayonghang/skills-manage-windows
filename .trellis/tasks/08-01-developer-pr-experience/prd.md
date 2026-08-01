# 开发与 PR 体验治理

## Goal

让新机器、日常修改、提交前验证和 PR 评审使用可复现、分层且语义清晰的入口，避免长期 promotion PR 承担首次集成验证。

## Requirements

1. 在仓库中固定 Node 22 LTS、pnpm 10.12.3 和明确 Rust toolchain，并提供跨平台 `just doctor` 输出缺失/版本不匹配。
2. 保留 `just ci` 完整门禁，新增可快速运行的静态/聚焦入口；文档不得暗示快速门禁可以替代提交前完整门禁。
3. 增加 PR template，覆盖用户问题、实现边界、风险、验证、UI 证据、打包/发布影响和回滚。
4. README、CONTRIBUTING、AGENTS 和 quality spec 对 CI 触发器、分支模型与合并要求保持一致。
5. 保留长期 `dev` 作为用户的日常开发分支，不删除本地或远端 `dev`；远端保护设置修改前后都必须读取并验证实际规则。
6. 短生命周期任务分支以 squash merge 进入 `dev`；`dev -> main` promotion PR 使用 merge commit 保留祖先关系，promotion 后 `dev` 可快进到 `main`，只有 `main` 存在独立 hotfix 时才需要显式同步。
7. 合并后自动删除短生命周期任务分支；`dev` ruleset 要求 linear history 并禁止 force/delete，与关闭 rebase 共同实现 task PR squash-only。仓库允许 squash merge 和 merge commit；promotion 使用受控精确-head merge 命令，`dev` 不会被自动删除。

## Acceptance Criteria

- [ ] 声明工具链与 CI 完全一致，`just doctor` 在当前主机给出明确结果且不修改环境。
- [ ] 快速与完整门禁都有合同测试/文档，`just ci` 继续通过。
- [ ] PR template 可直接支持功能、修复、UI、打包和发布变更，不要求无关字段伪造证据。
- [ ] workflow/docs/spec 明确任务分支 squash 到 `dev`、`dev -> main` promotion PR 使用 merge commit 的模型，不存在要求删除或退役 `dev` 的有效说明。
- [ ] 获得外部设置授权后，仓库允许 squash merge 和 merge commit、关闭 rebase merge 并自动删除已合并任务分支；`dev` ruleset 强制 task PR squash 且禁止 force/delete，实际设置回读与目标合同一致。

## Out of Scope

- 强制引入本地 Git hooks、merge queue 或多审批人流程。
- 自动安装/替换开发者系统 Node、pnpm、Rust 或 just。
- 删除本地或远端 `dev` 分支。
