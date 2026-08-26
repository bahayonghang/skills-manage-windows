# Skills CLI 技能详情抽屉 — 技术设计

共享契约见 `../08-26-skills-cli-redesign/research/design-contract.md`。本任务按顺序依赖
`backend-contract`、`page-shell` 和 `batch-actions`，不允许并行抢占 shared store/surface 所有权。

## 1. 组件与受控 Surface

新增 `src/components/skillsCli/SkillsCliDetailDrawer.tsx`：

```ts
interface SkillsCliDetailDrawerProps {
  skill: SkillsCliGlobalSkill | null;
  targets: SkillsCliInstallTarget[];
  contentWidth: number;
  docState: SkillsCliDocState;
  updateAvailable: boolean;
  focusSection: "links" | null;
  busyPlacements: ReadonlySet<string>;
  onClose: () => void;
  onToggleLink: (agentId: string, next: boolean) => void;
  onLinkAll: () => void;
  onUnlinkAll: () => void;
  onRetryDoc: () => void;
  onUpdate?: () => void;
  onRevealFolder: () => void;
  onUninstall: () => void;
}
```

外壳复用 `OperationLogDetailDrawer` 的 Base UI Dialog primitive，但保持 controlled open。
`contentWidth < 720` 时面板宽为当前 Skills CLI content width；否则固定 460px。content width 来自
`page-shell` 的 ResizeObserver/context，不用 viewport `md` breakpoint 猜测。overlay、panel、title、description
和 close trigger 使用现有主题 token 与 Base UI focus restore。

## 2. Header 与元信息

- canonical path 视觉省略但 `title`/accessible description 保留完整值。
- source、hash、time 任一为空就不创建 pill；hash tooltip 明确为 local content hash，不称作 commit。
- 相对时间复用现有 helper，测试固定 clock，避免 locale/timing flake。
- Update badge 只表达 store update state；按钮还要求 `onUpdate` 已接入。

## 3. Placement Rows

从 backend inventory 的 target-ordered placements 直接派生 row model：

| placement | associated | switch | action |
| --- | --- | --- | --- |
| `managed_link` | yes | checked/enabled | unlink |
| `missing` | no | unchecked/enabled | link |
| `direct_copy` | yes | checked/disabled | 显示 retained copy 说明 |
| `conflict` | no | disabled | 显示 conflict reason |
| `unavailable` | no | disabled | 显示 unavailable reason |

摘要 `n` 是 associated 数，`m` 是 enabled target 数。若存在 missing，显示 Link all 并只提交 missing；
否则若存在 managed links，显示 Unlink all 并只提交 managed links；只有 direct copies/blocked placement 时
按钮 disabled 并给原因。详情只复用 `batch-actions` 已建立的 `linkPlatform`、`unlinkPlatform` 与 batch helper，
不得在本任务改写其 mutation/rollback 逻辑。

## 4. Doc State 与竞态

本任务在同一 `skillsCliStore` 追加 doc read state，但不改 shared mutation actions：

```ts
type SkillsCliDocState =
  | { status: "idle" }
  | { status: "loading"; skillName: string; requestId: string }
  | { status: "ready"; skillName: string; content: string; byteSize: number }
  | { status: "empty"; skillName: string; byteSize: 0 }
  | { status: "error"; skillName: string; errorCode: string };

readSkillDoc(skillName: string): Promise<void>;
clearSkillDoc(skillName?: string): void;
```

每次打开生成 request id；响应只有在 request id + skillName 仍匹配时提交，避免快速切换技能后的 stale result。
关闭时清当前 doc/error；重开总是重新读取，不做跨会话 cache。组件用 `<pre>` 展示原始文本，保留 frontmatter；
loading、0-byte empty、error/retry 是互斥状态。bounded read、UTF-8 与 size limit 由 backend-contract 保证。

## 5. Reveal Folder

store 追加：

```ts
revealSkillFolder(skillName: string): Promise<void>;
```

它只调用 typed `skills_cli_reveal_skill_folder({ skillName })`。renderer 不读取 `skill.path` 并传给通用
`open_in_file_manager`；backend 依据 Local lock ownership 解析 canonical/owned folder、检查存在性后打开。
non-Local、not-owned、missing、OS launch failure 都保留稳定 error code；调用方在 drawer 内联显示并 toast。

## 6. Surface Coordinator 与 focusSection

页面使用 page-shell 的：

```ts
openDetail(skillName, { focusSection: null | "links", returnFocusTo });
openUninstall(skillNames, { returnFocusTo });
closeSurface();
```

- 卡片普通点击始终显式传 `focusSection: null`；Manage Links 传 `"links"`。
- drawer 打开且 focus 为 links 时，等待 named drawer + links heading 后 `scrollIntoView`；完成后消费该 intent，
  coordinator 中 focus 重置为 null，避免重渲染重复滚动。
- close/switch skill 都把 focus 设 null；旧技能 doc state 同时清理。
- Uninstall 不叠加第二个 Dialog：原子把 active surface 从 detail 切到 uninstall，并调用 batch-actions
  的 `openUninstall`。因此没有 placeholder callback，也没有两套 dialog state。
- Base UI 是 Escape topmost owner；本任务不新增 window/document listener。

## 7. Error、Busy、Offline 与可访问性

- 每行 mutation 有独立 busy key，其余安全行可操作；Link all/Unlink all 期间相关行和 aggregate action disabled。
- 新提交清旧 inline mutation error；失败用 `formatBackendError`，toast 只补充，不暴露路径/details。
- 所有 doc/reveal/link 反馈只调用 page-shell 的 `skillsCliActionToast` 并选择 operation semantic；
  本任务不直接调用 `sonner`，不复制稳定 id、duration 或 icon helper。
- doc/reveal failure 留在 drawer 内，可 retry；关闭清 local error。
- Local capability 不可用时 placement/reveal/update actions disabled，仍可阅读 cached metadata/doc error state。
- switches 有包含平台名与状态的 label；disabled placement 以文本 + icon 解释，不只靠颜色。
- 小图标热区 40px、focus-visible、标题初始焦点和关闭后的 trigger restore 纳入组件测试。

## 8. Tests

- row-model pure tests：五种 placement、summary、Link all/Unlink all partition、stable target order。
- drawer component：header/pills、loading/empty/error/ready doc、720 boundary props、disabled/busy、Update visibility、Reveal/Uninstall callbacks。
- store：doc success/error/empty、stale request ignored、clear；reveal typed args/error；确认 link actions 来自 batch owner。
- page integration：普通/Manage Links focus reset、scroll once、card/drawer shared snapshot、detail→uninstall surface switch、focus restore。
- async surface tests先等待具名 dialog，再 `within(drawer)`；局部 5000ms budget，不增加全局 timeout。

## 9. Rollback

本任务仅回滚 detail component、doc/reveal store 增量、card detail entry 和 surface wiring。
共享 link/unlink/remove/export actions 与 uninstall dialog 由前置 `batch-actions` 所有，回滚 detail 不得删除。
若需继续回滚 batch，必须先完成本任务回滚，依赖顺序与提交顺序一致。
