# Design: GitHub import manifest fast path

## 1. 原则

把 acquisition 与 domain/persistence 分开。tree/raw 和 archive 都必须产出能够进入同一候选、preview、selection validation 和 atomic import 管线的数据，不允许出现两套业务语义。

## 2. 新数据结构

建议在 `services/github_import/` 内引入内部类型：

```text
RepositoryFileMeta { repo_path, byte_len, kind: RegularBlob }
RepositoryManifest { repo, files, acquisition_context }
PreparedSelectedFiles { manifest, bytes_by_repo_path }
AcquisitionMode = TreeRaw | Archive
FallbackReason = Truncated | Unsupported | Denied | Transport | Budget | Integrity | Threshold
```

这些类型不序列化到前端。Tree parser 先按 Git mode/type 分类：`100644` / `100755` blob 转成 `RegularBlob`；`120000` symlink blob 与 `160000` commit/gitlink 被确定性跳过并计入内部 diagnostics；未知组合返回可回退 acquisition error。`GitHubRepoSnapshot` 继续作为 archive fallback 和现有写入路径的兼容表示。

## 3. Preview 流程

1. `resolve_repo_source` 得到规范化 repo/branch/sourcePath。
2. `try_fetch_tree_manifest` 通过 `send_github_request_with_fallback` 获取 tree JSON，执行响应体和条目预算。
3. 以 manifest paths 调用现有 discovery pure functions。
4. 读取 plugin manifests，再读取候选 `SKILL.md`，使用现有 frontmatter/invalid-candidate 规则构建 `RemoteSkillCandidate`。
5. `build_preview_skills` 查询 Central 冲突；把 manifest path/size 转换成 `PreviewRepositoryFile`，继续调用 `attach_preview_file_manifests`。
6. 若 acquisition 返回可回退错误，调用现有 archive preview；domain invalid/error 只有在 archive 也应失败时才直接返回。

tree response 本身也受 archive_files / expanded_bytes 类似上限；JSON body 需有单独合理上限，避免“为了不下 archive 而无界读 tree JSON”。

## 4. Import 流程

1. 重新 resolve，并优先取得 tree manifest（可命中有界 cache）。
2. 复用候选发现并验证 frontend selections；不要仅凭 sourcePath 下载任意路径。
3. 计算选中 sourcePath 的文件并集。若含根 skill，集合自然是全仓库。
4. 依据基线决定 `TreeRaw` 或 `Archive`：考虑文件数、总字节、预计请求数、root scope 和 mirror/PAT 能力。
5. TreeRaw 使用 `buffer_unordered`/Semaphore 有界并发下载 bytes，逐项验证 content length、实际长度、单文件和总预算；全部成功后组装只含选中路径的 snapshot/prepare object。
6. 调用现有 `plan_import_staging`、source mapping、progress、atomic swap 与 DB persistence。

任何 Central write 之前都要确定 acquisition mode 并准备完整输入。

## 5. Parity 策略

- 路径发现：共享 `discover_skill_manifests_from_paths_with_plugin_discovery`。
- plugin manifest：把“读取 bytes”与“解释 manifest”拆开，共用解释函数。
- frontmatter：共享 scanner parser 与现有 invalid candidate 分类。
- 文件边界：共享 `repo_file_relative_to_source`。
- preview：共享 `build_preview_skills` 与 `attach_preview_file_manifests`。
- import：共享 staging/atomic persistence。

测试对相同 fixture 分别走 TreeRaw 和 Archive，并断言 candidate/preview/selected files/import result 等价。Parity fixture 必须构造真实 tar symlink entry 与 tree mode `120000`、tar directory/gitlink absence 与 tree mode `160000`，证明两条 acquisition 都排除这些条目；不能只用 regular-file helper 模拟。

## 6. HTTP、PAT 与镜像

- Tree API 使用 `GitHubFetchSurface::Api`；raw bytes 使用 `GitHubFetchSurface::Raw`。
- token 只发送给直接 GitHub endpoint，继续遵守现有镜像规则。
- private repo 的 tree/raw 若镜像不可用，应使用 PAT 直接路径；401/403 不应被错误地降级成匿名镜像泄露尝试。
- 已知 symlink/gitlink 是可安全忽略的结构条目，不因它们单独 fallback；未知 mode/type 才 fallback archive，避免快路径与 tar regular-file 语义漂移。
- 404/5xx/rate-limit 是否 fallback 由现有 denial 分类和 acquisition policy 决定，最终错误保留最有信息量的原因。

## 7. 性能门槛

在 research 中先记录基线，再定阈值。推荐初始决策模型不是常量 300，而是：

```text
tree_raw_cost = request_overhead * file_count + selected_bytes
archive_cost  = archive_bytes + extraction_cost
```

CI 断言确定性指标：受支持嵌套 fixture 不请求 archive、传输文件集合等于 selection 并集、并发不超过上限。Wall time 只设宽松防退化预算，避免 flaky 精确毫秒阈值。

## 8. Cache 决策

若需要 cache，使用 process-local Mutex/LRU，最多 4 个 manifest、TTL 10 分钟；value 不含 token/bytes。PAT set/clear、target change 和 app shutdown 清空。若没有测量收益，保持无 cache 是正确结果。

## 9. 回滚

保留单一 `ArchiveOnly` policy 开关或可删除的 fast-path dispatcher。回滚只改变 acquisition policy，不回滚 DTO、UI、数据库或现有 archive implementation。
