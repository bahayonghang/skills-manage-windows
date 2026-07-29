# 整理测试目录并补齐 Rust 集成测试

## Goal

评估并整理 `src/test` 的测试文件组织方式，使测试按稳定、可维护的边界分类；同时识别并补齐 `src-tauri/tests` 的 Rust 集成测试缺口，且不破坏现有测试发现、`just ci` 或 GitHub Actions 质量门禁。

## Background

- 用户反馈 `src/test` 当前文件较多、可发现性较差，希望判断是否应创建子目录分类。
- 仓库为 Windows-first 的 Tauri 应用；测试组织调整必须兼容 PowerShell、本地 Vitest/Cargo 命令和现有 CI。
- `src/test/contracts/ciWorkflowContract.test.ts` 是 `just ci` 与 GitHub Actions 工作流合约的回归锁，目录调整不得削弱或绕过该合约。
- 当前基线为 127 个 Vitest 文件，全部位于 `src/test` 顶层；Vitest 递归配置可支持子目录，但 `pnpm test:serial` 的发现器只扫描顶层。
- Rust 当前约有 914 个模块内测试标记，而 `src-tauri/tests` 仅有 `projects_e2e.rs` 的 5 个外部 crate 集成测试；缺口是跨 crate 契约，不是模块内测试总量。

## Requirements

- 基于现有测试文件、Vitest 配置、测试辅助模块及路径引用，判断 `src/test` 是否需要分层，并给出由仓库职责边界支持的分类规则。
- `src/test` 采用按主要生产代码归属镜像的分类规则；仓库级静态合约、脚本测试和共享测试支持分别进入明确目录，不另造与生产结构竞争的产品域分类体系。
- 目录调整必须保持所有现有测试均被发现并执行，修复受影响的相对导入、脚本和配置，不改变测试语义。
- `pnpm test:serial` 必须递归、确定性地发现嵌套测试，并有聚焦回归测试防止静默漏跑。
- 基于 `src-tauri` 的公共边界、现有单元测试和集成测试，识别真实覆盖缺口并补充聚焦的 `src-tauri/tests` 测试。
- Rust 补测固定为 `cli_api` 外部 crate 契约：覆盖公共身份、dry-run 计划、歧义引用、重复引用及无效参数；不设全模块覆盖率目标，不为追求文件数量复制模块内单元测试。
- 保持 `just ci` 与 GitHub Actions 的检查契约一致。

## Acceptance Criteria

- [x] 形成有证据的 `src/test` 目录现状清单、耦合关系和分类结论。
- [x] `src/test` 按主要源码归属完成分类，`support`、`contracts` 和 `scripts` 等特殊边界有唯一、可复用的归类规则。
- [x] Vitest 与串行回退脚本都能递归发现移动后的测试，发现清单没有意外减少。
- [x] `src-tauri/tests` 的补测范围对应已确认的外部 crate 契约缺口，并包含失败/边界路径。
- [x] `cli_api_e2e.rs` 仅通过 `skillport_lib` 公共 API 测试，不访问网络、系统 keyring 或用户真实目录。
- [x] 新增第二个 Rust 集成测试 crate 后，共享 fixture 不在多个集成测试文件中复制，相关 backend spec 与实际结构一致。
- [x] 聚焦的 Vitest 与 Cargo 测试通过。
- [x] `just ci` 完整通过，且测试数量或测试清单没有因目录变更意外减少。
- [x] 最终差异仅包含经批准的测试组织、测试代码及必要配置/文档调整。

## Out Of Scope

- 改写产品功能或借测试整理进行无关的生产代码重构。
- 仅为了统一风格而移动与分类边界无关的共置测试。
- 为 `src-tauri/tests` 广泛复制已有模块内测试，或为了可测性扩大 `pub(crate)` 生产接口的可见范围。
