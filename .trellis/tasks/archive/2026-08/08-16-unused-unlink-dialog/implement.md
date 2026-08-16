# 执行计划:未使用技能 unlink 弹窗化

前置:阅读 `.trellis/spec/frontend/index.md`、`skill-usage-state.md`、`icon-control-hit-area.md`,
再读本任务 `prd.md` / `design.md`。全程仅动前端与前端测试。

## 步骤

1. **夹具先行**:`src/fixtures/usage.ts` 补充
   - 多 Agent(≥3,含 1 个 shared-root `linkType: "native"`、1 个 `hasPendingRecovery`)的 Central 条目;
   - 跨 ≥2 Agent、含 1 个 `isReadOnly` / 1 个 `sourceKind !== "user"` / 1 个 `rowId: null` 的平台散件条目。
2. **归一纯函数**:在 `src/components/usage/` 下实现 `centralTargets` / `platformTargets`
   (`UnlinkTarget` 模型 + 禁用原因判定),从 `UnusedSkillsPanel.tsx` 迁移
   `platformUnlinkDisabledReason` 逻辑;先配直接单测(可并入面板测试文件)。
3. **Store 批量方法**:`src/stores/usageStore.ts` 新增 `unlinkUnusedSkillFromAgents`
   (顺序执行 + `unlinkActionKey` pending 生命周期 + 批后一次 `refreshUnused()` + 逐项结果返回);
   全局 grep `unlinkUnusedSkill` 调用方,无残留则删除旧方法。
4. **新弹窗组件**:`src/components/usage/UnusedSkillUnlinkDialog.tsx`
   (Dialog + Checkbox,全选/单选/禁用原因/确认计数/部分失败呈现,见 design.md 布局与 testid)。
5. **面板改造**:`src/components/usage/UnusedSkillsPanel.tsx`
   - 删 `CentralAgentChip` / `PlatformUnlinkAction` / `preferredPlatformInstall`;
   - Central 第二排改 `entryAgentIds(entry).join(" · ")` 文本行;
   - 操作列最右加 Unlink 触发按钮(全禁用时禁用 + title),`dialogEntry` state 挂弹窗。
6. **绑定与透传**:`src/pages/skillUsageBindings.ts` 换 `onUnlinkAgents`;`src/pages/SkillUsageView.tsx`
   同步 prop。
7. **i18n**:en/zh 增 `skillUsage.unused.unlink.dialog.*`;grep 确认后删
   `unlink.actionLabel` / `unlink.confirm`;禁用原因 key 保留。
8. **测试重写**:
   - `src/test/components/usage/UnusedSkillsPanel.test.tsx`:移除 `unlink-chip-*` / `unlink-action-*`
     用例,新增触发器/弹窗/全选/禁用/确认/部分失败用例;
   - `src/test/stores/usageStore.test.ts`:批量方法成功、部分失败、异常、pending 生命周期、
     refresh 恰一次。
9. **文档门**:本任务不动 Tauri 命令与 `src-tauri/src/db/schema/`,预期无需 `pnpm docs:gen`;
   若实现中意外触及,补跑并纳入提交。

## 验证命令(迭代期)

```bash
pnpm test -- --run src/test/components/usage/UnusedSkillsPanel.test.tsx
pnpm test -- --run src/test/stores/usageStore.test.ts
pnpm typecheck && pnpm lint
```

## 完成门

```bash
just ci
```

## 审查点 / 回滚

- 步骤 5 完成后先目检(或截图)确认:行右单入口、行下无任何确认按钮,再进入测试重写。
- 回滚单元 = 整个任务单 commit,`git revert` 即可;无数据/契约迁移。

## 收尾

- Phase 3.3:更新 `.trellis/spec/frontend/skill-usage-state.md` 中 unlink 契约
  (行内两段式 → 弹窗批量语义、`unlinkUnusedSkillFromAgents` 契约、testid 约定)。
- Phase 3.4:按仓库提交风格单 commit(参考 779aa340 的 `feat(usage):` 前缀与中英文混排体例)。
