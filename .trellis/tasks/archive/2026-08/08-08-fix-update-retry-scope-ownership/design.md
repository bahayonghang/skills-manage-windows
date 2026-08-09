# 技术设计：Update Center 仓库归属与重试合并不变量

> `prd.md` 定义结果和验收，本文定义模块边界、数据流、兼容策略与取舍。

## 0. 设计结论

| 决策点 | 方案 |
| --- | --- |
| 新清单的仓库归属 | 直接携带 `PreparedSkillUpdate.assignment.repository.id`，不从 scope 的 repository 列表或 state URL / branch 反向猜测 |
| 远端缺失的中间表示 | 在 inventory 计算模块内用私有的“仓库归属 + state”值对象传递，使新生产路径无法产生无归属的仓库型条目 |
| 旧清单兼容 | 重试时读取目标仓库当前的 Central member skill ids；只为 `repository_id = None` 的旧条目提供受限归属回退 |
| 合并替换规则 | `Some(repository_id)` 只按显式仓库判断；`None` 才按当前 member skill id 判断，避免删除显式属于其它仓库的条目 |
| 重复键处理 | 保留数据库严格 INSERT；在事务前校验 entry key 唯一，失败时返回 typed inventory invariant |
| IPC 边界 | 为该 typed variant 注册固定 Update Center code / message / retryable=false；不改全局遗留 unique 文本规则 |
| 兼容与迁移 | 保留 `Option<String>` 和现有表结构，无 schema / payload 迁移；旧清单在首次目标重试时自愈 |

## 1. 模块边界

本修复保持三个深模块，各自通过小接口承担一项不变量；不新增跨层 public API。

### 1.1 Inventory computation module

- **Interface**：`compute_skill_update_inventory(...) -> SkillUpdateInventory`
- **Implementation**：`inventory/mod.rs`、`inventory/relocation.rs`
- **责任**：把准备好的技能、仓库快照和 scope 转为无重复、归属明确的新清单。
- **不变量**：只要一个 actionable state 来自 `PreparedSkillUpdate` 的仓库分配，新清单中的对应项就必须带 `Some(repository_id)`。

### 1.2 Repository retry merge module

- **Interface**：现有 `retry_failed_repositories_impl(...) -> SkillUpdateInventory` 保持不变。
- **Implementation**：`inventory/retry.rs` 内新增私有 `RepositoryRetryTargets`（名称可在实现时按现有风格调整）。
- **责任**：计算仓库分片、识别基线中属于目标仓库的条目、返回并持久化完整面板清单。
- **不变量**：每个目标仓库旧结果被完整替换；不相关结果逐项保留。

### 1.3 Inventory persistence module

- **Interface**：现有 `persist_refresh_inventory(...)` 保持不变。
- **Implementation**：`inventory/persistence.rs` + `central_updates/error.rs` 的 typed error adapter。
- **责任**：把逻辑清单转换为 entry rows，在进入数据库事务前验证唯一键，再调用严格的 repository CRUD。
- **不变量**：同一 `inventory_id` 中 `(bucket, entity_key)` 唯一；违反时不修改已持久化清单。

SQLite 与临时文件系统都是可本地替代依赖，因此回归测试继续通过 service 接口使用内存数据库和 `TempDir`，不为它们新增 port / trait seam。

## 2. 新清单的权威仓库归属

### 2.1 当前问题

`Skills` / `Platform` scope 的 `repository_ids` 为空，导致 `valid_repositories` 和 `repo_by_id` 为空。随后 `repository_id_for_state(repo_by_id, state)` 通过 URL / branch 反查时得到 `None`。这把“本次不扫描仓库新增项”错误地等同于“技能没有仓库归属”。

### 2.2 新数据流

```text
SkillRefreshScope
    -> prepare_skill_updates
    -> PreparedSkillUpdate { assignment.repository.id, ... }
    -> classify remote state
       ├─ UpdateAvailable -> UpdatableSkill { repository_id: Some(assignment id) }
       └─ RemoteMissing   -> RepositoryOwnedState { repository_id, state }
                              -> relocation / final bucket
                              -> RemoteMissingSkill { repository_id: Some(id) }
```

实现约束：

1. `UpdateAvailable` 分支在移动 `state_result` 之前复制当前 `assignment.repository.id`，直接写入 `UpdatableSkill`。
2. `remote_missing_states` 不再只保存裸 `SkillUpdateState`；改用 inventory 私有值对象同时保存 `repository_id` 和 `state`。
3. `reconcile_relocated_remote_skills` 从该值对象读取仓库 ID，不再通过 `repository_id_for_state` 反查。
4. Inventory 最终构建 `RemoteMissingSkill` 时直接使用该 ID。通用 repository-sync summary 仍可保留现有反查逻辑；本任务不扩张到不相关接口。
5. `PendingRelocation` 已直接携带 repository ID，保持该路径，并补断言确保归位后的 updatable 仍有归属。

公开的 `UpdatableSkill.repository_id` / `RemoteMissingSkill.repository_id` 继续是 `Option<String>`，用于读取旧 payload 和表达历史未知状态；正常的新仓库型生产路径不再产生 `None`。

## 3. 旧 inventory 的自愈合并

### 3.1 目标集合

`retry_failed_repositories_impl` 对归一化后的每个目标 repository id 调用现有 `db::get_central_skill_ids_by_repository`，构建：

```rust
struct RepositoryRetryTargets {
    repository_ids: HashSet<String>,
    legacy_member_skill_ids: HashSet<String>,
}
```

该结构只存在于 `retry.rs`，向合并函数提供一个语义化判断：

```rust
fn owns_actionable(&self, repository_id: Option<&str>, skill_id: &str) -> bool {
    match repository_id {
        Some(id) => self.repository_ids.contains(id),
        None => self.legacy_member_skill_ids.contains(skill_id),
    }
}
```

关键点：显式 `Some(other_repo)` 永远不会因为 skill id 恰好出现在目标成员集合里而被移除。member fallback 只服务旧 `None` 数据。

### 3.2 合并规则

| Bucket | 基线剔除条件 | 分片处理 |
| --- | --- | --- |
| `updatable` | `owns_actionable(repository_id, state.skill_id)` | 全量追加该桶 |
| `remote_missing` | 同上 | 全量追加该桶 |
| `remote_added` | 显式 repository id 命中 | 全量追加该桶 |
| `failed_repositories` | 显式 repository id 命中 | 全量追加该桶 |
| `unsupported` | 不剔除 | 忽略分片，保留基线 |
| 平台 / orphan 桶 | 不剔除 | 忽略分片，保留基线 |

先剔除再追加使“目标技能变为 up-to-date，分片不再产出条目”自然删除旧 stale row。不得把分片和基线简单 dedup，因为 dedup 无法表达哪个版本权威，也不能删除已消失的旧项。

### 3.3 一致性与失败策略

- 目标成员关系从同一个 `DbPool` 读取，并紧邻分片计算 / 合并；它是旧空归属行唯一允许的回退事实源。
- 不从 state URL、branch、source path 或 skill name 猜测，避免仓库重命名、同源配置或路径移动造成误删。
- 若并发变化导致旧 `None` 行无法归属，合并会保守保留；若随后与分片冲突，持久化不变量校验会安全失败并保留旧 run，而不是覆盖任一条。
- 本任务不扩大 Central mutation lock 或改变 compute 期间现有的 relocation / pending-addition 写入边界；相关写入现有幂等语义保持不变。

## 4. 持久化不变量与错误边界

### 4.1 前置唯一性校验

`persist_refresh_inventory` 完成 entry 转换后，对 `(inventory_id, bucket, entity_key)` 建立 `HashSet`。首次重复立即返回 `CentralUpdatesError::InventoryInvariant`，不调用 `db::replace_skill_update_inventory`。

约束：

- 不在错误中携带重复 key、skill id 或 inventory id。
- 不做 `INSERT OR REPLACE`、不删除重复项、不选择“最后一个”作为赢家。
- 数据库主键继续作为最终防线；前置校验是让本域自身产生的重复在事务前得到稳定分类。
- 因校验发生在 transaction 前，已有 run / entries 保持原样。

### 4.2 Typed error 映射

建议固定契约：

| 字段 | 值 |
| --- | --- |
| Domain variant | `CentralUpdatesError::InventoryInvariant` |
| Diagnostic category | `central_updates.inventory_invariant` |
| IPC code | `central_updates.inventory_invariant` |
| Public message | `The update inventory could not be finalized.` |
| Retryable | `false` |
| Operation phase | `inventory_persistence` |

`CentralUpdatesError::to_ipc_error` 对该变体显式输出已注册的 coded message；`reviewed_operation_failure` 和 `diagnostic_category` 使用静态字面量。`ipc_error::public_message_for_code` 注册固定英文，前端中英文 `backendErrors` 同步登记。

不删除或改写 `legacy_plain_message` 中全局 unique-constraint 兼容规则：它服务多个尚未类型化的历史边界，直接修改会产生超出本任务的行为变化。通过 typed 前置错误，本 inventory 重复路径不再到达该规则。

## 5. 回归测试设计

测试 seam 为现有 service interface + 内存 SQLite + 临时 Central 目录，不 mock 私有实现。

### 5.1 精确红灯

在 `inventory/tests.rs` 固化诊断阶段的最小场景：

- 同一仓库有 `stable`（远端内容变化）和 `gone`（远端路径消失）；
- 先以 `scope_skills([stable, gone]) + Regular` 刷新；
- 再对该 repository 以 `mode_override = Sync` 调 `retry_failed_repositories_impl`；
- 修复前应复现 SQLite 唯一键失败，修复后断言一个 updatable、一个 remote missing、无 failed repository，且从原 `skills:regular:*` run 回读一致。

实现时先提交测试并运行确认红灯，再修改生产代码。

### 5.2 生产者不变量

- Skills scope：updatable 与 sync remote-missing 均为 `Some(expected_repo_id)`。
- Platform scope：通过 agent observation 选择同样的两个技能，验证归属和平台桶保持。
- Repositories / All 的既有归属断言保持通过。

### 5.3 旧数据兼容

通过测试 helper 在已持久化 payload 和 entry column 中把目标可操作项模拟为 `repository_id = null`：

1. retry 后旧项被替换且 reload 无重复；
2. retry 分片不产出目标技能时，旧 stale updatable 被移除；
3. 一个不属于目标当前 membership 的 `None` 项保持不变；
4. 一个显式属于其它仓库的项即使 skill id 碰撞也保持不变。

### 5.4 不变量错误

- 人为构造同 bucket / skill id 的两条 inventory，调用 persistence interface。
- 断言 typed domain variant、精确 IPC envelope 和 operation metadata 分类。
- 先持久化一份合法 inventory，再尝试重复 inventory，回读确认旧 run 未被修改。
- 断言序列化错误不含 `UNIQUE constraint failed`、表名、skill id、路径、URL 或测试 secret。

## 6. 文件改动面

| 文件 | 预期改动 |
| --- | --- |
| `src-tauri/src/services/central_updates/inventory/mod.rs` | 直接传播 assignment repository id；使用 owned remote-missing state |
| `src-tauri/src/services/central_updates/inventory/relocation.rs` | 消费 owned state，去除 inventory relocation 的反向归属推断 |
| `src-tauri/src/services/central_updates/inventory/retry.rs` | 目标 membership 加载与兼容合并规则 |
| `src-tauri/src/services/central_updates/inventory/persistence.rs` | entry key 前置唯一性校验 |
| `src-tauri/src/services/central_updates/error.rs` | typed inventory invariant、operation / diagnostic 映射 |
| `src-tauri/src/ipc_error.rs` | 注册固定公开错误码与文案 |
| `src-tauri/src/services/central_updates/inventory/tests.rs` | 精确回归、Platform、旧数据和持久化安全测试 |
| `src-tauri/src/services/central_updates/core/tests.rs` 或 error 定向测试 | IPC envelope / 错误分类断言，按现有测试布局选择 |
| `src/i18n/locales/en.json`、`src/i18n/locales/zh.json` | 新错误码本地化，实际路径以仓库现状为准 |
| `.trellis/spec/backend/update-inventory-retry.md` | 归属不变量、旧数据回退和 strict persistence 契约 |

不预期修改 DB schema、Tauri command registry、generated IPC command map 或发布 workflow。

## 7. 兼容、发布与回滚

- **向后兼容**：公开字段和序列化形状不变；旧 `null` 仍可反序列化。
- **无需迁移**：不主动重写所有旧 inventory；相关仓库首次重试时按成员关系自愈，完整刷新也会生成带归属的新数据。
- **回滚**：无 schema 与不可逆数据操作，可直接回退代码和 spec。已被新版本写入的 `repository_id` 是现有字段，旧版本可继续读取。
- **用户数据**：不自动 keep / delete 上游已删除技能，不清库，不修改 Central 目录。
- **平台**：改动只使用现有 Rust / SQLite 能力，不引入平台 API；无打包配置改动，因此不把 Windows bundle 作为本任务验收项，最终仍由跨平台 `just ci` 兜底。

## 8. 取舍

- 选择 assignment 直接归属，而不是扩充 scope repository map：assignment 是每个技能的权威事实，接口更深且不受检查范围影响。
- 选择内部 owned state，而不是在最终阶段重新查表：归属随状态一起流动，使生产者缺陷更难再次出现。
- 选择 current membership 兼容旧 `None`，而不是全库反推 source identity：范围小、可证明、能处理 up-to-date 后无分片条目的场景。
- 选择 fail-closed invariant，而不是自动 dedup：重复说明上游合并语义仍有缺陷，静默选边会产生陈旧或错误决策。
- 选择本域 typed error，而不是改全局 legacy mapper：修复准确覆盖已知路径，同时避免改变其它历史 command 的冲突语义。
