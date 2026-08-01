# 文档部署与生成物完整性

## Goal

让文档源码、已跟踪生成文档、VitePress 构建产物和公开 Pages 站点形成一条可验证且不重复构建的交付链。

## Requirements

1. 为 IPC 字典和数据库 schema 文档提供确定性的生成与只读 `check` 模式；check 发现 byte drift 时失败，不改写工作树。
2. `pnpm docs:build` 在干净 checkout 上不得留下 tracked file diff；需要更新生成文档时使用显式生成命令并共同提交。
3. PR CI 必须运行生成物 check 和 `pnpm docs:build`，耗时较短的文档门禁不得延迟到 release 后才发现。
4. Docs workflow 构建一次、上传一次并部署同一 artifact，不在 deploy job 重新 checkout/install/build。
5. 优先使用 GitHub 官方 Pages Actions、最小权限和 `github-pages` environment；所有外部 Action 继续固定完整 SHA。
6. 部署后请求配置的项目 Pages URL，验证 HTTP 200 和 SkillPort 页面身份。
7. 更新中英文开发说明和 workflow contract 测试，删除与当前触发器不一致的文案。
8. 实施阶段将仓库 Pages source 切换为 GitHub Actions，部署后回读设置；legacy `gh-pages` 分支必须从远端和本地跟踪引用中删除。
9. 生产文档继续由 `release.published` 触发；额外提供仅允许 canonical `main` workflow source 的手动迁移/恢复入口，用于首次 Pages source 切换和线上 smoke，不要求创建公开 release。

## Acceptance Criteria

- [x] 当前生成物 drift 被显式生成并审查，随后 `pnpm docs:gen:check` 在干净树通过。
- [x] `pnpm docs:build` 通过且不产生 tracked diff。
- [x] Docs workflow contract 证明 build artifact 被 deploy job 复用、权限最小且包含部署后 smoke。
- [x] `release.published` 和 canonical `main` 手动迁移/恢复入口共享同一 build/deploy/smoke 路径；其他分支手动触发在部署前失败。
- [x] `just ci` 通过。
- [x] Legacy `gh-pages` 分支已从 GitHub、远端 refs 和本地跟踪引用中删除，`dev` / `main` 保持不变。
- [ ] Pages source 已切换为 GitHub Actions 且设置回读一致；线上项目 Pages URL 返回 HTTP 200 且标题/标识属于 SkillPort。

## Out of Scope

- 重写产品文档内容、导航信息架构或视觉主题。
- 重新创建 legacy `gh-pages` 分支或恢复分支发布模式。
