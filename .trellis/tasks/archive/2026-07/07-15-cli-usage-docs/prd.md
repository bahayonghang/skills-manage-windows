# 完善 SkillPort CLI 使用文档

## Goal

为 docs 站点新增中英文 SkillPort CLI 使用参考，覆盖安装、命令、JSON、退出码、安全行为与常见工作流，并接入导航。

## Background

- README/README_CN 只有 CLI 摘要，没有完整参数、退出码或脚本集成说明。
- `docs/reference/cli-just.md` 面向仓库开发者，只描述 `just` 配方，不是用户 CLI reference。
- docs 站点采用 English root + `/zh/` 双语结构，reference sidebar 分别由 `sidebar.en.ts` 与 `sidebar.zh.ts` 管理。

## Requirements

- 新增 `docs/reference/skillport-cli.md` 与 `docs/zh/reference/skillport-cli.md`，两页信息结构与命令覆盖保持一致。
- 说明从源码运行、release build 与 `cargo install` 三种入口，区分 `skillport-cli` 和桌面 `skillport`。
- 覆盖全局参数以及 `skills list/show/search/install/sync` 的位置参数、选项、输入格式和可执行示例。
- 记录 stable ref 解析、GitHub/skills.sh source、duplicate/replace/yes 安全规则、Agent 选择、安装 method 与 dry-run 行为。
- 记录 JSON envelope、stdout/stderr、exit code 0-5、Local-only、共享 SQLite/secret store/mutation lock 和 GUI 手动刷新限制。
- 将页面加入中英文 reference sidebar，并从 README/README_CN 的 CLI 摘要链接到完整文档。
- 运行 docs generator，保持已跟踪 data model 与 IPC reference 和当前源码一致。
- 文案只描述当前实现，不承诺 SSH/WSL CLI、PATH 自动配置或未实现的远程命令。

## Acceptance Criteria

- [x] 英文与中文 CLI reference 均可通过站内 sidebar 访问，且互相对应。
- [x] 五个 `skills` 子命令、全局参数和所有当前 flags 均有准确说明与示例。
- [x] 文档明确覆盖 source/ref 解析、覆盖确认、sync scope、JSON、退出码与并发锁语义。
- [x] README/README_CN 提供完整 CLI reference 链接，摘要不重复整页内容。
- [x] `pnpm docs:build` 通过，无新增死链或 VitePress 配置错误。
- [x] `git diff --check` 通过。

## Out Of Scope

- 改变 CLI 命令、help 文案、JSON schema 或业务行为。
- 自动安装 CLI 到 NSIS PATH。
- 为尚未支持的 SSH/WSL target 编写占位文档。
