# 文档部署与生成物完整性设计

## 1. Source And Generated Boundaries

- IPC 文档的权威来源是 Rust commands；schema 文档的权威来源是 `src-tauri/src/db/schema/`。
- `pnpm docs:gen` 是唯一写入入口，只更新两份 tracked Markdown。
- `pnpm docs:gen:check` 在内存中生成目标字节并与 tracked 文件比较；不写文件，漂移时列出路径和显式修复命令。
- 生成内容不得包含 wall-clock 日期。删除当前 `Last generated: <today>`，保证相同源码在不同时区和日期产生相同字节。
- `pnpm docs:build` 先运行 `docs:gen:check`，再运行 VitePress；构建本身不调用写入生成器。
- `pnpm docs:dev` 可以显式运行 `docs:gen` 后启动预览，因为这是开发者主动选择的写入流程。

## 2. Implementation Shape

两个生成器共享相同 CLI 合同：默认写入，`--check` 只比较。仅在确有重复时提取一个小型 byte-check helper；解析和渲染逻辑仍留在各自脚本中。生成器脚本测试覆盖稳定字节、漂移失败和 check 不写入。

## 3. Pages Workflow

```text
release published OR canonical main workflow_dispatch
  -> build (checkout/install/docs:build/configure-pages/upload-pages-artifact)
  -> deploy (github-pages environment/deploy-pages; no checkout or rebuild)
  -> smoke (retry public page URL; require HTTP 200 + SkillPort identity)
```

- build job 只产生一个 Pages artifact。
- deploy job 使用 `pages: write` 和 `id-token: write`，其他 job 保持最小权限。
- 使用 GitHub 官方 Pages Actions，并继续固定完整 SHA。
- smoke 使用 deploy action 输出的 `page_url`，限制为该仓库的 HTTPS Pages 地址，采用有界重试处理 CDN 传播；最终失败使 workflow 失败。
- production workflow 保留 `release.published`，并提供只允许从 `refs/heads/main` 启动的 `workflow_dispatch` 迁移/恢复入口；PR 的 common CI 运行生成物 check 和 `docs:build`，但不部署。
- Pages deployment concurrency 不取消已开始的生产部署；environment policy 只允许 canonical `main` 手动运行和已批准的 release tag。

## 4. Remote Setting And Rollback

代码合并且再次展示副作用后，将 Pages source 切换为 GitHub Actions 并回读。Legacy `gh-pages` 分支保持不存在。若部署失败，先保留 Actions artifact 和日志，再通过 workflow 或 Pages 设置回滚；不得自动重建分支发布链路。

## 5. Compatibility

- VitePress 的 `base`、输出目录和公开 URL 不改变。
- README、README_CN、CONTRIBUTING、AGENTS 和质量 spec 只描述真实触发器与命令。
- 不改文档信息架构、主题或产品文案。
