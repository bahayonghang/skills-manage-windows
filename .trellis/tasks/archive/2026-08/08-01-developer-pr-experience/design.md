# 开发与 PR 体验治理设计

## 1. Toolchain Contract

- Node 声明为 22 LTS major，pnpm 精确为 10.12.3，并在 `package.json`、版本文件和 Actions 中保持一致。
- Rust 使用 `rust-toolchain.toml` 固定经 CI 验证的 1.97.0，包含 `rustfmt`、`clippy`；额外 target 仍由对应 workflow job 声明。
- `just doctor` 只读检查 Node major、pnpm 精确版本、rustc/cargo toolchain、just、Git 和 Windows Tauri 必要工具；逐项输出 `ok`/`mismatch`/`missing` 并以非零状态报告不可构建环境。
- doctor 不安装、不切换、不修改 PATH，不打印 token 或 secret。

开发命令分层：

| Command | Intended use |
| --- | --- |
| `just doctor` | 新机器/环境漂移诊断 |
| `just check` | 开发中快速静态与生成物反馈 |
| focused test command | 单模块行为验证 |
| `just ci` + `just audit` | PR 前完整本地门禁 |
| `pnpm tauri build` | Windows 打包/发布变更 |

## 2. PR Contract

`.github/pull_request_template.md` 使用可删除的相关项，而不是强制伪造证据，覆盖：用户问题、范围/非范围、风险、验证、UI 证据、打包/发布影响和回滚。模板明确 task PR 与 promotion PR 的区别。

## 3. Branch And Merge Settings

- task branch -> `dev`: squash merge；合并后自动删除 task branch。
- `dev -> main`: merge commit；关闭 rebase merge，保留祖先关系。
- `dev` 不删除，并在启用 repository-wide `delete_branch_on_merge` 前建立两个独立 ruleset。
- `main` 继续要求 PR、`just-ci`、对话解决、管理员受约束、禁止 force/delete。
- `dev-safety` 不配置 bypass actor，只禁止 deletion 与 non-fast-forward；`dev-flow` 要求 PR、app `15368` 绑定的 `just-ci`、strict update 与 linear history，不增加审批人数。

仓库级同时启用 squash 与 merge commit 并关闭 rebase。`dev-flow` 的 linear history 使常规 task PR 实际只可 squash；该 ruleset 为当前已认证维护者配置 `always` bypass，只允许 runbook 驱动的 archive/journal/final-evidence bookkeeping 和 promotion 后 exact fast-forward。bypass 不出现在 `dev-safety`，因此同一维护者仍不能 force push 或删除 `dev`。`main` 保留现有 required check/app binding、PR、对话解决和禁止 force/delete，但不能启用 linear history，否则 promotion merge commit 会被禁止。GitHub 不能把 `main` 自动限制为 merge-commit-only，因此 promotion 必须使用受控 `gh pr merge --merge --match-head-commit` 并在执行前检查 base/head。

远端更新顺序为：读取 repo/protection/ruleset/actor 快照 -> 建立并验证不可 bypass 的 `dev-safety` -> 建立并验证带最小维护者 bypass 的 `dev-flow` -> 更新 repository merge settings/auto-delete -> 回读全部设置。执行前必须展示实际 actor 类型/ID、两个 ruleset payload、repository PATCH、精确回滚 API 和副作用；任一步不一致即停止。

## 4. Sync Semantics

常规 promotion merge 后，旧 `dev` tip 是新 `main` merge commit 的父提交。为同时满足 `main` strict required checks、下一次 PR 的 `CLEAN` 门禁和 `dev` linear history，必须先刷新并证明 promotion head 是 `main` 的祖先，再通过维护者 bypass 将 `dev` fast-forward 到该 merge SHA；禁止 force。完成 fast-forward 后才能写最终证据或开始下一 task。若 `main` 存在独立 hotfix，则在新 task 前走单独同步审查，不把 hotfix 静默混入 task squash。

## 5. Documentation And Rollback

README、README_CN、CONTRIBUTING、AGENTS 和质量 spec 使用同一分支/命令/CI 触发器描述。远端设置变更前保留完整 JSON 和新 ruleset ID；回滚先恢复 repository merge settings，再删除/禁用 `dev-flow`，最后删除/禁用 `dev-safety`，并回读 `main` 未变化。绝不通过 force push 或删除 `dev` 回滚。
