# 日志治理与集成设计

## Canonical Ownership

`ipc_registry.rs` 的注册条目与其日志策略元数据共同构成命令级权威来源。核心 observability 模块拥有类型、
策略验证、Operation lifecycle 和 correlation；各 domain child 只声明语义并调用该接口。治理测试从权威来源
计算覆盖，不在文档中复制命令清单或数量。

稳定规则归入 `.trellis/spec/backend/operation-observability.md`，现有 redaction/domain-error/frontend async-error/
quality specs 保持各自职责。`docs/architecture/runtime-observability.md` 只描述 Operation 与 Runtime 两层边界、
关联路径、数据保留/清理和排障入口，并指向具体 symbol；不复制字段表和动态计数。

## Integration Audit

集成检查生成或遍历一张内存覆盖矩阵：command、policy class、owner、category/action、lifecycle、target kind、
correlation support 和 test fixture。编译/契约测试验证唯一性、穷尽性和受控枚举；domain tests 验证执行结果。
嵌套调用以最外层用户意图为 Operation owner，内部服务只追加同一 correlation 的 Runtime 诊断，避免重复历史。

兼容 adapter 是迁移工具而非永久 API。只有六个前置子任务均已迁移且静态搜索无旧入口时，本任务才移除它；
任何仍依赖旧入口的调用都使集成 gate 失败。

## Privacy and Diagnostic Contract

统一对抗种子穿过写入、查询、导出和 UI fixture 四个边界。允许持久化的是 reviewed code/category/phase、稳定
目标类型、时长/计数、retryable、source 与 correlation ID；任意原始错误、参数、路径、URL、凭据、堆栈和
source chain 都必须在构造事件之前消失，而不是只在展示层遮盖。

用户提示由 stable diagnostic 生成：首句说明动作和结果，位置字段说明目标与失败阶段，操作建议提供安全的
重试、检查配置或打开日志入口；correlation ID 可复制。未知错误使用固定诊断码和中性建议，不泄漏底层文本。

## Evidence Model

自动检查证明结构、契约与对抗种子；Windows 原生 smoke 证明弹窗尺寸/居中/键盘焦点、Operation/Runtime 跳转、
清理和 retention 行为。受控异常终止验证 `started -> interrupted` 收口。缺少原生环境、外部 provider 或真实
崩溃条件时明确记为 UNVERIFIED，不能由浏览器 fixture 推断通过。
