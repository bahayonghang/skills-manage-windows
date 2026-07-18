# Design: SkillKit 对标优化路线图

## 1. 父任务边界

父任务不承载代码。它只定义四个子任务共享的不变量、顺序和最终集成门禁。后续应启动拥有下一项交付物的子任务，而不是启动父任务。

## 2. 目标架构

### 2.1 统一来源意图

```text
Central “添加技能”
  -> Source intent router
     -> GitHub intent -> 现有 GitHubRepoImportWizard
     -> ZIP intent    -> 新 LocalArchiveImportWizard
     -> Deep-link intent (后续) -> GitHub intent + 预填 URL
```

统一的是入口和 intent，不合并两个复杂 wizard 的内部状态机，也不产生 modal 套 modal。GitHub 继续由 `marketplaceStore.githubImportSlice.ts` 管理状态；ZIP 使用独立 Zustand slice 和 Tauri command。

### 2.2 GitHub acquisition 分层

```text
URL/PAT/mirror resolution
  -> try tree manifest acquisition
     -> discover paths using shared source rules
     -> fetch only plugin manifests + candidate SKILL.md
     -> preview uses path/size manifest
     -> import downloads union(selected subtrees)
  -> typed fallback before mutation
     -> existing archive snapshot path
  -> existing candidate/preview/staging/atomic Central persistence
```

快路径只替换“如何取得仓库清单和选中文件”，不重写候选身份、preview DTO、selection、冲突、pluginName、sourcePath、Central 写入或远程 workspace 语义。

### 2.3 深链

```text
skillport://import?source=<https GitHub URL>
  -> native parser + allowlist + length limit
  -> cold-start queue / running-instance forward
  -> frontend import intent event
  -> navigate Central + open unified entry prefilled
  -> user Preview -> Confirm -> Import
```

深链是意图载体，不是命令执行协议。

### 2.4 排版治理

```text
inventory arbitrary sizes
  -> classify semantic role and essentiality
  -> define semantic type tokens + font-scale behavior
  -> migrate by surface
  -> contrast/overflow/keyboard/theme checks
  -> comprehensive no-growth guard for production text-[...]
```

不以统一放大字号为目标；正确目标是同一信息角色使用同一 token，并确保必要信息在实际主题与缩放下满足可读性和 AA。生产 TS/TSX 不保留按文件、行号、数值或单位维护的 arbitrary text allowlist；deliberate display 值也必须落入有明确组件所有者的命名 utility。

## 3. 跨层契约

- Frontend 组件不直接 `invoke()`；所有新 command 经 Zustand store/typed IPC adapter。
- GitHub preview 的 `pluginName` 与 `files` 保持 preview-only；不得进入 import selection、result、数据库或 source metadata。
- `sourcePath = "."` 始终表示完整根仓库技能包；嵌套 source 只映射精确子树，继续复用 `repo_file_relative_to_source`。
- 所有 Central 写入先完成下载、校验和 staging，再获得中央 mutation guard 并原子替换；回退不得发生在部分写入之后。
- ZIP 和 deep-link 新依赖不是规划默认授权。实施启动前分别确认 `zip`、`tauri-plugin-deep-link` / `tauri-plugin-single-instance` 的版本与许可。
- SSH/WSL GitHub 路径在快路径子任务中保持现有 workspace/archive 方案；本地 ZIP MVP 在远程目标下显示明确不可用状态，不静默写入本机 Central。

## 4. 兼容与迁移

- 无数据库 schema 迁移作为默认方案。
- GitHub DTO 和现有 IPC command 名保持稳定；新增性能字段只允许内部日志/测试，不进入用户持久化契约。
- 统一入口可以更换 CTA 文案与打开方式，但现有 GitHub wizard 的 Preview / Confirm / Result 行为和测试语义保持。
- 深链注册只在对应子任务内修改 bundle/capabilities；卸载或回滚后普通 UI 导入仍可用。
- 排版 token 先添加、后分区迁移；每一批可独立回滚，不删除仍被使用的旧 utility。

## 5. 度量策略

- 后端 acquisition 测试使用可控 mock HTTP，记录请求数、传输字节、选中文件数、fallback reason 和 elapsed；网络实测只作补充，不替代确定性测试。
- GitHub 目标仓库至少覆盖：根 skill、一个小子 skill、多 skill 大仓库、私有/PAT、限流、tree truncated、镜像失败。
- 排版覆盖 6 主题、代表性 accent、三档 font scale、900x600 最小窗口，以及 Central / GitHub wizard / Usage / Settings 关键表面。
- 每个子任务最小门禁先跑定向 Vitest/Rust tests，再跑 `just ci`；深链额外跑完整 Windows bundle。

## 6. 执行协调

- Codex inline 模式一次只启动一个子任务，推荐顺序为 unified import → manifest fast path → typography → deep-link。
- `CentralSkillsShell.tsx` 同时属于 unified import 的 CTA/launcher 改造和 typography 的 Central migration slice；typography 必须在 unified import 归档后基于最新文件重跑 inventory，再迁移该表面。
- fast path 与 typography 虽无产品依赖，也不得借“可并行”在同一共享工作树中交叉实施；父任务只记录调度，不把这种协作约束伪装为功能依赖。

## 7. 风险与回滚

| 风险 | 控制 | 回滚 |
| --- | --- | --- |
| 快路径与 archive 候选语义漂移 | 共用纯路径/manifest/frontmatter 规则 + parity fixtures | 关闭快路径，继续 archive |
| raw 小文件请求放大 | 文件数/字节/并发阈值 + 根 skill 直接 archive | 调低阈值或全量 archive |
| ZIP Slip / zip bomb | 预览期拒绝危险条目和预算超限，写入前全部校验 | 禁用 ZIP 入口，不影响 GitHub |
| 深链绕过确认 | 事件只打开预填 UI，不携带 resolution/targets | 移除 scheme/plugin，UI 入口保留 |
| 字号迁移破坏密度 | task-start inventory、语义分类、分区截图、最小窗、三档缩放与全面 no-growth guard | 按 surface 回滚 token 映射，但不重新引入 arbitrary class |

## 8. 明确拒绝

不在本路线图中引入账号、云分享、Recent installs、固定卡片海、横向 agent chip 主导航、同步扫描、整页 key 重挂载或 SkillKit 视觉复刻。
