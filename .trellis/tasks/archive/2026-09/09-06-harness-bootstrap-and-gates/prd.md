# 补齐 harness 接线来源和持续测试覆盖

## Goal
P1。让新检出具备可重放的 Trellis 接线来源，避免本机有 hooks、CI 没 hooks仍显绿；父要求 R1/R2/R3/R4。

## Evidence
`.gitignore:11` 起忽略平台目录；HEAD无五工具生成接线。`.trellis/scripts/tests/test_runtime_resilience.py:304` 检查 hooks_present，缺失会skip；`scripts/check/run-ci.mjs:33` 的现有lane没有Python tests。当前完整clone行为尚未实跑，这是源码确认的覆盖缺口。
本机 Trellis 和 tracked .trellis/.version 同为0.7.0-beta.3；`trellis init --help` 确认五平台flags与 --skip-existing。

## Requirements
- R1：已定制的两个 inject-subagent-context.py 必须有受版本控制的来源，不依赖开发者本机隐藏目录。
- R2：使用现有 pinned Trellis生成其余平台接线，保护项目定制，不创建新的模板引擎或拷贝六套全量规则。
- R3：Python安全/运行时/会话回归进入已有跨平台CI入口，必要hook缺失必须显式失败而非skip通过。
- R4：新检出可按说明完成五工具静态发现/context smoke；运行时trust/provider未具备则明确UNVERIFIED，不伪称五工具端到端通过。

## Acceptance Criteria
- [ ] AC1（R1）：交付diff包含两个非私有inject hook及精确ignore例外；新检出的hook字节与受控源一致。
- [ ] AC2（R2）：隔离Git检出中用已安装0.7.0-beta.3初始化五平台，--skip-existing不覆盖受控hook/项目规范；必要agent/skill/extension入口可发现，父工作区不变。
- [ ] AC3（R3）：删除fixture必要hook时回归非零退出；正常fixture通过；runCi/ciWorkflowContract断言Python回归接入现有rust-platform lane、失败传播且每主机只跑一次。
- [ ] AC4（R4）：Windows本机执行Python套件成功；Linux/macOS真实runner未跑则分别缺失证据；五工具列静态发现、hook启用、会话注入、provider结果四层，不合并为单一PASS。

## Out of Scope / Approval
planning。不追踪凭据、个人trust/settings.local、runtime/session、cache、全局配置；不更新全局Trellis、不启用本机hooks、不运行provider请求、不新建CI lane。文档由rules child统一拥有。
