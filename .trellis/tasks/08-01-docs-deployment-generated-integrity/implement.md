# 文档部署与生成物完整性实施计划

## Steps

1. 为两个生成器补充聚焦测试：相同输入稳定、`--check` 不写入、漂移时失败并指出文件。
2. 移除生成内容中的 wall-clock 日期，实现默认写入和 `--check` 只读模式。
3. 新增 `docs:gen:check`；把 `docs:build` 改为 check 后构建，保留 `docs:gen` 作为显式更新入口。
4. 显式运行一次 `pnpm docs:gen`，审查并提交当前两份生成文档的真实漂移。
5. 将 `.github/workflows/docs.yml` 改为官方 Pages artifact 单次构建/部署/smoke DAG，并补充 YAML 合同测试。
6. 更新中英文说明、CONTRIBUTING、AGENTS 和 `.trellis/spec/quality/ci-quality-gate.md` 中的文档合同。
7. 删除 legacy `gh-pages` 分支，并从 GitHub Branch API、远端 refs 和本地跟踪引用验证其不存在，同时确认 `dev` / `main` 未变化。
8. 在本地变更合并后，单独展示并确认 Pages source API 更新；执行更新、回读、触发部署并验证公开 URL。

## Focused Validation

```powershell
pnpm docs:gen
pnpm docs:gen:check
pnpm docs:build
pnpm vitest run src/test/contracts/ciWorkflowContract.test.ts
git diff --exit-code -- docs/architecture/_generated
just ci
just audit
```

增加文档 workflow 合同测试后，用其实际路径替换或补充上述聚焦测试。执行线上 smoke 时记录 URL、HTTP 状态、页面身份和对应 workflow run；失败不得只凭 deploy job success 结案。

## Risk And Rollback Points

- 在生成器逻辑稳定前不批量刷新其他文档。
- Pages source 更新前保存原设置；回滚只恢复设置和 workflow，不重建 legacy `gh-pages` 分支。
- 任何 `docs:build` 后 tracked diff 都是失败，不能由清理命令掩盖。
