# 稳定身份与并发锁设计

## 1. Data Model

采用 additive 模型：

```text
skills.uid   immutable UUID, unique, stable API/CLI reference
skills.id    existing slug-compatible primary key, retained
skills.canonical_path / file_path  existing physical paths
```

`uid` 是实体身份，`id` 是兼容 slug/key。当前 relation tables 继续引用 `skills.id`，避免无 Git 合并收益支撑的全库主键迁移。

### 1.1 Migration

1. 在 `db/schema/core.rs` 通过 `ensure_column` 增加 nullable `uid`。
2. 在一个 transaction 中为 NULL/空值 row 生成 UUID v4。
3. 校验无 NULL、空值和重复值后创建 unique index。
4. fresh insert 明确写 `uid`；upsert 的 conflict-update 分支禁止覆盖现有 `uid`。

SQLite 无法一次安全增加 `NOT NULL DEFAULT random UUID`，因此非空约束由初始化校验、repository write contract 和 unique index共同保证。迁移重复运行必须保持原值。

### 1.2 Identity Preservation

- scanner 对已存在 `id` upsert 时读取/保留 `uid`。
- import overwrite 与 central update 复用目标 row 的 `uid`。
- store relocation 只更新 path，不创建新 row。
- portable-state import：若 manifest `uid` 命中同一实体则保留；若 `uid` 与 slug 指向不同实体，必须进入明确冲突而非重写身份。
- 删除后重新创建同 slug 是新实体，生成新 `uid`。

### 1.3 Resolver

新增 backend 单点 resolver，输入保持 string 便于 IPC/CLI：

```text
exact uid -> exact skills.id -> unique case-sensitive name -> error
```

不得用模糊包含匹配或“第一条”。DTO 对中央技能增加 `uid`；兼容 `id` 字段继续存在。

## 2. Central Mutation Lock

### 2.1 Location And Primitive

- 锁文件位于 `paths::app_data_dir()/locks/central-mutation.lock`，不放在可迁移的 central root 内。
- 使用跨平台 advisory file lock（优先 `fs2::FileExt`）；依赖为纯 Rust/系统文件锁，不引入 native Git 依赖。
- async service 通过现有 blocking helper 或 `spawn_blocking` 获取锁，避免阻塞 Tokio worker。

### 2.2 API

```rust
pub struct CentralMutationGuard { /* owns locked File */ }

pub async fn acquire_central_mutation_guard(
    operation: &'static str,
    timeout: Duration,
) -> Result<CentralMutationGuard, CentralMutationError>;
```

guard drop 释放锁。错误枚举至少区分 IO、Busy/Timeout 和 task join；用户错误经既有 backend error 映射显示。

### 2.3 Critical Section

正确顺序：

```text
download/inspect/plan
  -> acquire guard
  -> reload DB row + inspect target path
  -> reject stale plan if identity/path changed
  -> atomic stage/swap/delete
  -> persist DB relation/update state
  -> operation log best effort
  -> release guard
```

锁不得包住 GitHub/skills.sh 网络请求。现有 nested service 调用不得重复获取锁；顶层 mutation use case 持 guard，内部 helper 接收 `&CentralMutationGuard` 或使用明确 `_guarded`/`_unlocked` 私有边界。

### 2.4 Covered Local Mutations

- GitHub import / skills.sh install final apply
- `ensure_centralized` 与本机 agent install 所需中央化
- central update atomic write 与 copy refresh 的中央源更新部分
- central skill/repository delete
- portable-state import
- central-store relocation apply

扫描 DB observation、纯查询、marketplace search、preview 和 Local→Remote snapshot read 不持锁。SSH/WSL 远端脚本继续使用其现有原子机制，本任务不声称提供跨主机分布式互斥。

## 3. Cross-Process Test Strategy

- 单元测试固定 schema/backfill/resolver/upsert preservation。
- integration helper 进程 A 获取锁并等待 stdin/barrier；进程 B 尝试获取，断言等待或 timeout；释放 A 后 B 成功。
- crash case 强制结束持锁 helper，再断言新进程能获取。
- mutation service tests 注入 fake guard/lock path，验证网络准备先于锁、锁内重新校验和失败不写 DB。

## 4. Compatibility And Rollback

- Schema additive，不删除/重命名旧列；旧 UI 忽略额外 `uid`。
- portable-state version 只做向后兼容字段扩展，旧 manifest 无需重写。
- 若上层暂时回滚，可停止消费 `uid`，但不删除已生成列或重用值。
- lock 接入后不可在生产 fallback 到无锁；故障时 mutation 返回错误，read-only 功能保持可用。
