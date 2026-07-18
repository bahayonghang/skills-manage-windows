# Implementation Plan: GitHub import manifest fast path

## 1. Research gate

1. 在任务 `research/` 写 archive baseline、fixture 形状和预期指标。
2. 记录当前 preview 与 import 各下载一次 archive 的确定性证据。
3. 先确定 fast-path/fallback policy 和阈值，再改生产路径。

## 2. 测试优先顺序

1. 扩展 fake/mocked GitHub HTTP，能记录 surface、URL、请求数、bytes、PAT header 和响应序列。
2. 写 TreeRaw vs Archive candidate/preview parity tests，覆盖 root/nested/plugin/invalid candidate，以及真实 tar symlink 对应 mode `120000`、submodule absence 对应 mode `160000`；断言二者不进入 candidate/files/raw download。
3. 写 selection union、dedupe、budget、bounded concurrency 和“mutation 前失败”测试。
4. 写 fallback matrix：truncated、bad JSON、missing size、401/403/429、404、5xx、mirror、raw partial failure。
5. 如 cache 被数据要求，再写 TTL/LRU/PAT/target invalidation tests。

## 3. 实现顺序

1. 增加 manifest types 和 tree response 的有界 parser，只把 `100644` / `100755` blob 转为 `RegularBlob`，跳过 `120000` / `160000`，未知 mode/type typed fallback。
2. 提取共享 path/plugin/frontmatter discovery，使 snapshot 与 manifest 输入复用同一规则。
3. 接入 preview dispatcher，archive 保持默认可靠回退。
4. 实现 selected subtree planner 与有界 raw bytes downloader。
5. 将 prepared bytes 接入现有 staging/atomic import，不改变外部 command/DTO。
6. 添加 acquisition diagnostics，并写 before/after research 结果。
7. 仅在阈值仍未达标且数据指向重复 tree 请求时实现 metadata cache。
8. 更新 `.trellis/spec/backend/github-import-preview-contract.md` 或拆分 acquisition spec，并同步 index。

## 4. 定向验证

```powershell
cd src-tauri; cargo test services::github_import
cd src-tauri; cargo test github_import -- --nocapture
cd src-tauri; cargo clippy -- -D warnings
pnpm vitest run src/test/GitHubRepoImportWizard.test.tsx src/test/CentralSkillsView.github-import-preview.test.tsx
pnpm typecheck
pnpm lint
git diff --check
just ci
```

## 5. 风险文件

- `src-tauri/src/services/github_import/archive.rs`
- `source.rs`
- `plugin_manifest.rs`
- `preview.rs`
- `import.rs`
- `raw_http.rs`
- `types.rs`
- `tests.rs`
- `.trellis/spec/backend/github-import-preview-contract.md`

## 6. Commit / rollback shape

- Commit 1：mock baseline + parity/fallback tests。
- Commit 2：tree preview fast path。
- Commit 3：selected subtree import + diagnostics。
- Commit 4（条件）：metadata cache。
- 任一阶段可将 dispatcher 恢复为 ArchiveOnly；不得删除 archive tests 或把 fallback 变成未覆盖分支。
