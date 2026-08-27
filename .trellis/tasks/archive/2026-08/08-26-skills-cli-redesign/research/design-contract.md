# Skills CLI 重设计持久化设计契约

## 目的与证据边界

本文件是 `08-26-skills-cli-redesign` 任务树的任务内设计依据。原规划引用的
`design_handoff_skills_cli/README.md` 与 `support.js` 不在仓库中，因此不能作为实施或验收证据。
`research/skills-cli-redesign.dc.html` 只保留为静态构图参考；其中的 no-op、CDN、模板事件和尺寸不自动成为产品契约。

实现验收以父/子任务的 PRD、design 以及本文件为准。若未来恢复原始 Claude Design 交付物，只能作为差异输入；任何会改变范围、交互或 AC 的差异都必须先回到 planning。

## 八个界面区域

| 区域 | 必须呈现 | 所有者 |
| --- | --- | --- |
| 1. 页头 | 标题、installed/linked/unlinked/repositories 四个派生计数、运行时状态、刷新、Install skills | `page-shell` |
| 2. 工具栏 | 搜索、Repository/Platform/Status/None 分组、单选平台过滤、Unlinked only、Select、Export all | `page-shell` |
| 3. 分组网格 | 吸顶可折叠组头、组内计数、更新状态、Select all、Update all、紧凑卡片、空态、页尾所有权说明 | `page-shell`，更新入口由 `update-center` 接线 |
| 4. 批量操作栏 | 选中数、Link to platform、Unlink、Export selected、Uninstall、清除选择 | `batch-actions` |
| 5. 安装弹窗 | Source → Skills → Platforms 三步、最近源、真实 preview、已安装标记、命令预览 | `install-wizard` |
| 6. 详情抽屉 | 名称、更新状态、canonical/source/revision/time、平台 placement、SKILL.md、Update/Reveal/Uninstall | `detail-drawer` |
| 7. 更新抽屉 | 按来源分组、选择、installed → upstream、变更摘要、本地修改/基线/失败状态、Update selected | `update-center` |
| 8. 卸载确认与反馈 | 单个/批量共用确认、受管 canonical/link 计数、copy/conflict 阻断说明、命令预览、危险确认、单实例 toast | `batch-actions` |

## 卡片与响应式契约

- 仍由 `UnifiedSkillCard` 唯一渲染；为 `variant: "skillsCli"` 增加显式 dense 布局，不新建第二套卡片。
- `font_scale=1` 时默认卡片目标高度 76px，允许因 1.125 字号偏好按 token 增长，但不得裁切标题、路径或焦点环。
- 内容容器宽度 `>=1180px` 为四列，`900–1179px` 为三列，`<900px` 为两列。断点按内容容器而非整个窗口判断。
- 工具栏在 chip 溢出前换行；不能依靠水平页面滚动维持布局。
- 抽屉内容宽度 `>=720px` 时为 460px，`<720px` 时占满内容宽度。不要用 Tailwind 默认 `md=768px` 代替 720px。
- 本地 HTML 不能独立运行，集成验收对照任务内契约和真实应用截图，不把 HTML 事件执行结果列为通过证据。

## Placement 状态表

| 状态 | 含义 | 计入平台关联 | Link/Unlink |
| --- | --- | --- | --- |
| `managed_link` | Windows junction 或受管 symlink，目标解析到 owned canonical | 是；计入 linked | 可安全 unlink |
| `direct_copy` | 平台目录为普通目录，来源可能是 CLI 或安装 fallback | 是；不计入 linked | 禁用，显示 copy 说明；本任务不自动转换 |
| `missing` | 平台目录不存在 | 否；计入 unlinked | 可 link，Windows 创建 junction |
| `conflict` | 路径存在但不能证明由当前 canonical 拥有 | 否 | 禁用；绝不覆盖或递归删除 |
| `unavailable` | 目标平台禁用、缺目录或不支持本地 placement | 否 | 禁用并显示原因 |

`Unlinked only` 只筛选至少一个 enabled target 为 `missing` 的技能，不把 `direct_copy` 或 `conflict` 冒充未链接。

## 更新状态表

| 状态 | 用户语义 | 允许动作 |
| --- | --- | --- |
| `not_checked` | 尚未联网检查，只有缓存/本地库存 | Check updates |
| `checking` | 正在按去重来源检查 | Cancel（若后端已建立可取消 job） |
| `current` | 已有 installed baseline，且与上游一致 | 无 |
| `update_available` | 上游身份不同，本地内容未修改 | Update |
| `local_modified` | 当前内容 hash 与 installed baseline hash 不同 | 带覆盖警告的 Update |
| `baseline_required` | 普通新装或旧记录缺少 installed baseline，不能诚实判断 | Verify current files / Reinstall；不得显示“无更新” |
| `unsupported` | 非受支持来源 | 显示原因，无更新动作 |
| `rate_limited` | GitHub 额度不足 | 显示 reset/retry 信息 |
| `failed` | 仓库或技能检查失败 | 显示安全错误与 Retry |

更新提示必须在重启、重复检查且尚未 apply 时保持稳定。`installed baseline` 与 `last observed upstream` 是两个独立字段，不能用“上次检查 SHA”代替已安装身份。普通 install-wizard 不掌握 pinned upstream identity，因此不写 baseline；只有 update-center 的 Verify exact-match 或成功 Apply/Reinstall 可以建立它。

## 交互矩阵

| 触发 | 成功 | 失败/取消 |
| --- | --- | --- |
| Refresh inventory | 列表与派生计数刷新 | 保留 stale inventory，展示 inventory error |
| Install source preview | 成功后进入 Skills 步骤 | 留在 Source，内联错误 + toast |
| Link missing placement | 卡片、详情与计数同步 | 乐观状态回滚；copy/conflict 不调用后端 |
| Export all | 保存版本化全量 snapshot | 用户取消不报错；不可写路径显示本地化错误 |
| Export selected | 保存版本化 selection snapshot | 同上；零选择禁用 |
| Open detail | 详情 focus 默认为 null | Manage Links 才使用 links focus；关闭时复位 |
| Check updates | 缓存并显示所有状态 | 失败仓库可见，stale 成功结果仍保留 |
| Apply updates | 更新 canonical/lock/baseline 后刷新 placement | 可恢复 journal 保留；中断后不得半更新冒充成功 |
| Uninstall | 删除 CLI owned canonical/lock/managed links | direct-copy/conflict 进入阻断列表，不自动删除 |

## Esc 与焦点

- Dialog/Menu/Drawer 继续使用 Base UI 的 topmost dismissal；不得再注册第二个无条件全局 Escape handler。
- 页面选择态只在没有任何 Base UI 浮层处理本次 Escape 时清除。
- 打开浮层后焦点进入标题或第一个可操作控件，关闭后返回触发器。
- 多层关闭顺序为卸载确认 → 安装弹窗 → 更新抽屉 → 详情抽屉 → link menu → 清除选择。

## Toast

- 所有 Skills CLI 操作使用一个稳定 toast id；新 toast 替换旧 toast。
- `duration=2800ms`；破坏性成功/失败使用危险语义图标与现有 status token。
- 同一失败同时需要当前表面的内联错误时，toast 只作补充，不能替代可重试错误面。

## Token 与资源

- 页面、卡片、边框、文本和状态统一使用现有主题 token 与 `statusTone.ts`，不复制原型 hex。
- 图标使用 `lucide-react` 与 `PlatformIcon`；不加载 CDN、远程字体或图片。
- 显示字体使用 `displayFont.ts` 偏好；等宽文本使用仓库已打包字体。
- 所有可见字符串成对进入 `src/i18n/en.json` 与 `src/i18n/zh.json`。

## 仍需实证的表面

- Windows 本机 junction 创建、识别、权限失败与安全删除。
- `skills@1.5.23` 的真实 `--force` / `--keep-links` 能力；未保存帮助输出前不得进入 argv。
- Windows installer/WebView2 下的断点、焦点、Escape 层级、中文排版与视觉保真。
- GitHub PAT、共享 rate-limit、网络失败和真实仓库数据。
