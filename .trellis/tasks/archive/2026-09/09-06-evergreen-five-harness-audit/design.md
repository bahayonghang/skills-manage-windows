# 常青项目五harness整改设计

## Architecture and Ownership
保留现有React/Zustand/typed IPC、Tauri services/repositories、SecretStore及CI lane结构。本轮设计集中于开发工作流，不重写产品核心。

R1/R2：audit-report与research记录当前源码、tests、远程run及临时probe。历史已修复与当前缺陷分开。共享active_task修ownership；doctor修只读probe；bootstrap恢复来源与Python测试覆盖；Central child仅恢复有用户意义的skip。

R3/R4：AGENTS是稳定项目事实源，CLAUDE只import；五工具的发现/注入/权限/模型分层描述。Kimi只读research经explore返回主线程持久化；OMP model未证不自创别名。主线程/强模型承担批准、风险判断和独立验收，便宜模型执行固定文件/断言。

R5：最终rules child拥有全部项目指南与Kimi skill内容写回，其他child交证据；不写Basic Memory或global规则。bootstrap child仅拥有其ignore例外、hooks和CI脚本；doctor拥有自身script/test；session拥有Python解析/消费者与新isolation tests；Central仅拥有其页面test。

## Sources and Dependencies
- 保留两个定制inject hooks于版本控制；其他接线复用已安装pinned Trellis init + --skip-existing，不加模板框架。
- Python suite接入现有rust-platform lane，使三host各跑一次；不添加第三方Action依赖或独立lane。
- doctor无副作用验证不等于pin可用；标准CI仍要求pnpm10.34.5。缺少时明确BLOCKED，安装在现有授权外。
- session/doctor/bootstrap/Central可独立工作，已有runtime tests运行时不得并发增删tasks；rules等待四项结果。父统一最后CI，避免每文档改动重复全库。

## Validation and Scope
每项AC由child机制/命令支撑，父通过同一基线/diff整合证据。本轮planning-only只验证规划结构与独立review；批准后实施阶段运行just ci及必要专项，不把readonly替代测试称作canonical PASS。Windows安装、真实provider/trust、未跑remote属于外部证据，不以static/fixtures冒充。

## Tradeoff and Rollback
最小完整修复优先于统一配置平台：没有模型路由器、规则DSL、compat shim、全部agent目录入库、用户全局改写或REL重开。各child可独立回退所拥有diff；任何真实session、cache、凭据与用户设置不参与rollback。
