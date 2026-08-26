# Skills CLI 多选、批量操作、导出与安全卸载 — 执行计划

状态保持 `planning`。前置顺序固定：`backend-contract` → `page-shell` → `install-wizard` → `batch-actions` → `detail-drawer`；install 是共享文件的串行交付前置，不是本任务的产品 API owner。

## Ordered Steps

1. **核对前置契约**
   - 确认 placement enum/fields、link/unlink/remove/export IPC 和错误码已进入 generated command map。
   - 确认 remove 只删除 owned canonical/lock/managed links，保留 direct copies，conflict 拒绝。
   - 确认 page-shell 的 controlled surface coordinator、选择入口、Export all 回调和唯一
     `skillsCliActionToast` helper 已落地并有 contract tests。

2. **先写纯函数与失败用例**
   - selection reconcile、placement mutation partition、removal impact、export v1 envelope/default filename。
   - 覆盖 direct_copy retained、conflict blocked、missing-only link、managed-link-only unlink、稳定顺序。

3. **建立 canonical store actions**
   - 在 `skillsCliStore` 唯一实现 link/unlink/batch/remove/export；按命令名 mock IPC。
   - 批量逐项串行，返回 succeeded/failed/skipped；单项乐观更新、失败回滚、结束 refresh 分开捕获。
   - 这是 detail-drawer 的硬依赖；不得把相同 action 留给后续任务再建。

4. **实现 Export all/selected adapter**
   - 复用 `@tauri-apps/plugin-dialog` save，null cancel 静默；JSON v1、文件名与 trailing newline 固定。
   - 工具栏导出未过滤全量；批量栏导出 selection；不可写/IPC 失败保留 UI 并格式化错误。

5. **实现批量栏和 placement 菜单**
   - `SkillsCliBatchBar`：计数、Link、Unlink、Export selected、Uninstall、Clear、busy/disabled/error states。
   - Menu 用 Base UI controlled root；显示各 placement bucket，不让 direct_copy/conflict 发 mutation。

6. **实现安全卸载对话框**
   - 调用 backend preview command，渲染单/复数标题、名称 chips 与 owned/managed/retained/conflict
     四类结构化影响；不展示或在 renderer 拼接 CLI argv。
   - conflict 非空禁用确认；删除 Keep links UI 和所有重建分支。
   - partial failure 保留 failed names/inline error，成功项从选择集移除并 refresh。

7. **接线 surface、selection 与 Escape fallback**
   - 通过 page-shell coordinator 注册 uninstall；卡片单个卸载和批量卸载共用入口。
   - Base UI 处理 topmost Escape；页面容器只在无 surface/menu 且非文本输入时清 selection。
   - 禁止新增无条件 `window`/`document` Escape handler。

8. **Toast、i18n、a11y 与响应式**
   - 只消费 page-shell `skillsCliActionToast`；为 link/unlink/export/remove 选择正确 semantic，
     不直接调用 `sonner` 或复制稳定 id/duration/icon helper；en/zh parity。
   - 40px 热区、focus return、aria-live/aria-busy、窄宽度换行。

9. **定向验证**

   ```powershell
   pnpm vitest run src/test/components/skillsCli src/test/pages/SkillsCliView.test.tsx src/test/lib/skillsCliViewModel.test.ts src/test/stores/skillsCliStore.test.ts
   pnpm typecheck
   pnpm lint
   ```

   重复运行涉及 dialog/menu 的异步测试，使用局部 5000ms 等待预算，不提高全局 timeout。

10. **全量门禁与人工证据**

    ```powershell
    just ci
    ```

    在 Windows Tauri/WebView2 实机检查 junction/copy/conflict 预览、系统保存对话框、焦点返回、
    连续 Escape、中文长文案与窄宽度；未执行前记录为 `UNVERIFIED`，不得以 jsdom 代替。

## Risk and Rollback

- 风险集中在 destructive placement classification、批量 partial outcomes 与 dialog/menu dismissal。
- 若 backend placement/removal contract 或 page-shell coordinator 未落地，停止实施并返回 planning，
  不以 frontend heuristic/placeholder 补洞。
- 回滚顺序：先回滚依赖它的 `detail-drawer`，再回滚本任务；不回滚 backend/page-shell 前置契约。

## task.py start 前检查

- [ ] `backend-contract` 与 `page-shell` 已完成并合入 `dev`。
- [ ] placement/remove/export/generated IPC、surface coordinator 与 `skillsCliActionToast` 的实际签名已回填并核对。
- [ ] `implement.jsonl`、`check.jsonl` 无 `_example`，且所有路径存在。
- [ ] PRD 已完成 machine-readable R/AC traceability 检查。
- [ ] 最新规划总结已呈现，用户在后续消息明确批准实施。
