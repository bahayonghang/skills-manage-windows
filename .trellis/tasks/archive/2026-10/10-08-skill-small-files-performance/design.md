# 设计：大量小文件 Skill 生命周期性能

状态：用户于 2026-10-08 要求本地提交并归档；选定 D1–D4 已实施，性能验收仍有缺口。生产优化在保存基线、锁定 [research/implementation-budget.md](research/implementation-budget.md) 后开始。下文保留原测量设计；实际执行范围与验收判断见第 7 节。

## 1. 责任边界

- GitHub acquisition/preview：继续固定 commit、资源预算、archive fallback 和不可变 snapshot registry。
- `central_updates` service：继续作为 import/update 内容落盘唯一生产边界，经 `update_skills_batch` 管理 target guard、selected recovery、journal、swap、DB、copy 和 finalize。
- `central_operation`：继续拥有 delete 的 manifest、rename、rollback、fingerprint 与 finalize。
- `installation`：继续拥有 Local copy/symlink 的安装/卸载，leftover 复用 under-guard 入口。
- renderer store/adapter：接收聚合进度，保持 generation/job/target 关联和现有预览状态；组件不直接 invoke。

不预设新的通用文件系统框架、第二套缓存或并行 mutation 入口。已有功能与源码证据见 [research/performance-analysis.md](research/performance-analysis.md)。

## 2. 测量数据流

`固定 fixture -> 临时 snapshot/DB/Central/Platform -> 当前生产 service -> 阶段计时与计数 -> 内容及 journal 核验 -> 受约束的临时根清理`。

生产流和基准必须共用 service，独立脚本重写 copy/delete 的结果只能作为微基准。Rust release/backend 测量与 native WebView 测量分开；没有 app 时 emit 成本为零，不能从该结果推断 IPC 收益。

| 阶段 | 指标 | 对应问题 |
|---|---|---|
| acquisition/parse/verify | 请求数、传输/展开 bytes、ms、snapshot 哈希次数 | 网络/解包与确认成本是否混淆 |
| plan/hash/clone | 遍历数、文件读取数、bytes、ms、工作集 | 重复 SHA/清单/整批 clone |
| lock/recovery | 等待 ms、pending 总数/selected 数、恢复 ms | contention 或恢复被计入 IO |
| stage | mkdir、open/write、完成 files/bytes、ms | 小文件元数据及写入成本 |
| swap/DB/copy | rename、事务、copy files/bytes、ms | 更新投影成本及事务边界 |
| finalize/rollback | hash files/bytes、remove 操作、ms、遗留 artifacts | 清理及失败恢复是否主导耗时 |
| renderer/result | emit/store update 数、主线程长任务、队列排空、ms | 事件放大及实际等待时间 |

统计只使用数量、bytes、ms、phase、target kind 和受控 error codes；不新增全路径、文件内容、凭据或用户机器信息到生产日志。硬件/缓存环境写入本地基准记录，公开资料需脱敏。

## 3. 优先实验

### D1. 真实进度与合并（R3、R5）

从实际 stage 文件写入成功处取得累计 files/bytes。路径校验属于 preparing；不能在准备循环中累计 writing 完成数。数据按现有 phase/DTO/i18n 语义表达，终态等待现有 service settle。

blocking closure 只处理文件系统和轻量累计数据。由 async orchestration 使用有界、聚合的进度桥接并 emit；不把 `AppHandle` 移入 blocking closure，不为每文件发 channel message。优先复用项目既有进度生命周期，只有受影响调用链确实需要时才新增局部接口。

事件预算按基线锁定，候选为时间节流加文件批次门槛；开始、阶段转换、错误、末尾 flush 不合并丢失。基准同时记录 source emit、store update 和 WebView render/队列成本。不能只在前端 throttle 后仍发送 13,016 条 IPC。

有总字节时现有百分比按 bytes 计算，继续保持该含义。写入计数达到总数也不提前宣布整个事务/cleanup 完成；现有 Finalizing/结果语义继续有效。

### D2. 文件内容和清单（R3、R4）

先以计数确认 snapshot -> RemoteSkillFile -> write -> stage -> closure 的复制量和峰值。能移动所有权的分支使用 move；跨 worker 或多个持有者确需共享的内容复用已有只读 Arc 或局部共享表示。选择以测量和实际 owner 为准，禁止给全项目加缓存层。

每个不可变 repository snapshot 可复用一次生成的路径/size/hash 清单来计算各 Skill 内容摘要；保持 repository digest、skill digest、update manifest fingerprint 和 delete fingerprint 的既有 framing，不混用不同摘要。

immutable 字节/清单的复用不自动授权复用磁盘状态。swap/finalize/recovery 面向可被外部修改的路径，必须保留 fresh validation。任何减少完整读取的方案都需证明边界、失败矩阵和历史 manifest 兼容性。

### D3. 目录操作与有界 IO（R3、R5）

测量同 parent 的重复 mkdir；在 operation-owned staging 中缓存已建父目录，仍逐文件安全路径检查，确认顺序及错误行为不变。不能把相同策略应用到所有未知根目录。

串行是基线；按 1/2/4/8 writer 做独立实验。仅当 release/native 结果显示收益且句柄/内存有界时采用局部 bounded worker 方案。并发实验不等于实施决策，不按文件 spawn_blocking，不让几个 Skill 同时绕过 target guard。

### D4. 删除与清理（R1、R4）

分别测 fingerprint、rename、DB cascade、物理 delete、结果返回和重试。已有 rename 暂存沿用。优先评估重复 physical-path 计划、哈希实现与批量调度；保持 marker/fingerprint fresh check 和 Windows symlink 不跟随目标。

pending journal 的 backup/staging/marker 必须由 owning operation 消费。临时根实验清理前检查绝对路径属于该 fixture；真实 Central、Universal Agents、只读插件和其他任务目录不能成为基准 cleanup 目标。

Remote preview ticket 生命周期沿用；Local preview 内存释放需计量内存，无虚构的磁盘 workspace。物理清理失败继续保存现有 pending/recovery 状态，不能只隐藏错误降低 UI 等待时间。

## 4. 后续候选的门槛

| 候选 | 何时重新评估 | 默认边界 |
|---|---|---|
| 增量 staging/copy refresh | 少量变化场景仍由完整写入/复制主导 | 保留 atomic full-directory swap；必须说明未变文件如何安全构造；不默认 hardlink 或原地写 canonical |
| 持久化内容索引 | 操作内复用不足，且能完整检测外部变化 | 需要迁移/失效合同和新的批准；当前不添加 schema |
| 后台 backup/delete cleanup | 物理清理主导，用户接受更改完成时间语义 | 需要 backlog、quota、锁、关机/崩溃恢复和 UI pending 设计；当前同步收尾 |
| 跨 Skill import batching | 多 Skill 的 guard/DB overhead 已测为显著 | 保留请求顺序、partial/fail-fast、token retry 与 per-item journal，不因单 Skill 案例扩大范围 |

## 5. Fixture 与正确性矩阵

| Fixture | 形状 | 目的 |
|---|---|---|
| S | 128 files，含有效 SKILL.md | 小型正常操作回归 |
| M | 4,096 files，多级目录 | 文件数扩展曲线 |
| T | 13,016 files，含 SKILL.md；资源多数约 1 KiB | 对应截图规模，文件内容固定 |
| H | 18,000 files，目录总条目仍低于 copy 20,000 上限 | 预算内压力 |
| L | 约 32 个较大文件，总 bytes 与 T 相近且每文件预算内 | 区分 files 与 bytes 成本 |
| X | 文本/二进制混合、空文件、Unicode/深目录及预算边界 | 内容、路径与资源约束 |

T 的 13,016 为总文件数，包含 SKILL.md；实际 manifest 大小及生成字节数必须写入样本，不能用约 1 KiB 代替实测 bytes。

每组覆盖首次/overwrite、check + 无变化 apply、改 1 文件/1% 文件/全部文件、远端新增/移除文件及本地修改冲突、Central 删除、0/1/多 copy、symlink、重复 leftover。注入 stage、swap、DB、copy、finalize 失败及 restart recovery；错误和终态 flush 属于进度断言。

性能计时只跑成功路径；失败路径单独验证正确性，不混入性能均值。超预算、越界、只读插件、symlink、保留 copy、未选中 pending、marker/fingerprint mismatch 必须 fail closed 或按现有结果保持数据。

## 6. 验收、兼容与回退

相同源码构建配置和硬件条件比较，至少 7 个配对测量轮次，保留全部样本、中位数、最小/最大和环境噪声；7 次样本不宣称可靠 p95。首次运行与后续重复运行分组，不能声称已清 OS cache。

先保存基线，再为实际选中的改动锁定事件预算、目标阶段和端到端预算。预算记录后才能形成最终实施方案，改动后不得为通过而改预算。收益需超出运行噪声，小型/少量大文件及删除/更新不能有未解释回退。

优先方案不改 schema、journal manifest 版本、持久化摘要格式或 updater。内部文件所有权/进度接口变动应回退到原串行路径且仍读现有数据。回退仅涉及批准的产品 diff；未完成 operation 不删除、不重写，保留原始失败与 retest 证据。

`just ci` 为实施完成门槛；Windows native WebView 单独验收。无打包配置改动时 installer smoke 标明是否执行；若改变 command/schema 则运行 `pnpm docs:gen`，若涉及 bundle 则运行 Windows Tauri build 并确认产物。SSH/WSL fake 不替代实测，未执行一律显式记录。

## 7. 已实施范围与证据

- D1：Local writer 在成功写入后累计 files/bytes，每 256 个文件或 100 ms 发布 retained cumulative value；async importer 每 100 ms 采样，在 service 成功或错误结束时精确 flush。文件写完仍等待 journal/DB/copy/finalize。Remote 单 Skill 计数在 service 成功后增加。
- D2：普通更新、force、mirror、stage/worker 和 Remote archive chunk 移动已有文件所有权。保留 immutable snapshot 到首次 owned files 的复制。candidate digest 先选择子树再计算文件 SHA，旧 framing、root/missing-manifest 语义及 fresh disk validation 保留。
- D3：仅在 operation-owned staging 复用已成功创建的 exact parent。串行写入、逐路径安全检查和首个错误顺序保留；未增加 writer 并发。
- D4：delete fingerprint 借用路径比较排序；update hash 移除冗余排序。全树收集、fresh 内容读取、marker、symlink 非跟随、journal 和同步 finalize 保留。没有实现持久化索引或后台删除。

源码接口只在 Rust service 内部变动；没有 command/DTO/schema、前端 store、依赖、版本或打包配置变更。原 inline delete tests 因文件行数门槛移到 `central_operation/fs/tests.rs`，原 tokens、literals、7 个函数及 1 个 ignore 属性经独立检查一致。

T stage 首 7 轮改善 19.14%，随后单轮控制方向反转。补充 AB/BA 交替 7 对后，中位数下降 6.56%，原定 10% 目标 `NOT_MET`。source 发布下降 99.2472%，数量目标 `PASS`。删除 stage +3.1179 ms/+12.77% 和 L 控制 no-op +9.3939 ms/+30.72% 的原因未查明；不声明稳定删除提速或全生命周期无回退。预算未修改。

S/L 服务各 7 轮、T stage/fingerprint/finalize 交替 7 对、M/H stage 各 1 轮和 T corrected full-service 单轮已执行。T 完整服务单轮只判断正确性，允许与 CI 并行，不用于性能预算。原完整 M/T/H 服务 7 轮、X 扩展 fixture、峰值资源、native WebView、真实网络及 SSH/WSL、安装器均未执行。完整样本、构建身份、首次失败与中断 artifacts 见 [research/benchmark-results.md](research/benchmark-results.md)。

构建时的 after source 清单未覆盖新增 `central_updates/fs/progress.rs` 和拆分的 `central_operation/fs/tests.rs`。冻结 binary SHA、共同 harness SHA 与 tracked diff 已核验；上述两文件只可记录事后 hash，完整 after source 冻结清单为 `UNVERIFIED`。Clippy 定点修复仅改变进度测试的 borrow 词法作用域，最终 gate 身份与原冻结测试身份分别保留。

最终完整 `just ci` 15/15 阶段通过、退出 0，覆盖 Clippy、Rust、前端测试/构建、文档与 Trellis Python。首次失败来自现有 dangling `target` link；进程内 `CARGO_TARGET_DIR` 改用仓库忽略的独立构建根，原链接与系统设置保留。重测发现两处新增测试借用跨 await，缩短词法作用域后完整门槛通过。两次失败、定点修复和最终 raw log 均保存，见 [research/quality-checks/verification.md](research/quality-checks/verification.md)。质量通过不改变性能 `NOT_MET` 或其他未验证状态。

回退按保存的 `after-source.diff` 逐项审核已批准生产 hunks，保留其他人的改动、测试隔离与原始 receipts；不删除或重写数据、pending journal、marker、backup/staging。不执行 reset/stash/clean。回退后须建立新 binary 身份并复测对应范围。当前未执行产品回退。
