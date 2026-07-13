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

## Validation

- 定向运行受影响的测试文件，确认交互和断言本身正确。
- 连续多次运行相关测试组，确认无时序失败。
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
