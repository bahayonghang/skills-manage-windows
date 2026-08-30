# Skills CLI 数据、Placement 与安全 IPC 契约 — 执行计划

## Dependencies

- 无 child 前置；本任务是其余五个 child 的显式前置。
- 保持 `planning`，直到用户审阅更新后的整棵任务树并另行批准实施。
- 实施前记录工作树并保护无关改动；本任务不得与其它 child 并行修改生成 IPC/spec 文件。

## Ordered Steps

1. **持久化 CLI capability 研究证据**
   - 按 `research/skills-cli-capability-probe.md` 协议，对 pinned `skills@1.5.23` 采集 add/remove `--help`，
     并在隔离临时 HOME/probe 中验证 pinned full-SHA source 与 direct-copy refresh。
   - 每项记录命令、UTC 时间、版本、退出状态、原始 stdout/stderr 或安全失败；不得读取或修改真实用户 HOME。
   - 失败/未执行保持 `UNVERIFIED/unsupported`；不把 `--force`/`--keep-links` 或未证明行为放入产品 argv。

2. **先写 placement/lock 红灯测试**
   - lock camel/snake/empty/missing 字段；placement 五态、稳定顺序、兼容 `agents` 派生。
   - Windows junction、Windows/Unix symlink、wrong/broken link、普通目录、文件、canonical missing、
     disabled/not-detected reasonCode fixtures。

3. **实现 Rust DTO 与 classifier**
   - 扩展 `lock.rs`、`inventory.rs`、`mod.rs`，新增 `placement.rs`。
   - 新 UI 以 `placements` 为权威；不增加 `agentIds`/`linkTargets` 平行数组。

4. **实现 typed directory-link primitive**
   - 在 installation fs utility 中加入 inspect/create/remove verified link；Windows 使用 `windows-sys`
     reparse API 创建 junction，Unix 使用 symlink。
   - 不改变既有 install `method=auto` fallback；Skills CLI link 路径不 fallback copy。
   - 先跑纯分类/失败清理测试，再跑 Windows native junction test binary。

5. **实现 bounded doc 与 safe reveal**
   - 新建 `services/skills_cli/files.rs`；共用 lock ownership、canonical containment 与 1 MiB bounded reader。
   - 抽取/复用 path-safe file-manager launcher；command 只接收 skill name。
   - 补 exactly-limit、growth、UTF-8、missing、escape、非目录与 spawn failure tests。

6. **实现 link/unlink service 与 command**
   - command acquire exclusive job lease；service acquire Local mutation guard，under-guard 重算 placement。
   - 只允许 Missing→ManagedLink 与 ManagedLink→Missing；普通目录、conflict/unavailable 零写入。
   - 补 cancel/busy/lock-order/operation-log redaction 与 partial-create cleanup tests。

7. **实现安全 remove 与恢复清单**
   - 新增 `skills_cli_preview_remove_global` 与结构化 `SkillsCliRemovePlan`；改 `skills_cli_remove_global` result，
     新增 domain-local versioned remove manifest/path helper。preview 无 path/argv，execution 在 guard 内 fresh recheck。
   - 写 prepared → staged → metadata_committed → cleanup 流程、lock fingerprint CAS 与幂等 recovery。
   - 注入每个 phase/rename/link/atomic persist/cleanup/collision failure；证明 conflict 零写入、copy 字节级保留。
   - 不调用 `skills remove`，不使用任何未验证 flag。

8. **实现 export writer**
   - 独立 parser 校验 v1 envelope、`.json`、1 MiB 与 forbidden fields。
   - blocking same-directory temp + flush/sync + atomic persist；补旧目标保留和 temp cleanup tests。

9. **收紧 recent-source settings policy**
   - exact key + `SettingCategory::SkillsCli`；数组 schema、8 项/16 KiB/2048-byte、去重、control char、
     source/credential validation。
   - 补 single/batch zero-write、audit redaction、重启 roundtrip tests。

10. **错误、command registry 与前端类型**
    - 增补 `SkillsCliError` reviewed variants 和 `ipc_error` mapper。
    - 在 `ipc_registry.rs` 登记 read/link/unlink/reveal/export/preview-remove 六条 command；同步 remove result。
    - 更新 browser fixtures、typed IPC coverage、`src/types/index.ts` re-export。

11. **规范与生成物**
    - 更新 `.trellis/spec/backend/skills-cli-global.md` 的 signatures、placement、ownership、remove recovery、
      settings、error matrix、tests；必要时同步其直接引用的 lifecycle/path specs。
    - 运行 `pnpm ipc:codegen`、`pnpm docs:gen`；生成后连续两次 read-only check。

12. **Focused gates**
    - `cargo test --manifest-path src-tauri/Cargo.toml skills_cli --locked`
    - settings policy / bounded ingestion / IPC registry focused Rust tests
    - Windows junction test binary 的真实运行；若环境不支持，单独报告 `UNVERIFIED`
    - `cargo fmt --all -- --check`
    - `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
    - `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --locked`
    - `pnpm typecheck && pnpm ipc:codegen:check && pnpm docs:gen:check`

13. **Repository gate**
    - `just ci`
    - `git diff --check`
    - `.trellis/scripts/task.py validate 08-26-backend-contract`

## Risk and Rollback Points

- Windows reparse handling必须识别 link 本身而不是跟随后的 directory；任何 ordinary directory 命中删除 helper
  都是 release blocker。
- remove 的 lock atomic replace 是 metadata commit point；有非终态 recovery manifest 时禁止回滚 binary。
- generated IPC/docs 与 Rust source 同提交，不能手改 artifact 或只回滚一侧。
- CLI capability probe、Windows native junction、installer/WebView2 不是编译可证明的证据；未执行必须标 `UNVERIFIED`。

## `task.py start` Gate

- [ ] 父/本 child PRD、design、implement 与 JSONL manifests 通过复审和 precheck。
- [ ] `research/design-contract.md` 存在并是规范依据；缺失的外部 handoff 不再被引用。
- [ ] `research/skills-cli-capability-probe.md` 已按协议得出逐项 `VERIFIED_SUPPORTED`、
      `VERIFIED_UNSUPPORTED` 或 `UNVERIFIED` 结论；产品 capability plan 对后两者均 fail closed。
- [ ] 用户在最新规划摘要之后明确批准实施。
- [ ] task 仍为 `planning`、`base_branch=dev`，未启动其他依赖 child。
