# Skills CLI 技能详情抽屉 — 执行计划

状态保持 `planning`。实施顺序固定：`backend-contract` → `page-shell` → `batch-actions` → `detail-drawer`。

## Ordered Steps

1. **核对硬前置**
   - backend：placement fields、bounded doc IPC、name-only reveal IPC 与 generated command map 已落地。
   - page-shell：controlled surface coordinator、content-width signal、卡片 detail/manage-links callbacks 与
     唯一 `skillsCliActionToast` helper 已落地。
   - batch-actions：canonical link/unlink actions、`openUninstall` 与共用 dialog 已落地。
   - 任一接口缺失即返回 planning；不写 placeholder、不复制 shared action。

2. **先写 row/doc/focus 纯函数测试**
   - 五种 placement 到 row model、associated summary、Link all/Unlink all safe partition。
   - doc state stale-response guard、focus intent consume/reset、719/720 width mode。

3. **追加 doc/reveal store actions**
   - `readSkillDoc`/`clearSkillDoc` 使用 skillName + requestId correlation；覆盖 ready/empty/error/stale/close。
   - `revealSkillFolder` 只传 skillName，按命令名 mock；不复用 renderer path-based open command。
   - 不改 batch-actions 所有的 link/unlink/batch/remove/export action 实现。

4. **实现受控 drawer**
   - Base UI shell、header、optional pills、placement rows、doc `<pre>`、底部 actions。
   - 用 page-shell content width 精确处理 `<720` full / `>=720` 460；补 loading/empty/error/offline/disabled states。
   - 小图标热区、Switch label、focus-visible、initial/return focus 与文本化状态原因。

5. **接入 placement actions**
   - 复用 batch store actions；missing-only link、managed-link-only unlink；direct copy/conflict/unavailable 禁用。
   - 单项/aggregate busy 与 inline safe error；抽屉和卡片继续读同一 store snapshot。

6. **接入卡片与 surface coordinator**
   - 普通入口显式 focus null；Manage Links focus links、scroll once 后消费 intent。
   - close/switch 重置 focus/doc/error；Update 仅在 state + callback 同时具备时渲染。
   - Reveal 调 name-only IPC；Uninstall 原子 detail→shared uninstall，不叠加/占位。

7. **i18n 与文案检查**
   - en/zh 成对覆盖 placement reasons、doc states、metadata tooltip、Reveal errors 和 actions。
   - hash 文案明确 local content hash；不把它称为 commit。

8. **定向验证**

   ```powershell
   pnpm vitest run src/test/components/skillsCli src/test/components/skill src/test/pages/SkillsCliView.test.tsx src/test/stores/skillsCliStore.test.ts
   pnpm typecheck
   pnpm lint
   ```

   对 drawer 异步交互重复运行，先等待具名 dialog 并在 surface 内查询。

9. **全量门禁与原生检查**

   ```powershell
   just ci
   ```

   在 Windows Tauri/WebView2 检查 719/720 内容宽度、动画、遮罩、初始/返回焦点、逐次 Escape、
   Reveal folder、junction/copy/conflict rows、中文长路径与 SKILL.md 大文本滚动。未执行前均记录 `UNVERIFIED`。

## Risk and Rollback

- 最大风险是跨任务 shared action/surface drift、doc race 和 viewport/content-width 混淆。
- 开始前把实际 generated types 与 plan 签名逐项回填；禁止用 `as` 或 optional placeholder 绕过漂移。
- 回滚本任务不触碰 batch actions/uninstall surface；若进一步回滚 batch，先确认 detail 已回滚。

## task.py start 前检查

- [ ] `backend-contract`、`page-shell`、`batch-actions` 已按顺序完成并合入 `dev`。
- [ ] `skills_cli_reveal_skill_folder(skillName)`、bounded doc 与 placement generated types 可用。
- [ ] surface coordinator/content-width/store action 的实际签名已与本计划对齐。
- [ ] `skillsCliActionToast` 只由 page-shell 定义，detail 只消费且没有直接 `sonner` 调用。
- [ ] `implement.jsonl`、`check.jsonl` 无 `_example`，且所有路径存在。
- [ ] 最新规划总结已呈现，用户在后续消息明确批准实施。
