# Skills CLI 全局页重设计

## Goal

把 `/skills-cli` 收敛为一个可安全操作、可解释 placement、可恢复更新状态的全局技能管理页：计数与筛选在首屏完成，技能按来源等维度分组，安装、批量、详情、更新和卸载都使用真实后端状态，并把 Windows x64 原生行为作为一等验收面。

父任务只拥有共享需求、子任务映射、依赖顺序和最终集成验收，不直接实现产品代码。

## In Scope

- 页头、工具栏、分组网格、紧凑 `UnifiedSkillCard`、Dashboard 统计迁移和精确响应式布局。
- 三步安装、最近源、批量操作、详情抽屉、更新抽屉和安全卸载确认。
- placement-aware inventory、受管 junction/symlink link/unlink、bounded `SKILL.md` 读取、CLI-safe reveal、版本化导出。
- installed baseline 与 last observed upstream 分离的更新检测、缓存、迁移、apply/recovery。
- i18n、可访问性、错误反馈、IPC/codegen/docs、Rust/React 测试和 Windows 手工证据清单。

## Out of Scope

- 自动把 `direct_copy` 转换为 junction/symlink，或删除不能证明归属的普通目录。
- 保留指向已删除 canonical 的平台 link；`Keep platform link entries` 不进入本次产品。
- 非 GitHub 来源的真实更新检测。
- 恢复缺失的 Claude Design README/support.js，或引入原型 CDN、字体、图片和运行时。
- 仅凭静态测试宣称 Windows installer/WebView2、真实 GitHub 网络或原生 junction 验收通过。

## Confirmed Repository Facts

- 当前页面为 `src/pages/SkillsCliView.tsx`，store 已区分 `runtimeError`、`inventoryError`、`actionError`，后端已有 7 条 Skills CLI IPC。
- 本机 lock v3 当前有 51 条记录、3 个来源；`pluginName` 不是每条记录都存在，不能作为必填契约。
- `~/.claude/skills/ask-matt` 当前是 Windows Junction。现有安装 helper 使用 `symlink_dir`，失败时可能 copy，不能宣称已有 junction 创建路径。
- Inventory 现状会把 direct-copy 目录计入平台关联，因此 linked、copy、missing、conflict 必须分开表达。
- `08-25-skills-cli-inventory-frontend` 已归档 completed；本任务不再执行重复归档。
- 原规划引用的 `design_handoff_skills_cli/README.md` 与 `support.js` 不存在；规范性 UI 契约已持久化为 `research/design-contract.md`，HTML 仅为非规范静态参考。

## Requirements

- R1: 页头显示 `Skills CLI global`、installed/linked/unlinked/repositories 四个由当前 inventory 派生的计数、运行时状态、刷新按钮和 `Install skills`；doctor 失败不清空 stale inventory，但禁用安装。
- R2: 工具栏支持名称/来源/路径大小写无关搜索，Repository/Platform/Status/None 分组，单选平台过滤、`Unlinked only`、Select 和 `Export all`。
- R3: 分组网格提供吸顶可折叠组头、组内计数、更新状态、Select all、Update all、空态和所有权页尾；默认 `font_scale=1` 的 Skills CLI dense 卡目标高度 76px，仍由 `UnifiedSkillCard` 唯一渲染。
- R4: Placement 必须区分 `managed_link`、`direct_copy`、`missing`、`conflict`、`unavailable`；linked 只统计指向 owned canonical 的 junction/symlink，普通目录不被当作 link，也绝不被 link/unlink 覆盖或递归删除。
- R5: 批量模式提供 Link to platform、Unlink、Export selected、Uninstall 和清除选择；preview 分别报告受管 canonical/link、retained direct-copy 和 blocking conflict，零选择禁用动作。
- R6: 安装弹窗为 Source → Skills → Platforms 三步；最近源点击必须完成真实 preview 后才能进入 Skills；已安装项、默认选择和命令预览来自真实 inventory/target/argv 数据；安装 mutation 成功与随后 inventory refresh 失败必须分开报告，不能把已成功安装误报为失败。
- R7: 详情抽屉展示 canonical、来源、revision/time、逐平台 placement、bounded SKILL.md 和 Update/Reveal/Uninstall；普通入口与 Manage Links focus 独立且关闭时复位。
- R8: 更新中心分离 installed baseline、current content hash、last observed upstream 与 pending update，覆盖 `not_checked/checking/current/update_available/local_modified/baseline_required/unsupported/rate_limited/failed`；普通新装和旧记录缺基线时均不得显示“无更新”，普通 install 不写 baseline。
- R9: 更新检查有可到达的 Check/Refresh、loading/cancel/retry 和失败仓库显示；更新 apply 使用恢复 journal 保持 canonical、lock、baseline、placement 一致，未 apply 的更新在重复检查和重启后不得消失。
- R10: Export all 导出当前完整 inventory snapshot，Export selected 只导出选择集；两者使用同一版本化 JSON schema、save dialog、默认文件名、取消和不可写路径反馈。
- R11: 单个与批量卸载共用确认面；完整卸载删除 CLI owned canonical、lock row 和 managed links，保留并披露 independent direct-copy，遇到 conflict 阻断；不提供 Keep links。
- R12: Skills CLI toast 使用稳定 id、2800ms、替换旧 toast和危险语义图标；可见 action 失败同时提供当前表面的本地化内联错误。
- R13: Base UI 拥有 topmost Escape dismissal；页面只在没有浮层处理 Escape 时清除选择，关闭顺序为卸载确认 → 安装弹窗 → 更新抽屉 → 详情抽屉 → link menu → 清除选择，并恢复触发器焦点。
- R14: 内容宽度 `>=1180px` 四列、`900–1179px` 三列、`<900px` 两列；工具栏在溢出前换行；抽屉 `<720px` 全宽，不能用 viewport 或默认 `md=768px` 冒充内容断点。
- R15: `InventoryCensus` 从 `/skills-cli` 移到 `/dashboard`；所有新增文本 en/zh 成对存在，颜色、字体、图标、状态和热区遵守现有 token/spec。
- R16: 后端新增命令必须使用 typed domain errors、path policy、bounded ingestion、exclusive job/target lock、SecretStore 和现有 IPC adapter；IPC 生成物、架构文档与版本化数据库迁移必须同步。

## Acceptance Criteria

- [ ] AC1 (R1,R3,R15): Local 页面首次加载显示页头计数、工具栏和 Repository 分组网格，不再渲染 `InventoryCensus`；Dashboard 渲染同一组件，四个计数都由 inventory 派生。
- [ ] AC2 (R1,R12): doctor 失败时 stale inventory 仍可浏览，运行时 pill 显示本地化错误并禁用 Install；inventory 首次失败与 stale refresh 失败不会被空态吞掉。
- [ ] AC3 (R2,R3): 搜索、四种分组、平台单选、Unlinked only、折叠、Select all 与空结果在组合使用时结果正确且无空桶。
- [ ] AC4 (R3,R14,R15): `UnifiedSkillCard` 的 Skills CLI dense 场景在默认字号达到任务契约的紧凑三行布局；内容容器在 1180/900 精确切换 4/3/2 列，抽屉在 720 精确切换全宽，无裁切或水平页面滚动。
- [ ] AC5 (R4): Rust inventory tests 覆盖 Windows junction、symlink、direct-copy、missing、conflict 和 unavailable；UI 图标、linked/unlinked 计数与允许动作严格按 placement 状态表派生。
- [ ] AC6 (R5,R11,R12): 批量 preview 分别显示 managed links、retained copies、blocking conflicts；确认只删除 owned 对象，direct-copy 保留且 conflict 时不能提交，结果同步列表/选择并使用 2800ms 单实例危险 toast。
- [ ] AC7 (R6): 安装弹窗可前后导航；最近源必须 await 真实 preview，失败留在 Source 并显示安全错误；Skills/Platforms 默认选择和 argv preview 与当前 inventory/targets 一致；mutation success 后 refresh reject 仍报告安装成功并单独提示刷新失败。
- [ ] AC8 (R7,R13): 详情普通入口 focus 为 null，Manage Links 入口只聚焦 links；placement action 同步抽屉与卡片并在失败时回滚；Reveal 只能打开 owned canonical；关闭后 focus 和焦点正确复位。
- [ ] AC9 (R8,R9): fresh cache、已有 baseline、new-install/legacy no-baseline、上游变化、重复检查未 apply、重启、local modification、unsupported、rate limit 和 repository failure 都有稳定状态测试；普通 install 不写 baseline，任何未知/失败都不显示成 current。
- [ ] AC10 (R8,R9,R16): Apply update 成功后 canonical、lock、installed baseline、last observed 和 placement 一致；注入 remove/add/DB/FS 中断后 recovery 可重试或回滚，不留下被 UI 当作成功的半状态。
- [ ] AC11 (R10): Export all 与 Export selected 生成同一版本化 schema，分别覆盖全量和选择集；save dialog 取消无错误，不可写路径显示本地化错误，序列化快照测试稳定。
- [ ] AC12 (R13): 多层真实组件测试与 Windows 手工检查证明一次 Escape 只关闭 topmost 层并按规定顺序退栈；页面没有重复全局 handler，关闭后焦点返回触发器。
- [ ] AC13 (R12,R15): 新增 en/zh key 集合一致，无硬编码可见字面量；toast 时长/稳定 id/替换/危险图标与内联失败反馈有 fake-timer 和组件测试。
- [ ] AC14 (R16): SKILL.md 读取限制 1 MiB，拒绝越界、增长竞争和非法 UTF-8；reveal、link/unlink、export 使用 path policy，错误不泄漏绝对路径或密钥。
- [ ] AC15 (R16): IPC 变更后 `pnpm ipc:codegen` 与 `pnpm docs:gen` 已运行，check 命令无漂移；数据库新库、旧库升级、checksum、future-version 和 rollback fixture 通过。
- [ ] AC16 (R1,R2,R3,R4,R5,R6,R7,R8,R9,R10,R11,R12,R13,R14,R15,R16): `just ci` 通过；Windows installer/WebView2 的 junction、响应式、焦点、Escape、i18n/theme 截图矩阵有人工证据，未执行项明确为 `UNVERIFIED`。

## Child Task Mapping And Dependencies

| 子任务 | 独立交付物 | 依赖 | 覆盖 |
| --- | --- | --- | --- |
| `08-26-backend-contract` | placement-aware inventory、bounded doc、link/unlink、reveal、export writer、recent-source policy | 无 | R4、R10、R16 |
| `08-26-page-shell` | 页头/工具栏/网格/dense card/Dashboard 迁移/响应式、install mount、mutation-only add seam、共享 toast helper | backend-contract | R1-R3、R6、R12、R14-R15 |
| `08-26-install-wizard` | 独立安装 surface/recent-source store/view model、三步 preview/argv | backend-contract、page-shell | R6、R12 |
| `08-26-batch-actions` | selection、批量 link/unlink/export、卸载确认、操作语义 toast 调用 | backend-contract、page-shell；交付顺序在 install-wizard 之后 | R5、R10-R13 |
| `08-26-detail-drawer` | 详情、doc、placement、reveal、调用共享 uninstall | backend-contract、page-shell、batch-actions | R7、R13 |
| `08-26-update-center` | baseline/upstream/cache/migration/check/apply/update drawer | backend-contract、page-shell；集成入口依赖 detail-drawer | R8-R9、R16 |

执行顺序：`backend-contract` → `page-shell` → `install-wizard` → `batch-actions` → `detail-drawer` → `update-center`。install 与 batch 没有产品语义依赖，但都会在各自阶段追加共享 i18n/页面接线，因此按仓库“一次一个任务”规则串行交付，禁止并行写同一工作树。父任务最后执行 AC1–AC16 集成验收，不作为实现任务启动。

## Risks And Deferred Evidence

- `skills@1.5.23` 的 `--force`/`--keep-links`、pinned full-SHA source 与 direct-copy refresh 能力仍未验证；
  backend-contract 必须按 `research/skills-cli-capability-probe.md` 的隔离协议逐项持久化证据，
  `UNVERIFIED/unsupported` 能力不得进入 argv。本计划不依赖 `--keep-links`。
- Windows junction/reparse-point、installer/WebView2 和真实数据库升级必须在实现阶段取得原生证据；静态 Rust/React 测试不能替代。
- GitHub PAT、共享 rate-limit、网络失败和真实仓库数据保持 `UNVERIFIED`；实现必须读取 rate-limit headers 并提供缓存/重试/失败状态。
- 恢复 journal、legacy baseline 和 direct-copy 保留行为是高风险集成面，必须在子任务 focused test 后再进入父任务集成。
