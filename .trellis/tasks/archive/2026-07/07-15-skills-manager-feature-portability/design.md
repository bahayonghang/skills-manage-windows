# 父任务设计

## 1. Scope

父任务只协调两个交付域：

```text
stable skill uid + central mutation lock
                  |
                  v
        shared Rust cli_api
                  |
          +-------+-------+
          |               |
      Tauri IPC      skillport-cli
```

Git 备份、历史版本和多设备合并已被用户明确排除，不能以“为未来预留”为由引入 Git repository、merge protocol、snapshot table 或相关 UI。

## 2. Why Identity And Lock First

CLI 若先暴露当前目录型 `skills.id`，后续新增稳定身份会造成脚本契约迁移；CLI 若先开放 install/sync，又没有跨进程锁，会允许桌面端和 CLI 同时置换同一目录。因此实施顺序必须是：

1. 新增不可变 `uid`，但保留原 `skills.id` slug 兼容语义。
2. 建立 GUI/CLI 共用的中央 mutation lock，并让现有 GUI 写路径接入。
3. CLI 只通过稳定 resolver 和共享 mutation use case 开放变更命令。

这比把 `skills.id` 主键整体改成 UUID 更小：当前没有 Git merge 对 path-independent identity 的强一致性要求，增量 `uid` 足以提供稳定外部引用，并避免重写所有 relation primary keys。

## 3. Cross-Child Contracts

### 3.1 Skill Reference

统一输入：

```rust
pub enum SkillRef {
    Uid(String),
    Slug(String),
    Name(String),
}
```

解析优先级必须确定且无歧义：精确 `uid` → 精确现有 `id`/slug → 唯一 name；name 多匹配返回冲突，不静默选择。CLI JSON 同时返回 `uid` 与兼容 `id`。

### 3.2 Mutation Boundary

跨进程锁属于 service/infrastructure，不属于 CLI：

```text
prepare network/archive outside lock
    -> acquire CentralMutationGuard
    -> revalidate current DB/filesystem state
    -> atomic filesystem apply
    -> DB persist / operation log
    -> release
```

两个子任务必须复用一个 guard。禁止 CLI 在外层锁一次、service 内再次获取同一锁造成自死锁；需要锁内复用时采用显式 guard token 或单一顶层 use case。

### 3.3 Output And Errors

领域 service 返回 typed errors/DTO。Tauri command 映射为 IPC string，CLI 映射为 stderr/JSON envelope/exit code。不得让 CLI 文案反向渗入 service。

## 4. Compatibility

- `skills.id` 保持现有 slug 兼容键，目录仍按它命名；新增 `uid` 不改变现有 UI 展示。
- import overwrite、scan upsert、central relocation 和 update 必须保留已有 `uid`；新实体才生成新 `uid`。
- portable state 增量支持可选 `uid`，并继续接受旧格式。
- CLI 首发只初始化 Local `DbPool`、`SystemSecretStore` 和 Local target，不复刻 `AppState`。

## 5. Rollout And Rollback

- 身份/锁子任务先合入；新增列和锁默认不改变用户可见行为。
- CLI binary 后续增量启用，查询命令和 mutation 命令共用同一 façade。
- 若 CLI 回滚，新增 `uid` 与锁可保留，不影响桌面端。
- 若 `uid` backfill 失败，数据库初始化失败并保留原值，不得留下部分 NULL/重复状态。
- lock 获取失败必须返回 busy/timeout，不得退化为无锁写入。

## 6. Integration Gate

父任务最终只做跨子任务检查：稳定 resolver、共享锁、CLI façade、Tauri command 的调用链一致；随后运行 `just ci`、CLI 离线 E2E 和 Windows Tauri bundle。
