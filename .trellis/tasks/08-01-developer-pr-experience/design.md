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
- `dev` 不删除，并在启用 repository-wide `delete_branch_on_merge` 前建立保护。
- `main` 继续要求 PR、`just-ci`、对话解决、管理员受约束、禁止 force/delete。
- `dev` 至少禁止 force/delete，并按 task PR 模型要求 PR 与 `just-ci`；不增加审批人数。

仓库级同时启用 squash 与 merge commit 并关闭 rebase。`dev` ruleset 要求 PR、`just-ci`、禁止 force/delete 和 linear history；由于 rebase 已关闭，task PR 实际只可 squash。`main` 保留现有 required check/app binding、PR、对话解决和禁止 force/delete，但不能启用 linear history，否则 promotion merge commit 会被禁止。GitHub 不能把 `main` 自动限制为 merge-commit-only，因此 promotion 必须使用受控 `gh pr merge --merge --match-head-commit` 并在执行前检查 base/head。远端更新顺序为：读取快照 -> 建立/验证 `dev` ruleset -> 更新 repository merge settings/auto-delete -> 回读全部设置。任一步不一致即停止。

## 4. Sync Semantics

常规 promotion merge 后，旧 `dev` tip 已是 `main` 的祖先，不要求制造 `main -> dev` merge commit。只有 `main` 有独立 hotfix、promotion 冲突解决改变内容或下一批工作确需新基线时，才通过显式同步 PR/受控 fast-forward 更新 `dev`。

## 5. Documentation And Rollback

README、README_CN、CONTRIBUTING、AGENTS 和质量 spec 使用同一分支/命令/CI 触发器描述。远端设置变更前保留完整 JSON；回滚按相反顺序恢复 merge settings 和保护规则，绝不通过删除 `dev` 回滚。
