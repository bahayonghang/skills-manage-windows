# 设计：Release 构建验证与 draft 原子公开

## 1. 发布状态机与不变量

发布对象的可见状态是本任务的核心边界：任何未经同一 tag/SHA 的质量门禁、构建、签名、metadata、附件和 checksum 验证的内容都只能存在于 Actions artifact 或 GitHub draft 中。

```text
tag push / workflow_dispatch(tag)
  -> resolve context(tag, version, sha)
  -> reusable CI(checkout_ref=sha)
  -> required build matrix(checkout_ref=sha)
  -> aggregate + signature/metadata/artifact/checksum verification
  -> create or reset same-tag draft
  -> upload -> API inventory -> fresh download checksum verification
  -> PATCH draft=false
```

公开 release 的唯一状态转换是最后一个 `draft=false` API 调用。CI/matrix 失败发生在 draft 创建之前；上传后的任何失败都保留不可见 draft，便于检查但不会污染 updater 的 `/releases/latest/`。

## 2. Frozen Release Context

新增可测试的 release-context 检查脚本，输入显式 tag，输出：

- `tag`：严格 `v<semver>`，不从 branch/ref name 猜测；
- `version`：去掉 `v` 的版本；
- `sha`：tag peel 后的 commit SHA；
- `release_name`：`SkillPort v<version>`。

tag push 使用 `github.ref_name`，manual dispatch 使用必填 `inputs.tag`。脚本/前置步骤要求 tag 已存在并 fetch 完整 tag/ref，验证 peeled SHA 位于 `origin/main` 历史。结构化读取两个 JSON；Cargo 版本通过 `cargo metadata --locked --no-deps` 取得，`--locked` 同时证明根 package 与 `Cargo.lock` 同步。任何 mismatch 在 CI 和 build 之前失败。

所有 checkout 使用 frozen SHA；所有文件名、release notes、URL 和 metadata 使用 frozen tag/version。`GITHUB_REF_NAME` 不再进入 release asset 逻辑，避免 manual dispatch 把分支名当版本。

## 3. Reusable CI

`.github/workflows/ci.yml` 增加：

```yaml
workflow_call:
  inputs:
    checkout_ref:
      type: string
      required: true
```

`just-ci` checkout 取 `inputs.checkout_ref`，普通 PR/push/dispatch 则回退到事件 SHA。release workflow 以 reusable job 调用它，并让三个 build job 同时依赖 `release-context` 与该 reusable job。

旧 `release.published` trigger 删除。CI 自身的手动 dispatch 继续运行质量门禁与跨平台 smoke package；release 调用的 `workflow_call` 只运行 `just-ci`，正式 release matrix 由 `release-desktop.yml` 唯一负责，避免重复打包。

## 4. Artifact Contract

聚合 validator 根据 frozen version 匹配精确文件集合：

| 平台 | 必需文件 |
| --- | --- |
| Windows x64 | NSIS `.exe`、`.exe.sig`、MSI、ZIP、`latest.json` |
| macOS universal | DMG、ZIP、TAR.GZ |
| Linux x64 | DEB、RPM、AppImage |
| Linux arm64 | optional；若出现则 DEB/RPM/AppImage 必须齐全 |

匹配器拒绝重复、缺失和非预期文件，避免 `find | head -1` 或 stale draft asset 静默选中错误版本。Linux arm64 仍可 `continue-on-error`；其 artifact 不存在时不阻断，其任一文件存在时必须满足完整组。

验证通过后按文件名排序，对除 `SHA256SUMS` 自身外的全部最终附件计算 SHA-256，写入确定性 manifest。上传前在本地回验一次；draft 上传完成后清空本地目录，从 draft fresh-download，再以同一 manifest 回验，证明远端附件字节与被验证的集合一致。

## 5. Updater Signature And Metadata

Tauri CLI 2.11.2 没有验签子命令，因此新增一个 release-only Rust verifier binary，并在 `Cargo.toml` 直接声明已锁定的 `base64 0.22.1` 与 `minisign-verify 0.2.5`。其算法与本机锁文件对应的 `tauri-plugin-updater 2.10.1` 源码一致：

1. base64 decode release config 中的 public key；
2. `PublicKey::decode` 解析 minisign key 文本；
3. base64 decode `.sig` 内容；
4. `Signature::decode` 解析签名；
5. `public_key.verify(installer_bytes, &signature, true)`。

`release-preflight.mjs` 的 CLI 路径在结构校验后调用该 verifier；测试 fixture 覆盖有效签名与三种篡改。该 binary 只服务 release 校验，不进入 Tauri IPC、应用 capability 或 updater runtime；新增 target 不得改变 `default-run = "skillport"`、现有 `skillport-cli` 或 Tauri `mainBinaryName` 契约。

`latest.json` validator 继续要求 `windows-x86_64-nsis` 与兼容 key `windows-x86_64`，但增加严格 JSON/schema shape、精确 version/tag URL、非空 signature 以及 signature 与已验签 `.sig` 的字节文本一致性。它不宣称 macOS/Linux updater 支持。

## 6. Draft Lifecycle

聚合验证成功后使用 GitHub CLI/API：

1. 查询同 tag release；已 public 则 fail closed，draft 则校验 target commit 后复用，不存在则 `--draft --verify-tag` 创建。
2. 删除 draft 中旧附件，上传当前完整集合，避免重跑残留 stale/extra asset。
3. API 回读 `draft=true`、tag、target、asset names 和 non-zero sizes。
4. fresh-download 到空目录并校验 `SHA256SUMS`。
5. 最后一步 PATCH release id，将 `draft=false`；回读确认 public 且附件清单未变。

workflow concurrency 以 frozen tag 分组且不在 publish 中途 cancel。失败重跑会复用 draft 并重新建立附件集合。回滚是停止在 draft 或手动删除 draft；不得恢复 `release.published` 触发的旧链路。

## 7. Tests And Observability

- Vitest 解析两个 workflow，断言 trigger、frozen checkout、`needs` DAG、draft 创建位置、无 `always()` 绕过，以及唯一 publish 状态转换。
- release scripts 使用临时目录 fixture 覆盖 version、artifact inventory、metadata、checksum 的 good/base/bad cases。
- Rust verifier fixture 覆盖真实 minisign success 和 data/signature/key tamper。
- workflow 日志只记录 tag/version/SHA、文件名、大小、hash；不打印 signing private key、password 或 secret 内容。
- 真实 GitHub draft/release 演练是外部写操作，不作为未经授权的本地完成步骤；本地 contract test 模拟 predecessor failure，远端发布前再执行受控演练。

## 8. Compatibility And Task Boundary

现有 asset 命名、Windows updater endpoint、release notes fallback 和 Linux arm64 optional 策略保持不变。英文/中文文档同步更新；README 只调整入口描述，不扩写发布实现细节。

`07-24-ci-supply-chain` 后续负责把本设计留下的 Actions 引用 pin 到 full SHA，并加入依赖审计。该子任务不得在并行旧 workflow 上实施，避免重写本任务的 DAG。
