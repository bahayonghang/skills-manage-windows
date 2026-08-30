# PIN 1.5.23 copy 模式与库存归属

日期：2026-08-24  
任务：`.trellis/tasks/08-24-skills-cli-inventory-ux`  
用途：冻结 TPR-01 的上游与本仓库证据，以及库存算法必须遵守的归属规则。

## 上游：单目标目录默认 copy，且 copy 不写 canonical

PIN `skills@1.5.23`（`SKILLS_CLI_NPM_SPEC`，`src-tauri/src/services/skills_cli/argv.rs:12`）。

安装模式选择（`src/add.ts` tag `v1.5.23`）：

- `installMode` 初值：`options.copy ? 'copy' : 'symlink'`。
- `uniqueDirs.size > 1` 且非 `--copy`、非 `-y` 时才提示 symlink/copy。
- **`uniqueDirs.size <= 1` 时强制 `installMode = 'copy'`**（单目标目录「不需要 symlink」）。
- SkillPort 写路径带 skills 层 `-y`（`argv.rs` add builder）。因此：用户只勾一个非 Universal 平台时，PIN **不会提示、直接 copy**。多平台且目录不同时，`-y` 跳过提示，保持默认 `symlink`。

copy 安装路径（`src/installer.ts` tag `v1.5.23`）：

```ts
// For copy mode, skip canonical directory and copy directly to agent location
if (installMode === 'copy') {
  await cleanAndCreateDirectory(agentDir);
  await copyDirectory(skill.path, agentDir, agentType);
  return { success: true, path: agentDir, mode: 'copy' };
}
```

成功的全局安装仍写入 lock v3（`src/add.ts` 调用 `addSkillToLock`；schema `src/skill-lock.ts`：`version`、`skills: Record<name, { source, sourceType, sourceUrl, ... }>`）。

官方 `skills ls` 会扫 canonical **以及** 各 agent 目录（`installer.ts` list 段：canonical + each installed agent's directory）。本任务 list **不 spawn CLI**，因此必须在 Rust 里复现「lock 名字 + canonical **或** agent 目录副本」这一并集，而不是只认 canonical/junction。

## 本仓库：现有 origin 分类明确把 copy 标成 Other

`src-tauri/src/services/skills_cli/lock.rs`：

- `CliLockOwnership` 只保留名字（`owned_names`，`:20-23`）。
- `classify_local_path_origin`（`:167-189`）仅当路径在 lock 拥有的 canonical 内，或 symlink/junction 解析进 canonical 时返回 `LinkOrigin::SkillsCli`。
- 单测 `tests.rs:469-472`：平台下的 **copy 目录** 断言为 `LinkOrigin::Other`。

Leftover Local 保护走同一分类（`src-tauri/src/services/central_updates/inventory/scan.rs:92-101`）。因此 copy 模式的合法 CLI 安装：

1. lock 有名字；
2. 往往 **没有** `~/.agents/skills/<name>`；
3. 文件在 `{agent.global_skills_dir}/<name>`；
4. origin = Other → 若库存只扫 `SkillsCli` 链接，则 KPI「至少链接一平台」为 0；leftover 仍可能把该副本当可删项。

这不是假设，是 PIN 默认行为 + 当前分类测试的合取。

## 本任务必须采用的归属算法

成员资格（有无这条技能）：**仅 lock v3 的 skill 名**。目录碰巧叫同一名字但 lock 无记录 → 不是本页库存（可能是 leftover / 手工副本）。

每条技能的 `path`（展示用，单值）：

1. 若 `universal_skills_dir/<name>` 是目录 → 用该 canonical（symlink 模式或 Universal-only copy 写入 canonical）。
2. 否则若恰好一个已检测且已映射平台上存在 `{global_skills_dir}/<name>` 目录 → 用该副本路径。
3. 否则若多个平台有副本 → `path` 取稳定排序后的第一条副本路径，卡片仍列出全部 `agents`。
4. 否则 `path = null`（lock 有名、磁盘无货）。仍列出，UI 标明缺失，**不得**当成空库存。

`agents`（平台归属，驱动 KPI 与条形图）：

对 `install_targets` 同款集合（detected ∩ mapped）中每个平台，若 `{global_skills_dir}/<name>` 存在为目录，则收录该平台的 `displayName`（及内部 id），**无论** `classify_local_path_origin` 是 `SkillsCli` 还是 `Other`。不扫未映射、未检测平台。不把 Central 根下的路径算作 CLI 平台归属。

`installKind`（只读，供测试与卡片，不新增用户流程）：`canonical` | `copy` | `missing`。canonical 目录存在为 `canonical`；否则有平台副本为 `copy`；否则 `missing`。

`sourceType` 规范化（KPI 去重）：

PIN `SkillLockEntry.sourceType` 注释与写入值为字符串。本页允许桶：

`github` | `gitlab` | `git` | `mintlify` | `huggingface` | `local` | `well-known` | `unknown`

lock 缺字段、空串或不在上表 → `unknown`。KPI「来源种类数」= 这些规范化桶的去重个数（多个未知只计 1 个 `unknown`）。卡片可同时展示 lock 原始 `source` / `sourceUrl`。

## Leftover 一致性（本任务范围内的最小扩展）

Local leftover 在 lock 含 `<name>` 时，除 canonical 与解析进 canonical 的链接外，还须排除 `{mapped_detected_agent.global_skills_dir}/<name>` 目录。远程扫描仍不得用本机 lock。无 lock 条目的同名副本仍可 leftover。本任务不把「整个 Universal 根」或「整个 agent skills 根」加入保护。

## 本任务不强制安装模式

不给 add argv 加 `--copy`，也不试图覆盖 PIN 在 `uniqueDirs.size <= 1` 时的 copy 默认。写路径继续 `-g -y -a … -s …`。读路径必须同时理解 symlink 与 copy。
