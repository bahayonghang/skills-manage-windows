# 安装引擎

`services::installation` 是把 Central 技能落地到平台目录的唯一写入方。完整流水线分布在五个文件。

## 模块布局

```text
services/installation/
├── types.rs       InstallRequest / InstallResult / InstallMethod
├── fs_util.rs     symlink / copy / 目录遍历共享工具
├── centralize.rs  ensure_centralized：把非 Central 技能拷入 ~/.skillsmanage/skills
├── native.rs      安装到平台 `global_skills_dir`
├── project.rs     安装到项目级技能目录
├── remote.rs      通过 targets::exec 走 SSH 安装
└── batch.rs       一个事务把一个技能安装到多个 agent
```

## 决策树

```text
[install_skill_to_agent] ──┬── skill is_central? ──否──► centralize::ensure_centralized
                           │                              │ 拷入 ~/.skillsmanage/skills
                           │                              ▼
                           │                         更新 skills.canonical_path / is_central
                           │
                           ├── target == Local ──► native | project（按 agent.project_skills_dir）
                           └── target == SSH   ──► remote（走 targets::exec）
```

契约统一：每个入口先确保 Central 源存在，再写 symlink 或 copy，最后把 `link_type` / `symlink_target` 写入 `skill_installations`。

## Symlink vs Copy

| 方法 | 何时使用 |
| --- | --- |
| `symlink` | 默认。单一规范源，更新自动传播到所有安装点。 |
| `copy` | 未开启开发者模式的 Windows；跨文件系统的 SSH 主机；只读位置的 Discover 导入。 |
| `auto` | 优先 symlink，权限错误时回退 copy。Discover 默认。 |

UI 把 `method` 透传到 IPC，每个服务路径在写 DB 前会校验落盘形态与 `installation::types::InstallMethod` 一致。

## 批量安装

`batch.rs` 服务两条 UI 路径：

1. **集合。** `commands::collections::batch_install_collection` 把集合成员逐个安装到选定的 agents。
2. **Central → 多平台。** `commands::linker::batch_install_central_skills` 是 `UnifiedSkillCard` 与 Install 对话框背后的开关。

批量路径在单个 sqlx 事务内执行，每个 (skill, agent) 写一条 `operation_logs`。

## Auto-centralize 不变量

`ensure_centralized` 是幂等的，每个安装入口都调用一次。它保证后续流水线看到 Central 行——即使用户直接从 Discover 结果安装。跳过它会让符号链接清理逻辑失效。

## 卸载

`commands::linker::uninstall_skill_from_agent` 处理：

1. 按 `(skill_id, agent_id)` 查安装行。
2. 解析 `installed_path`；拒绝删除 agent 的 `global_skills_dir` / `project_skills_dir` 之外的文件。
3. 删除 symlink 或 copy。
4. 删除安装行；写一条 `operation_logs`。

路径绑定校验防止异常 DB 行误删 SkillPort 已知边界外的用户数据。

Last reviewed: 2026-05-04
