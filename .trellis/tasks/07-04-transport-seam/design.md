# Design：收拢 Local/SSH/WSL transport seam（试点 install/uninstall）

> 依据：`research/inventory-impl-families-and-dispatch-sites.md`、`research/execution-layer-and-install-path.md`（2026-07-05 实测盘点）。

## 0. 盘点修正 PRD 基线

- 「14 文件 match active_target」实测拆为三类：**真分发 8 文件 19 处**、守卫 2 文件（Obsidian/central_store_location，合法保留）、委托 4 文件（已通过 `CentralFs` façade 达到目标形态）。收敛对象只有第一类。
- 5 个 mutation 类 `_ssh_impl`（install/uninstall/delete×3）**全部是零调用点的死适配器**，仅被 `pub use` 桥保活；preview 族的 `_ssh_impl` 反而是活的远程终端实现（纯 DB+路径，无连接）。
- 仓库已有 seam 范本：`services/central_updates/fs.rs` 的 `CentralFs { Local, Remote(Box<ConnectedRemoteTarget>) }`——「解析一次、之后不再分支」。本设计泛化该形状，不发明新形状。
- `targets/exec.rs` 的 14 个 spawn 点全部收敛为两种执行形态（`output()` 捕获 / `spawn`+stdin 管道），且都先经 `base_command()` 纯构造器——runner 有天然单点。

## 1. Design-it-twice：三个候选

### 方案 A：文件系统原语 trait（SkillFs：exists/mkdir/copy/symlink/remove…），业务逻辑一份、逐原语调用

**否决**。远程侧现状是把 centralize+清槽+mkdir+落链收进一个原子 POSIX 脚本（`REMOTE_CENTRAL_INSTALL_SCRIPT`）单次往返，existing-entry 分类在 shell 内做（exit 42/43，报错文案用户可见）。拆成逐原语调用意味着：每次 install 从 ~2-3 次 SSH exec 膨胀为 6+ 次；清槽与落链之间出现 TOCTOU 窗口；exit 42/43 的报错文案与行为漂移。违反「既有语义零变化」约束，血量最大收益最虚。

### 方案 B：操作级枚举分发（纯 CentralFs 泛化：`install()` 每 arm 委托既有 native/remote impl）

把 19 处命令层 match 收进 service 层一个分发函数。**作为终点否决**：命令层干净了，但 install 仍是两份业务实现，AC1（一份业务实现）不满足，新操作仍要写两遍。**作为入口形状保留**（见方案 C 的组合）。

### 方案 C（裁决）：入口用 B 的枚举分发 + install/uninstall 内部做「编排/执行」分离

- **入口**：`InstallTransport { Local, Remote(ConnectedRemoteTarget) }`，`for_target(&ActiveTarget)` 解析一次（Local 零开销，Remote 连一次可复用——批量路径天然受益）。
- **编排一份**：install 的业务决策骨架（guard → agent/central 查询 → canonical 路径 → 同根 native 捷径 → skip 检测 → method 解析 → 落链 → DB 记录）写一遍，作用于两个传输。
- **执行差异进 adapter**：transport 暴露少量**复合**执行原语（不是文件系统原语）——`ensure_centralized` / `detect_existing`（Local 有、Remote 返回 None，如实保留现状差异）/ `place_install`（Local=离散 fs_util 步骤含 auto 回退；Remote=整段原子脚本单次往返）/ `remove_install`（Local=按 link_type 分类删除；Remote=remove_tree）。远程原子性、往返数、exit 42/43 文案全部原样保留。
- **runner 注入**在 targets 层独立成缝：`CommandRunner` trait 挂在 `ConnectedSshTarget`/`ConnectedWslTarget` 之下，`ConnectedRemoteTarget` 之上的一切（脚本拼装、退出码分支、UTF-8 解析）全部变为进程外可测；installation/local_remote_sync/scanner 三个消费者同时受益。

## 2. 目标形状

### 2.1 targets 层：CommandRunner（Requirement 3）

```rust
// targets/exec.rs（或新 targets/runner.rs）
pub(crate) trait CommandRunner: Send + Sync {
    /// stdin=None → .stdin(null).output()；stdin=Some → spawn + write_all + wait_with_output。
    fn run(&self, cmd: Command, stdin: Option<&[u8]>) -> std::io::Result<Output>;
}
pub(crate) struct ProcessRunner;   // 逐字节复刻今天的两种执行形态
```

- `ConnectedSshTarget` / `ConnectedWslTarget` 增加 `runner: Arc<dyn CommandRunner>` 字段；生产构造路径（`connect_ssh_target` / WSL 连接）默认 `ProcessRunner`，`#[cfg(test)]` 构造器接受注入。
- `exec.rs` 中 12 个 spawn 现场（SSH 6 + WSL 6）改为 `self.runner.run(...)`；`base_command()` 纯构造器**不动**；`wsl_discovery.rs` 的 2 处发现用 spawn **不动**（非操作执行面）。
- 方法内的后处理（UTF-8 解析、`exists`/`inspect_path` 退出码分支、stderr 拼错误）留在 exec.rs——这正是 FakeRunner 要测的半边。
- 现状是 `async fn` 内直接阻塞调 `std::process`；runner 保持同步契约、调用位置不变，**不**顺手引入 spawn_blocking（行为零变化；该债在试点结论里记录）。

### 2.2 installation 层：InstallTransport + 单份编排

```rust
// services/installation/transport.rs
pub enum InstallTransport {
    Local,
    Remote(ConnectedRemoteTarget),
}
impl InstallTransport {
    /// Local → 零开销；Ssh/Wsl → connect_remote_target 连一次。
    pub async fn for_target(target: &ActiveTarget) -> Result<Self, InstallationError>;
}
```

编排函数（一份实现，替代 local `_impl`/`_copy_impl`/`_auto_impl` + remote `ssh_with_connection` 四份骨架）：

```rust
// services/installation/install.rs（native.rs/remote.rs 的编排半边迁入）
pub async fn install_skill(pool, transport: &InstallTransport, skill_id, agent_id, method: &str)
    -> Result<InstallOutcome, InstallationError>;
pub async fn uninstall_skill(pool, transport: &InstallTransport, skill_id, agent_id, row_id: Option<&str>)
    -> Result<(), InstallationError>;
```

骨架步骤与差异点归属：

| 步骤                         | 归属                                  | 备注                                                                                                                                                                                                                             |
| ---------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| guard `agent_id=="central"`  | 编排                                  | 两侧现状一致                                                                                                                                                                                                                     |
| agent/central 查询           | 编排                                  | 一致                                                                                                                                                                                                                             |
| method 解析                  | 编排调 transport 策略                 | Local: symlink/copy/auto；Remote: `symlink`→需 `symlink_allowed` 否则 `RemoteSymlinkDisabled`，其余一律 copy（**逐调用点保持今天的强转结果**）                                                                                   |
| canonical/target 路径拼接    | transport                             | Local=`PathBuf::join`；Remote=`remote_join`（path-policy 单点）                                                                                                                                                                  |
| 同根 native 捷径             | 编排调 `transport.ensure_centralized` | Local=`ensure_centralized`+SKILL.md 存在检查；Remote=`ensure_remote_centralized`；记录统一走共享 record（link_type="native"）                                                                                                    |
| skip 检测                    | `transport.detect_existing`           | Local=`detect_existing_agent_install`；Remote=None（现状无 skip，语义如实保留）                                                                                                                                                  |
| 落链                         | `transport.place_install(plan)`       | Local=create_dir_all→ensure_replaceable→symlink（相对）/copy，auto 在此内部回退（仅重试落链步，等价于今天重跑全流程的幂等结果）；Remote=managed_copy 旗标+`REMOTE_CENTRAL_INSTALL_SCRIPT` 原样单次往返，symlink 指绝对 canonical |
| DB 记录                      | 编排                                  | 共享 upsert，link_type/paths 由 place 结果给出                                                                                                                                                                                   |
| uninstall：row_id 观察行路径 | 编排，仅 Local 生效                   | 现状远程命令忽略 row_id，保持                                                                                                                                                                                                    |
| uninstall：共享中央守卫      | 编排                                  | 两侧现状一致（SharedCentralUninstall）                                                                                                                                                                                           |
| uninstall：删除              | `transport.remove_install`            | Local=symlink/copy 分类删、拒删非托管目录；Remote=remove_tree 不查 link_type（现状差异如实保留）                                                                                                                                 |

### 2.3 调用面迁移

- `commands/linker.rs`：5 个命令的 6 处分叉全部改为 `InstallTransport::for_target` + 编排/批量调用；不再 import `connect_remote_target`（Requirement 4）。批量命令把「连一次、循环复用」的形状交给 transport 天然获得。
- `commands/collections.rs:259` 分叉消除：`batch_install_collection_impl` 接 transport 下传（内部 install 走编排；远程 method 强转结果与今天逐字节一致）。
- `services/installation/batch.rs`：`install_central_skill_to_agent_outcome_by_method` → 编排调用；批量中央安装的远程循环从 linker.rs 迁入 batch.rs 与本地循环合一。
- **project 安装走方案 B 粒度**（dispatcher-only）：`project.rs` 新增 transport 分发入口，Local/Remote 各自委托既有 project impl，不做本体统一（有效但试点外，结论记录）。
- 删除：`install_skill_to_agent_ssh_impl`、`uninstall_skill_from_agent_ssh_impl`（死适配器）及其 `pub use`；`native.rs`/`remote.rs` 中被编排替代的骨架函数；linker.rs 的 `pub use` 桥同步收缩。central_skills 的 3 个死 `_ssh_impl` **不动**（试点外，结论记录）。

### 2.4 错误与规范契约

- `InstallationError` 不新增、不删改任何 `#[error]` 文案；`TargetsError → InstallationError::Remote(String)` 的拍平边界收敛到 transport adapter 内部单点（现状 6 处散写），文案 `e.to_string()` 结果不变。
- 重 IO 照旧走 `fs_util`（`run_blocking_fs_with` 唯一包装入口）；path 拼接照旧 `remote_join`/`paths.rs`（path-policy）。

## 3. 测试策略（AC2 证明）

- **targets/**：`FakeRunner`（记录 `Command` program+args+stdin，返回罐头 `Output`）单测：`exists` 退出码三分支、`inspect_path` 输出解析、`run_script` 的 stdin 管道与参数、失败传播（非零退出码→stderr 进错误）。SSH/WSL 各覆盖代表路径。
- **installation/**：fake-backed `ConnectedRemoteTarget` + `test_support::mem_pool_with_home` 端到端远程 install/uninstall：脚本参数断言（canonical/source/target/agent_dir/method/managed_copy 六参）、`RemoteSymlinkDisabled` 门、同根 native 捷径、DB 行断言——**远程执行半边首次获得执行路径覆盖**。
- 既有测试全绿：installation 64+、targets 38+、local_remote_sync 7（不触碰）。

## 4. 验收目标值（AC3 量化）

| 指标                              | 基线                                                     | 目标                                              | 复核方式                                                                       |
| --------------------------------- | -------------------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------ |
| install 家族实现数                | 5（_impl/_copy/_auto/_remote/_ssh）+ ssh_with_connection | 1 份编排                                          | grep `install_skill_to_agent` 仅剩编排+壳                                      |
| 命令层真分发                      | 8 文件 19 处                                             | 6 文件 12 处（linker 6 处、collections 1 处归零） | grep `ActiveTarget::Local`/`is_remote_like` in commands/                       |
| commands/ 直接编排连接原语        | linker.rs import `connect_remote_target`                 | 0                                                 | grep `connect_remote_target` in commands/（targets.rs 自身的连接测试命令除外） |
| 死 `_ssh_impl`（installation 域） | 2                                                        | 0                                                 | grep                                                                           |

## 5. 风险与回滚

- 风险最高点：remote place_install 的 managed_copy/脚本参数次序——用脚本参数断言测试钉住；local auto 回退语义——用既有 Windows 门控测试+新增回退单测钉住。
- 提交分段（每段可独立 revert）：① targets CommandRunner + FakeRunner 测试（纯加法）；② installation 编排/transport + 调用面迁移 + 死代码删除；③ spec/docs。无 schema、无数据迁移，回滚=revert。
- 若实施中发现编排骨架被差异点撕裂（hook 超过上表清单、或需引入布尔开关表达传输差异），停手回到 design 修订——那是 seam 形状不成立的信号（PRD 允许以试点结论收场）。

## 6. 推广决策留给试点结论（写回本任务 notes + 父任务）

- delete×3 / preview×2 家族（central_skills）、scanner、agents、github_import、usage、local_remote_sync 的收敛顺序与是否值得；
- central_skills 3 个死 `_ssh_impl` 的删除；
- preview 族 `_ssh_impl` 命名统一；
- `Remote(String)` 拍平边界的类型化改进；
- exec.rs 阻塞调用无 spawn_blocking 的既有债。
