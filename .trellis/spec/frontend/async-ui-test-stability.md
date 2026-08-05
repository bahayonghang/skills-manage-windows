# 前端异步 UI 测试稳定性约定

## Scope / Trigger

适用于 Testing Library 中由用户动作打开 dialog、drawer 或其他异步 surface，
并继续在该 surface 内执行多步交互的测试。

## Contract

- 先等待具名 surface 出现，再用 `within(surface)` 查询其内部控件与结果。
- 不使用全局 `screen` 查询继续驱动 modal 内部流程，避免命中背景页面或旧节点。
- 全量 `just ci` 会并行运行前端测试与 Rust 检查；依赖异步 surface 渲染的用例
  使用共享的局部等待预算，当前基线为 `5_000ms`。
- 不为单个 flake 提高 Testing Library 的全局 timeout。等待预算应只覆盖已知的异步
  边界，不能掩盖同步断言失败或生产行为错误。
- 测试替身的 effect 必须保持幂等，不得在 props 未变化时覆盖用户已经进入的步骤。
- Update Center 的 scope skills 与 repository progress 必须分别断言：模式弹窗保留
  `Check all (N)`/“检查全部（N）”，进度分母明确是“可查询的去重远端仓库”。测试不得把
  `N skills / 1 repository` 当作筛选成一个 skill。
- 非 actionable 结果仍是可见结果。`unsupported` 非空时必须显示只读 tab、计数、skill ID
  和固定本地化 reason；当其它 bucket 为空时首选该 tab，且不得渲染“全部最新”。
- 前端 fixture 应允许旧 inventory 缺少 `unsupported`，并将其归一化为空集合；新增测试
  fixture 则显式提供该字段，避免可选兼容性掩盖生产行为。

## Validation

- 定向运行受影响的测试文件，确认交互和断言本身正确。
- 连续多次运行相关测试组，确认无时序失败。
- Update Center 定向测试同时覆盖 preferred tab、toolbar count、unsupported panel、en/zh
  reason parity，以及 progress 中 scope/repository 两个维度。
- 最后运行 `just ci`，覆盖与 Rust 检查并行时的资源竞争场景。

## Example

```tsx
const dialog = await screen.findByRole(
  "dialog",
  { name: /import wizard/i },
  { timeout: ASYNC_UI_TIMEOUT_MS },
);
const wizard = within(dialog);

fireEvent.click(
  await wizard.findByRole(
    "button",
    { name: /review import/i },
    { timeout: ASYNC_UI_TIMEOUT_MS },
  ),
);
```
