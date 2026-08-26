# Skills CLI 页面骨架与紧凑卡片网格 — 执行计划

## 启动前硬门禁

- [ ] `08-26-backend-contract` 已完成并合入 `dev`，placement-aware DTO 与 generated IPC 类型可用。
- [ ] 父任务树复审通过，用户在最新规划摘要之后明确批准实施。
- [ ] `implement.jsonl` / `check.jsonl` 为真实上下文，`task.py validate` 通过。
- [ ] 只以父 `research/design-contract.md` 与本任务 artifacts 为规范；不依赖缺失 README、
      `support.js` 或静态 HTML 事件。

未满足任一项时保持 `planning`，不得运行 `task.py start`。

## 有序实施步骤

1. **先锁定纯视图与宽度状态测试**
   - 新建 `src/test/lib/skillsCliViewModel.test.ts`，用 managed_link/direct_copy/missing/conflict/unavailable 混合 fixture
     覆盖四计数、搜索、chip、Unlinked only、叠加过滤、四分组和稳定桶 id。
   - 测试 719/720、899/900、1179/1180 边界；实现同源 layout/drawer band 常量和纯函数。

2. **建立页面级 surface/content-width controller**
   - 定义 `SkillsCliActiveSurface` 和 openInstall/openDetail/openUpdate/openUninstall/closeSurface；测试普通详情
     focus=null、links focus、close reset 与卸载 payload。
   - 用页面内容根的 `ResizeObserver` 更新 contentWidth；CSS container 仍是网格布局权威。
   - 页面根只在无 active surface 且事件未 prevented 时处理冒泡 Escape 清 selection；不得注册 window handler。

3. **先落安装 mount seam 与统一 toast helper**
   - 新建 `SkillsCliInstallMount.tsx`，冻结 open/change/returnFocus/contentWidth props；初始
     `available=false` + null render，并把 Header Install disabled 接到 available，确保没有 no-op。
   - 新建 `skillsCliActionToast.tsx`，集中 stable id、2800ms 与四种 reviewed icon/tone；API 只收 string message。
   - 单测 mount 的 disabled/透传/关闭复位，并锁 toast id、duration、replacement 和 semantic icons。
   - 把 adapter/helper 作为后续 install/batch 的唯一消费 seam；它们不得改 `SkillsCliView`/Header/controller。

4. **修复 canonical add mutation seam**
   - 先扩展 `src/test/stores/skillsCliStore.test.ts`：success 只有 add invoke、返回 result、无 loadAll IPC；
     selection/busy/backend failure reject；当前/stale job 状态不串写。
   - 将现有 `addGlobal` 改为 `Promise<SkillsCliAddResult>` 的 mutation-only action：成功返回，失败写
     `actionError` 后 rethrow；删除 action 内 `loadAll()`，不新增同义 store action。
   - 改当前页面 controller 为嵌套两段式：add 成功先保留/报告 success，再独立 `await loadAll()` 并检查
     `inventoryError`；refresh failure 只发 follow-up warning，不落入 add catch、不重试 mutation。
   - 为 mutation failure、success+inventory-refresh-failure、success+runtime-only-failure 添加页面测试；
     后续 install surface 必须消费同一 seam。

5. **以类型测试驱动 skillsCli dense-row**
   - 在 `SkillsCliSkillCardProps` 增加必需 `layout: "denseRow"` 与 placement/checkbox/detail/link props；
     其他场景不获得这些字段。
   - 先更新 `unifiedSkillCardVariants.test.tsx` 正例/负例，再经 `toModel` 单点实现。
   - 用 76px min-height + auto growth，补三行、4 图标、`+n`、状态、keyboard、focus-within、propagation；
     跑全量 UnifiedSkillCard 测试防其余场景回归。

6. **实现 Header**
   - 新建组件与测试，覆盖成功/失败/refreshing、四计数、Refresh/Install disabled、安全错误、热区和焦点。

7. **实现 Toolbar 的纯呈现边界**
   - 新建组件与测试，覆盖搜索/clear、group、platform pressed、Unlinked only、Select。
   - Export all 只接 `onExportAll?: () => void`；undefined/导出中 disabled，注入时调用一次且无
     filtered/selected payload。本任务不新增导出 store、文件对话框或 serializer。

8. **实现 GroupHeader**
   - 覆盖 sticky、`aria-expanded/controls`、稳定 id、update badge、Select all/Update all 的
     disabled/undefined 行为，禁止 no-op handler。

9. **重排 `SkillsCliView.tsx`**
   - 保留 Local gate 和 store action；页面不直接 invoke。
   - 组合 Header + Toolbar + 命名 container 网格 + 页尾；接入视图态、surface state 与纯函数。
   - 保持 runtime/inventory/action error 分离；实现 loading、inventory empty、filtered empty、
     stale+error、refreshing 五态。
   - 移除 `InventoryCensus`；后续子任务只走明确 callback/controlled surface 边界。

10. **实现 exact container contract**
   - 内容根加 `@container/skills-cli`；网格 2 列基线、900px 三列、1180px 四列，Toolbar 换行且无横滚。
   - contract test 锁类名；production build 后检查两个 `@container` rule。不得改用 viewport 断点。

11. **迁移 `InventoryCensus` 到 Local Dashboard**
   - 组件内部不改；Dashboard Local-only 区块按需调用 skillsCliStore.loadAll，不触碰 central summary。
   - 测 Local 渲染、非 Local 不加载/不渲染、loader failure 不污染 central summary；
     Skills CLI 测试断言 census 已移除。

12. **补 i18n、token 与集成回归**
    - en/zh 成对加入标题、计数、状态、group/filter/empty/aria/placement 文案。
    - 静态测试禁止原型 hex、CDN/远程资源和硬编码显示字体。
    - 保留 preview/add/uninstall 既有回归；记录 web/component 与 native evidence 的区别，未在 Windows
      WebView2 实测的断点、中文排版、焦点环、hover 和视觉保真标为 `UNVERIFIED`。

## Focused checks

```powershell
pnpm vitest run src/test/lib/skillsCliViewModel.test.ts src/test/pages/SkillsCliView.test.tsx src/test/pages/DashboardView.test.tsx
pnpm vitest run src/test/stores/skillsCliStore.test.ts
pnpm vitest run src/test/components/skillsCli src/test/components/skill/unifiedSkillCardVariants.test.tsx src/test/components/skill/UnifiedSkillCard.test.tsx
pnpm typecheck
pnpm lint
pnpm build
```

构建后检查 `dist/assets/*.css` 同时存在 900px 与 1180px 的 `@container` 规则；若未生成，修正 utility，
不能用 viewport breakpoints 掩盖失败。

## Completion gate

```powershell
python .trellis/scripts/task.py validate .trellis/tasks/08-26-page-shell
git diff --check -- .trellis/tasks/08-26-page-shell src/pages src/components/skillsCli src/components/skill src/i18n
just ci
```

## 风险与回滚

- `UnifiedSkillCard` 为共享组件：只改 skillsCli 判别成员、`toModel` 与 dense-row；其他场景差异阻断完成。
- 后续子任务依赖本任务的 surface/content-width/install-mount/toast/addGlobal seam；不得重写页壳、
  复制 toast 参数或新增第二个 add wrapper action。
- Dashboard census 的移除与挂载同提交，避免丢失或双渲染。
- 回滚同时撤销页面壳、surface controller、dense-row 类型和 Dashboard census，不留下孤立 i18n。
