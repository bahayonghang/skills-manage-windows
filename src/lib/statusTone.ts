// 语义状态色工具：统一 success / warning / info / error 四态的类名，
// 杜绝各组件复制原生 Tailwind 调色板（amber/emerald/sky/red…）。
//
// 配套 token 见 src/index.css：
//   --success / --warning / --info（fill，复用官方 Catppuccin --ctp-*，随 6 套主题换肤）
//   --success-foreground / --warning-foreground / --info-foreground
//     （暗色主题 = fill 本身；浅色主题取更深值，已用 WCAG 公式验 ≥4.5:1 on /10 tint）
//   error 态复用既有 --destructive，不新建 token。
//
// 用法约定：
//   彩色文字     → text-{tone}-foreground（全主题已验 AA）
//   tint 背景    → bg-{tone}/10（必要时 /15）
//   状态点 swatch → bg-{tone}
// 不要再写 `dark:text-amber-300` 这类二元适配——foreground token 已按主题自适应。

export type StatusTone = "success" | "warning" | "info" | "error";

/** 带边框的 tint chip 颜色类（不含 `border` 宽度本身与圆角/间距，调用方自带布局类）。 */
export const statusChipClass: Record<StatusTone, string> = {
  success: "border-success/30 bg-success/10 text-success-foreground",
  warning: "border-warning/30 bg-warning/10 text-warning-foreground",
  info: "border-info/30 bg-info/10 text-info-foreground",
  error: "border-destructive/30 bg-destructive/10 text-destructive",
};

/** 无边框 tint（仅底色 + 前景文字）。 */
export const statusSoftClass: Record<StatusTone, string> = {
  success: "bg-success/10 text-success-foreground",
  warning: "bg-warning/10 text-warning-foreground",
  info: "bg-info/10 text-info-foreground",
  error: "bg-destructive/10 text-destructive",
};

/** 纯前景文字色（无底色）。 */
export const statusTextClass: Record<StatusTone, string> = {
  success: "text-success-foreground",
  warning: "text-warning-foreground",
  info: "text-info-foreground",
  error: "text-destructive",
};

/** 状态点 / 图标 swatch 的实心填充（其上无文字叠加）。 */
export const statusFillClass: Record<StatusTone, string> = {
  success: "bg-success",
  warning: "bg-warning",
  info: "bg-info",
  error: "bg-destructive",
};
