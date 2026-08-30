# Skills CLI 三步安装弹窗 — 执行计划

## Hard prerequisites

- [ ] `08-26-backend-contract` 已完成并合入 `dev`：recent-source policy、generated DTO/error/argv 契约可用。
- [ ] `08-26-page-shell` 已完成并合入 `dev`：install surface mount、open/close/focus、shared toast helper，以及
  canonical `addGlobal(input): Promise<SkillsCliAddResult>` mutation-only/no-inline-`loadAll` 契约已有真实测试。
- [ ] page-shell 已隔离/移除旧页尾 install `<details>`，本任务不需要重写 `SkillsCliView` 页面壳。
- [ ] 父任务树最新规划已由用户另行批准；本 task 仍为 `planning`、`base_branch=dev`。

本任务与 batch 是 sibling，但只写独立 install modules。若实际实现发现必须修改 canonical
`skillsCliStore`、page coordinator 或 batch action，立即停止并返回 planning，不用“顺手接线”扩大所有权。

## Ordered Steps

1. **锁定 view-model 红灯测试**
   - 新建生产文件 `skillsCliInstallViewModel.ts` 与 `src/test/lib/skillsCliInstallViewModel.test.ts`：session+request correlation、normalized preview、默认未安装选择、
     platform join、stable dedupe 和命令 token/quoting。
   - 命令 preview 必须含重复 `-s`/`-a`、`-g`/`-y`，明确断言不存在 `--force`/`--keep-links`/comma list。

2. **实现 recent-source 独立 store**
   - 新建生产文件 `skillsCliRecentSourcesStore.ts` 与 `src/test/stores/skillsCliRecentSourcesStore.test.ts`，只通过 `@/lib/ipc` 的 named commands 读写 exact key。
   - 覆盖 valid roundtrip、latest-first、dedupe、8 项、invalid persisted JSON fail closed、load/push failure。
   - 不修改 canonical `skillsCliStore` 或把辅助错误写进 `actionError`。

3. **先写 dialog 状态机交互测试**
   - open reset、三态 stepper、manual/recent 共用 await-preview、preview single-flight/pending 不换步、
     close/reopen stale settle。
   - skills default/select all/clear、platform default/shared grid、Back、空选择、pending duplicate submit/dismiss。
   - 测试先等待 dialog，再用 `within(dialog)`；局部 async timeout 5000ms，不提高全局 timeout。

4. **实现 dialog 与共享平台选择**
   - 新建 `SkillsCliInstallDialog.tsx`，接近 400 行即拆 steps 文件。
   - 复用 `usePlatformTargetSelection`/`PlatformMultiSelectGrid`；adapter 只把 selected platform id 映射到
     backend `skillportAgentIds` 和 preview `cliAgent`，不复制平台列表/网格。
   - Base UI controlled dismissal；install pending 拒绝关闭且不注册 global Escape。

5. **实现 page-shell install surface adapter**
   - 填充 page-shell 预留的 `SkillsCliInstallSurface` mount，只组合既有 preview/add/load actions、recent store、
     reviewed formatter、surface close/focus 和 shared toast helper。
   - null/error mapping不暴露 raw rejection；runtime/doctor fail 保持入口 disabled。
   - 不改 `SkillsCliView` coordinator，不新增第二套 active surface。

6. **分离 success 与 follow-up failure**
   - 主 install success 后 close + success toast；库存 refresh 与 recent push 分开 settle。
   - 分别注入 refresh/recent failure，断言不重开 dialog、不把 install 改判失败、不重复提交。

7. **i18n、a11y 与 responsive**
   - en/zh 成对加入 title/subtitle/step/source/recent/skills/platform/footer/loading/error/aria/warning 文案。
   - 40px icon hit area、visible focus、trigger focus return、keyboard flow、窄宽度 wrap/internal scroll。
   - 不引远程资源、原型 hex、新 dependency 或 shadcn Sheet。

8. **Focused checks**

   ```powershell
   pnpm vitest run src/test/lib/skillsCliInstallViewModel.test.ts src/test/components/skillsCli/SkillsCliInstallDialog.test.tsx src/test/components/skillsCli/SkillsCliInstallSurface.test.tsx src/test/stores/skillsCliRecentSourcesStore.test.ts
   pnpm typecheck
   pnpm lint
   pnpm build
   ```

   对 recent preview、late settle、Base UI pending dismiss 相关测试重复运行，确认无时序 flake。

9. **Repository gate**

   ```powershell
   just ci
   git diff --check
   python .trellis/scripts/task.py validate .trellis/tasks/08-26-install-wizard
   ```

10. **Native evidence**
    - Windows Tauri/WebView2 检查 open/focus return、pending Escape/backdrop、中文长文案、两列到窄宽度、
      spinner/disabled 与 recent error。
    - 未执行时逐项标 `UNVERIFIED`；jsdom/build 不替代 native visual/focus evidence。

## Risk and rollback

- 最大风险是 stale preview 推进错误 session、主 action/follow-up failure 混淆、以及 sibling 偷改 shared owner。
- 回滚只移除 install dialog/surface/view-model/recent store 和其 i18n keys；保留 backend/page-shell contracts。
- 若 page-shell mount 或 shared toast helper缺失/漂移，停止实施并回到 planning，不能在本任务重建页面壳。

## `task.py start` Gate

- [ ] PRD/design/implement 与 JSONL manifests 通过 precheck 和独立复审。
- [ ] backend/page-shell 实际签名已回填并与 generated types 一致。
- [ ] 用户在最新 planning summary 后明确批准实施。
