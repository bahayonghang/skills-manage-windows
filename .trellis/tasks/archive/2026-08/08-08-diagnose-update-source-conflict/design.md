# 诊断设计

## 边界

本任务沿 Update Center 的同一生产路径建立反馈环，不改用户真实数据。优先复用现有
Rust inventory fixture；若现有测试无法覆盖“失败项增量重查后 resource.conflict”，
则用临时 SQLite/文件系统 fixture 组合生产 service，诊断结束后删除临时产物。

## 证据链

1. 前端：失败项如何形成 `SkillRefreshScope` 与 incremental mode override。
2. IPC：re-check command 的参数、lease/并发门禁及 `resource.conflict` 映射来源。
3. Service：regular refresh 对 missing source 的 relocation 规则；incremental refresh
   对 keep/delete decision 和 inventory run 的写入规则。
4. Persistence：只读验证 repository、membership、source path、inventory run/entry、
   pending addition 与 update state 是否满足冲突前提。
5. History：定位 missing-source、auto-relocation、retry 功能进入当前分支的提交，并以
   测试或差分证明回归边界。

## 假设检验方法

在最小复现建立后列出 3–5 个可证伪假设。每次只改变一个变量：source path 唯一性、
inventory run 状态、refresh scope/mode、repository membership 或当前任务 lease。根据
输出淘汰假设，不以错误文案本身代替因果证据。

## 安全约束

- 真实 `~/.skillsmanage/db.sqlite` 仅做只读连接；优先复制/immutable URI，禁止 migration。
- 不输出 PAT、完整 URL、绝对用户数据路径、技能内容或未脱敏日志。
- 网络探针如有必要仅访问公开 repository 元数据，不携带用户凭据。
- 诊断阶段不执行 Update Center apply、删除、重建 inventory、修复 provenance 或安装。

## 交付形态

最终报告给出：主根因、触发条件、此前正常的原因、立即恢复路径、永久代码修复、
回归测试与尚未验证项。若用户随后授权实现，再单独扩展任务范围并按仓库门禁执行。
