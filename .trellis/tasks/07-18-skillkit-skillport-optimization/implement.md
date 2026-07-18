# Implementation Plan: SkillKit 对标优化路线图

## 1. 执行原则

- 不启动父任务。一次只启动拥有当前交付物的一个子任务。
- 当前 Codex inline workflow 不并行实施子任务；按 1 → 2 → 3 → 4 串行推进，避免共享工作树和共同风险文件交叉修改。
- 每个子任务都先补失败测试/基线，再实现，再跑定向验证和 `just ci`。
- 子任务完成并归档后，回到父任务更新任务地图；依赖不满足时不提前启动后续任务。
- 任何性能结论必须附基线与复测；任何静态 detector 命中都必须人工核验后才进入修复。

## 2. 推荐顺序

### Phase A: 统一导入入口与 ZIP

1. 审批并启动 `07-18-unified-skill-import`。
2. 先完成 ZIP 安全模型与 command tests，再实现 frontend intent router 和 ZIP wizard。
3. 验证 GitHub wizard 无回归，归档子任务。

### Phase B: GitHub 清单快路径

1. 可与 Phase A 独立规划，但 inline 实施等待 Phase A 归档，不同时修改共享导入表面。
2. 审批并启动 `07-18-github-import-manifest-fast-path`。
3. 先记录 archive 基线与 parity fixtures，再实现 preview 快路径、selected subtree import、typed fallback。
4. 只有数据证明重复 tree fetch 是主要成本时才加 TTL/LRU metadata cache。
5. 归档子任务。

### Phase C: 排版与 WCAG

1. 审批并启动 `07-18-dense-typography-wcag`。
2. 启动时重跑生产 TS/TSX inventory 并记录相对 planning 快照的 delta。Planning 快照为 173 个数值型 arbitrary 字号、64 个文件：133 px（23x10px、107x11px、2x12px、1x13px）+ 40 rem；alpha-risk 为 22（21 foreground + 1 primary）。
3. 先按 label/meta/code/status/micro/decorative 分类，再定义 token 和禁止整个 `text-[...]` 家族的 no-growth guard；不建立 arbitrary px/rem/color allowlist。
4. Phase A 已先改造 `CentralSkillsShell.tsx`；基于其最新形态按 Central → GitHub wizard → Usage → 其他共享 UI 分批迁移，技能详情仅做 token 等价替换。
5. 完成主题/缩放/最小窗回归后归档。

### Phase D: 深链

1. 确认 Phase A 的 import intent router 已稳定。
2. 审批并启动 `07-18-skillport-import-deep-link`，单独确认新 Tauri 插件依赖。
3. 实现纯 parser、冷启动队列、单实例转发、frontend prefill；最后验证 Windows 安装包协议注册。
4. 归档子任务。

## 3. 子任务门禁

每个子任务至少执行：

```powershell
pnpm typecheck
pnpm lint
pnpm test -- --run <相关 Vitest>
cd src-tauri; cargo test <相关模块>
cd src-tauri; cargo clippy -- -D warnings
git diff --check
just ci
```

深链或 bundle 配置变更额外执行：

```powershell
pnpm tauri build
```

并确认 `src-tauri/target/release/bundle/nsis/` 生成新安装包，安装后用 PowerShell 启动 `skillport://import?...` 完成冷/热两条路径验证。

## 4. 父任务最终审查

1. 确认四个 child 均已归档，父 `task.json` 进度为 4/4。
2. 对照 `research/skillkit-skillport-comparison.md` 逐项检查：吸收项已落地，拒绝项未意外进入。
3. 复核 GitHub preview-only、sourcePath、Central 原子写入、远程目标、Operation Logs 和 i18n 契约。
4. 复跑 `just ci`；若任一子任务改过 bundle，再复核最新 Windows build 证据。
5. 检查 6 主题、代表 accent、900x600 和三档 font scale 的关键界面。
6. 仅在所有跨子任务 AC 有证据时归档父任务；缺失证据必须保持未通过。

## 5. 停止条件

- 快路径无法在确定性基线中减少传输或引入不可控 API 放大时，保留 archive 并停止该优化，不为了“对标”强行上线。
- ZIP 安全边界无法在预览前完整验证时，不提供导入按钮。
- 深链无法保证只打开确认 UI 或 Windows 单实例行为不可靠时，不注册协议。
- 排版迁移造成关键控件溢出、遮挡或明显降低单位屏信息量时，按 surface 回滚并保留审计结果。
