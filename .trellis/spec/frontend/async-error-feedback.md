# 前端异步动作失败反馈约定

## Scope / Trigger

当页面、controller 或 dialog 触发 store async action，且成功后才会打开目标面板/抽屉/对话框时，调用方必须负责当前可见界面的失败反馈。

典型场景：模式确认弹窗先执行 refresh，成功后才打开结果中心；如果 refresh 失败，目标中心不会打开，单纯写 store `error` 不会被用户看到。

## Contract

- Store action 仍负责业务状态：`isLoading/isRefreshing`、`error`、rethrow。
- 可见 UI 调用方负责交互反馈：捕获错误、显示当前界面内联错误，并发出 toast。
- 可见 UI 渲染 backend rejection 时必须经 `formatBackendError(err, t)` 保留稳定 code 的
  i18n 语义并丢弃动态 details；不得用 `String(err)` 直接拼入 toast 或内联错误。
- 新一轮提交或关闭弹窗时必须清除上一次内联错误。
- 成功路径不得因为失败处理改变原有导航/打开面板参数。
- 被多处 fire-and-forget 复用的共享 loader（如 `loadCentralSkills`）不得直接改成 rethrow；用可选参数（如 `{ throwOnError?: boolean }`，默认保持吞错写 store error）让需要反馈的可见 UI 调用方显式选择 rethrow，避免给既有调用点制造 unhandled rejection。
- 同一流程中"决定成败的主 action"与"成功后的后续同步步骤"必须分开捕获：后续步骤失败只报自己的错误 toast，不得落入主 action 的 catch 而误报、或改变打开面板的参数与时机。参考 `centralUpdateCheckModeController.handleConfirm` 的嵌套 try。

## Validation & Error Matrix

| Condition | Required UI behavior |
| --- | --- |
| Store action resolves | Continue existing success flow and clear local error |
| Store action rejects before target panel opens | Keep current dialog/view open, show inline error, show toast, re-enable submit |
| Store action rejects with a reviewed backend code | Render the localized public message; never expose legacy details |
| User retries after failure | Clear stale inline error before the new request starts |
| User closes the dialog/view | Clear dialog-local error |

## Tests Required

- Component-level test for the presentational inline error state.
- View/controller test proving rejected store action shows toast + inline error and does not open the success-only panel.
- Coded-error test proving the localized public message is visible and adversarial token/URL/path details are absent.
- Retry coverage proving stale inline error is cleared before the next request.

## Wrong vs Correct

### Wrong

```tsx
try {
  const result = await store.refresh(scope);
  openResultDialog(result);
} finally {
  setIsSubmitting(false);
}
```

### Correct

```tsx
setSubmitError(null);
setIsSubmitting(true);
try {
  const result = await store.refresh(scope);
  openResultDialog(result);
} catch (err) {
  const message = t("central.updateCheckError", {
    error: formatBackendError(err, t),
  });
  setSubmitError(message);
  toast.error(message);
} finally {
  setIsSubmitting(false);
}
```
