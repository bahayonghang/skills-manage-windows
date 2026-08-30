# Skills CLI 三步安装弹窗 — 技术设计

共享视觉/交互契约见 `../08-26-skills-cli-redesign/research/design-contract.md`。

## 1. Module ownership and mount seam

本任务只实现以下独立模块：

- `src/components/skillsCli/SkillsCliInstallDialog.tsx`：纯受控三步 UI 与本地 session state。
- `src/components/skillsCli/SkillsCliInstallSurface.tsx`：page-shell mount adapter，组合 canonical
  `skillsCliStore`、recent-source store、surface close/focus 与 shared toast helper。
- `src/pages/skillsCliInstallViewModel.ts`：preview/session reducer、platform adapter 与命令预览纯函数。
- `src/stores/skillsCliRecentSourcesStore.ts`：generic settings IPC 的唯一 recent-source invoke owner。

`src/components/skillsCli/SkillsCliInstallMount.tsx` 是 page-shell 预先创建的 handoff adapter；本任务只把
其内部 `available=false/null render` 替换为对 `SkillsCliInstallSurface` 的接线，并保持公开 props 不变。
该 adapter 是本任务获准修改的唯一 page-shell 文件，不构成 `SkillsCliView` 或 coordinator 所有权。

`page-shell` 前置必须提供可测试的 install mount seam：`activeSurface.kind === "install"` 时只渲染
`SkillsCliInstallSurface`，传入 `open`、`onRequestClose` 与 trigger ref；关闭统一走 `closeSurface`。
本任务不重写 `SkillsCliView.tsx`、surface coordinator 或 batch 的 canonical store actions。旧页尾 install
`<details>` 必须已被 page-shell 隔离到该 seam，本任务以 dialog 替换，不保留双入口。

shared `skillsCliActionToast` helper 由 page-shell 前置提供稳定 id、2800ms 与 reviewed icon/tone；install
只调用，不复制常量。若 mount/toast seam 与已批准签名不同，先回 planning 更新本文件。

## 2. Dialog session state machine

用 reducer 而不是四个松散 state 表达一个 open session：

```ts
type InstallStep = "source" | "skills" | "platforms";

interface InstallDialogSession {
  sessionId: number;
  step: InstallStep;
  sourceInput: string;
  resolvedPreview: SkillsCliSourcePreview | null;
  selectedSkillNames: Set<string>;
  selectedPlatformIds: Set<string>;
  submitError: string | null;
  pendingPreviewId: number | null;
}
```

`open: false → true` 递增 `sessionId` 并完全初始化；close 清 error/preview/selection。preview 是 single-flight：
`isPreviewing` 时手工按钮与 recent pills 都 disabled，事件 handler 也以 pending guard 拒绝重复调用。每次
preview 递增 request id，capture `{sessionId, requestId}`。settle 只有两者仍匹配且 dialog 仍 open 才可写状态：

- `previewStarted(source)`：清旧 error、保持 step=source、记录 trimmed source。
- `previewSucceeded(result)`：把 backend normalized `result.source` 作为权威，初始化未安装 skills，进入 skills。
- `previewFailed(message)`：保持 source，清 resolved preview，停在 source 并写安全 error。

手工按钮和 recent pill 只调用同一个 `awaitPreviewAndAdvance(source)`。recent click 只先更新 input/pending；
绝不在 await 前 dispatch step 变化。Back 从 platforms→skills、skills→source，不复用旧失败 error。

## 3. Presentational dialog boundary

```ts
interface SkillsCliInstallDialogProps {
  open: boolean;
  onOpenChange(open: boolean): void;
  canonicalRoot: string | null;
  npmSpec: string;
  targets: SkillsCliInstallTarget[];
  platformTargets: PlatformTarget[];
  installedNames: ReadonlySet<string>;
  platformInstalledCounts: Readonly<Record<string, number>>;
  recentSources: readonly string[];
  isRecentSourcesLoading: boolean;
  isPreviewing: boolean;
  isMutating: boolean;
  onPreview(source: string): Promise<SkillsCliSourcePreview>;
  onInstall(input: {
    source: string;
    skillNames: string[];
    skillportAgentIds: string[];
  }): Promise<SkillsCliAddResult>;
}
```

Surface adapter把 canonical store 的 nullable actions转换为 resolve/reject：null 时读取当次 `actionError`，经
`formatBackendError` 变成 reviewed UI error；component 不接收 raw rejection/path/details。

Dialog content 可同文件拆三个小组件；生产文件接近 400 行即拆为
`SkillsCliInstallDialog.steps.tsx`，不等 800 行 gate 才处理。

## 4. Skills and platform selection

Skills step 在 preview success 时默认 `preview.skills - installedNames`。Set 按 preview 顺序输出；
Select all/Clear all 只操作当前 preview。全部已安装时默认空并禁用 Continue，用户仍可 Select all 明确重装，
但本任务不自动添加 `--force`。

Platforms step 将 `SkillsCliInstallTarget` 以 id join 到 `PlatformTarget`，使用
`usePlatformTargetSelection` + `PlatformMultiSelectGrid`。默认 predicate 查对应 CLI target 的
`defaultSelected`；display/icon/keyboard 由共享 grid 提供。提交时不使用该 hook 的
`selectedInstallAgentIds()` 作为 CLI id，而按 selected platform id 回查 `SkillsCliInstallTarget`：

```ts
skillportAgentIds = targets
  .filter((target) => selectedPlatformIds.has(target.id))
  .map((target) => target.id);

cliAgentsForPreview = targets
  .filter((target) => selectedPlatformIds.has(target.id))
  .map((target) => target.cliAgent)
  .filter(uniqueStable);
```

平台已安装数从 inventory placement 中按 `managed_link | direct_copy` 求和；不从 display name/path 推断。

## 5. Deterministic command preview

`skillsCliInstallViewModel.ts`：

```ts
buildInstallCommandPreview({
  npmSpec,
  source,
  skillNames,
  cliAgents,
}): string;
```

先生成 display-only token array，顺序与 `build_add_global_argv` 一致：

```text
npx <npmSpec> add <source> -s <skill> [-s <skill>...] -g -a <cliAgent> [-a ...] -y
```

纯 quoting helper 只用于安全、可复制展示，不作为执行 argv。重复值稳定去重，输出次序沿 preview/target
权威顺序；任何选择为空返回空 preview/disabled state。`--force`、`--keep-links`、`--agent a,b` 不出现。

## 6. Recent-source store

`skillsCliRecentSourcesStore` 是独立小型 Zustand store：

```ts
interface SkillsCliRecentSourcesState {
  sources: string[];
  isLoading: boolean;
  error: unknown | null;
  loaded: boolean;
  load(): Promise<void>;
  push(source: string): Promise<void>;
  reset(): void;
}
```

- 只从 `@/lib/ipc` 调 `get_setting`/`set_setting`，key 固定 `skills_cli.recent_sources`。
- local parser 也 fail closed：必须是 0–8 个合法 string；非法 persisted value 得到 `[]` 与稳定辅助错误，
  不渲染/preview 原值。后端 settings policy 仍是安全权威。
- `push` 使用 normalized successful preview source，latest-first exact dedupe 后截断 8；只有 backend set 成功
  才更新 store，避免 UI 声称已持久化。
- load/push 不写 canonical `skillsCliStore.actionError`，避免辅助 preference 失败污染安装主 action。

## 7. Surface orchestration and result separation

`SkillsCliInstallSurface` 订阅 canonical store 的 doctor/targets/skills、previewSource/addGlobal/loadAll 和
busy/error；其中 `addGlobal(input): Promise<SkillsCliAddResult>` 必须是 page-shell 前置任务冻结的 mutation-only
action，内部不调用 `loadAll()`。surface 只在 open 时触发 recent `load()`。流程：

1. preview wrapper await `previewSource`，null → reviewed rejection；success → 返回本次 DTO。
2. install wrapper await canonical `addGlobal`；mutation reject → reviewed error，dialog 保持 open。
3. install success → 记录逻辑 summary，调用 page-shell close，发 shared success toast。
4. close 后分别启动 `loadAll()` 与 recent `push(source)`，各自独立 catch/feedback；refresh failure 依赖
   canonical inventoryError 并发辅助 toast，recent failure 只发 recent warning。两者都不能进入 install catch。

普通 install flow 不调用 update baseline IPC，也不直接写 update DB。它没有 pinned upstream identity 的证明，
因此 refresh 后仍由 update-center 把无基线技能呈现为 `baseline_required`；只有 Verify exact-match 或成功
Apply/Reinstall 可以建立 installed baseline。

在 install pending 时 `onOpenChange(false)` 忽略 backdrop/Escape/close request；取消不在本任务范围，因为
现有 add job 暴露 cancel command但本产品弹窗没有批准 Cancel UI。promise settle 后恢复 Base UI dismiss。

## 8. Accessibility, Escape and responsive behavior

- 使用现有 Dialog root/title/description；打开聚焦 source input，步骤变化聚焦 heading/第一控件，关闭由
  page-shell trigger ref 恢复焦点。
- Base UI 是唯一 topmost Escape owner；不添加 window/document listener。pending install 的 controlled
  onOpenChange 拒绝关闭，不绕过 focus trap。
- close/back icons 可见尺寸不足 40px 时使用对称 `after:size-10`，同行中心距 ≥40px；所有 focus ring 可见。
- content 用 `minmax(0,1fr)`、wrap 与内部 bounded scroll；窄宽度不制造页面水平滚动。

## 9. Tests and rollback

- View model：session/request stale protection、default selection、platform join、stable dedupe/ordering、完整 argv。
- Dialog：AC2–AC10，尤其 preview single-flight、recent pending 不换步、close/reopen late settle、pending
  dismiss、Base UI focus。
- Surface：runtime gate、nullable store action mapping、install/main vs refresh/recent 三段 failure、stable toast helper。
- Recent store：named IPC mocks、roundtrip/dedupe/truncate/invalid JSON/load/push failure。
- Platform shared grid：复用其既有测试并增加 wizard adapter integration；不复制 grid assertions。
- i18n/production build/native evidence按 PRD AC14–AC15。

回滚只移除本任务四个独立模块和 install i18n keys，并恢复 page-shell install slot 的未实现状态；不回滚
page-shell coordinator、backend contract或 batch store actions。若 legacy `<details>` 已由 page-shell 删除，
回滚本任务不能把未审阅旧 UI 私自加回。
