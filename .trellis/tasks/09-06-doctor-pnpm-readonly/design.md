# doctor 边界设计

## Ownership and Mechanism
R1/R2 的唯一修改边界是 scripts/check/doctor.mjs 中 pnpm 探测的 subprocess 参数/环境，不建立新配置面或版本适配框架。当前 pnpm 12 的官方文档明确 `pmOnFail` 默认为 download，`pnpm_config_pm_on_fail=ignore` 可在子进程禁用自动版本管理；参见 [pnpm CLI settings](https://pnpm.io/settings/cli#pmonfail)，2026-09-06。该设置不能成为运行生产 CI 时绕开项目 pin 的手段。

实施首先在隔离环境验证该最小 probe 对“实际12错配”和“项目指定10匹配”的行为，再选定一处调用参数；旧 `managePackageManagerVersions` 在12已删除，不能猜配置键或以旧记忆套用。未能证明无下载的调用应明确诊断阻塞，不重试获取包。R3 使用已安装的准确10.34.5运行，不恢复或修改全局状态。

## Files
修改 scripts/check/doctor.mjs、src/test/scripts/doctor.test.ts；父 research 保存实际 CLI 验证结果。docs/agents/build-and-test.md、quality spec 与全局分工说明由 rules child 统一回写，避免同文件并发。

## Test Shape
现有 commandRunner seam验证 child env/参数及结果分类；真实 probe 使用拥有的临时 cache/cwd 与不匹配 manifest，前后快照证明未写缓存。仅检查 mock 收到参数不够。超时 fixture 使用自有子进程；不触发网络安装。

## Tool / Model / Rollback
Codex/Claude Code 强模型判断 probe 与副作用边界、独立复核。便宜模型可按确定合同补 doctor fixtures/整理输出，不能擅自升级工具链。其余三个 harness 可执行同一命令收集结果，无工具专属产品代码。回滚限本 child 两文件。
