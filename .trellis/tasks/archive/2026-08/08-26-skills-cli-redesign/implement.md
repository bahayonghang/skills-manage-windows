# Skills CLI 全局页重设计 — 父任务执行计划

父任务没有产品代码所有权，也不作为实现任务启动。它负责共享契约、依赖顺序、跨 child 集成和最终证据汇总。

## 规划收敛

- [x] 将缺失的外部设计交接替换为任务内 `research/design-contract.md`；静态 HTML 降级为非规范参考。
- [x] 把 placement、direct-copy、legacy baseline、Export、Esc、toast、响应式和安全卸载写入父级 requirements/AC。
- [x] 删除已完成 `08-25` 任务的重复归档步骤。
- [x] 明确父任务不实现、每个 child 的唯一所有权和真实依赖。
- [ ] 树级 precheck、逐 task validate 和独立 plan review 均无 blocking finding。
- [ ] 用户在最新 planning summary 之后明确批准进入实现。

## 实施顺序

```text
08-26-backend-contract
        |
        v
08-26-page-shell
        |
        v
08-26-install-wizard
        |
        v
08-26-batch-actions
        |
        v
08-26-detail-drawer
        |
        v
08-26-update-center
        |
        v
父任务 AC1-AC16 集成验收
```

- `backend-contract` 先稳定 placement、bounded doc、reveal、export 和 recent-source policy。
- `page-shell` 再稳定 content-width、overlay controller、dense card、toolbar/grid 和 Dashboard census。
- `install-wizard` 在 page-shell mount/add/toast seam 后先交付；`batch-actions` 随后交付。两者虽无产品语义依赖，但会分别追加共享 i18n/页面接线，因此不得在同一工作树并行写入。
- `detail-drawer` 复用 batch 拥有的 link/unlink/remove/export store actions 和 uninstall dialog，不允许 placeholder completion。
- `update-center` 最后接入 page/detail surfaces，并独占 migration/check/apply/update drawer。

## 每个 Child 启动前 Gate

- [ ] `task.json.base_branch` 为 `dev`，status 仍为 `planning`，PRD/design/implement 已完成 convergence pass。
- [ ] PRD 使用 `- Rn:` 与 `- [ ] ACn (Rn,...):`，每个用户可见 action 都有 mechanism 和 test owner。
- [ ] `implement.jsonl` 与 `check.jsonl` 均只有真实 repo-relative spec/research entries，无 `_example`、源码路径或待修改文件。
- [ ] 所有 prerequisite child 已完成其 AC 并提供稳定 contract；不能用 tree order、占位 callback 或“谁先落地”代替。
- [ ] 未验证的 CLI flag、Windows junction、native GUI、GitHub real-data 和 migration evidence 被明确标为 `UNVERIFIED`，没有被静态测试替代。
- [ ] 用户已批准最新父级 planning summary；启动拥有下一个实际交付物的 child，而不是父任务。

## 父级集成验收

### 数据与安全

- [ ] 逐条验证父 PRD AC1–AC16，并记录 test/manual/UNVERIFIED 证据。
- [ ] 用 fixture 验证 junction、symlink、direct-copy、missing、conflict、unavailable 从 Rust inventory 到 UI count/action 的单一语义。
- [ ] 验证 link/unlink/reveal/export/doc/update/remove 不接受 renderer 任意路径，不删除普通目录，不泄漏 PAT、命令或绝对路径 details。
- [ ] 注入安装、卸载和 update apply 中断，验证 refresh/recovery 后 canonical、lock、baseline、placement 一致。

### 交互与视觉

- [ ] 组合验证搜索、分组、平台过滤、Unlinked only、selection 和空态。
- [ ] 同时打开多层真实组件，验证一次 Escape 只关闭 topmost 层，顺序与焦点恢复符合 AC12。
- [ ] 以内容容器宽度验证 1180/900/720 精确断点；默认字号 dense card、1.125 字号增长、中文/英文、light/dark 无裁切。
- [ ] 验证 Export all/selected、save cancel/write failure、install recent preview failure、update failure/retry 和卸载 conflict 的可见反馈。
- [ ] Windows installer/WebView2 的 junction、响应式、焦点、theme/i18n 截图矩阵未实际执行时保持 `UNVERIFIED`。

### 生成物与质量

按 affected surface 先跑 focused checks，最终运行：

```powershell
pnpm ipc:codegen
pnpm docs:gen
pnpm ipc:codegen:check
pnpm docs:gen:check
pnpm docs:build
pnpm vitest run src/test/pages/SkillsCliView.test.tsx src/test/lib/skillsCliViewModel.test.ts src/test/components/skillsCli src/test/stores/skillsCliStore.test.ts
cargo test --manifest-path src-tauri/Cargo.toml skills_cli
just ci
```

- [ ] `en.json` 与 `zh.json` 的 Skills CLI key 集合一致，reviewed error code 均有安全文案。
- [ ] IPC registry/generated map、architecture docs 与 migration docs 无 drift。
- [ ] `just ci` 的 passed/failed/skipped 与 native/external missing evidence 分开报告。

## Rollback Boundaries

- `backend-contract` 回滚只移除它新增的 additive contract；消费者 child 必须先回滚。
- `page-shell` 回滚只恢复页面壳与 census 位置，不回滚 backend contract。
- `batch-actions` 回滚同时移除其 store actions/uninstall dialog；因此 detail 是显式后继，不能独立保留依赖。
- `update-center` 通过兼容 migration/feature disable 回滚，不能留下旧二进制无法读取的 future-version DB。
- 父任务只在所有 child 完成后归档；不在 planning 阶段提交、启动、归档或推送。
