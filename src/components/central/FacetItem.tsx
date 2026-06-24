import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/**
 * Sidebar 通用 facet 项。可单选 / 多选（由 caller 控制 active 状态）。
 *
 * 设计：
 * - 左侧 16px 图标槽位
 * - 主标签 truncate
 * - 右侧 count 单元（tabular-nums，固定宽）
 * - active 状态使用 primary 配色
 * - disabled 状态降低不透明度并禁用点击
 */
export interface FacetItemProps {
  label: string;
  active: boolean;
  count?: number;
  icon?: ReactNode;
  /** 折叠层级，0 = 顶层，1 = owner 下面的 repo 等。 */
  indentLevel?: 0 | 1;
  disabled?: boolean;
  description?: string;
  testId?: string;
  className?: string;
  "data-pinned"?: boolean;
  onClick: () => void;
  /** 常驻显示的右侧角标（如「可更新数」），在 count 之前渲染。 */
  badge?: ReactNode;
  /** Hover 时显示的右侧操作（如 delete）。 */
  trailingAction?: ReactNode;
}

export function FacetItem({
  label,
  active,
  count,
  icon,
  indentLevel = 0,
  disabled = false,
  description,
  testId,
  className,
  "data-pinned": dataPinned,
  onClick,
  badge,
  trailingAction,
}: FacetItemProps) {
  return (
    <div
      className={cn(
        "group relative flex min-h-9 w-full items-center gap-2 rounded-lg border px-2 py-1.5 text-left text-xs transition-[background-color,border-color,box-shadow,color]",
        indentLevel === 1 && "ml-3",
        active
          ? "border-primary/25 bg-background text-foreground shadow-sm ring-1 ring-primary/10"
          : "border-transparent text-muted-foreground hover:border-border/70 hover:bg-background/70 hover:text-foreground",
        disabled && "pointer-events-none opacity-50",
        className,
      )}
      data-pinned={dataPinned}
    >
      <button
        type="button"
        data-testid={testId}
        data-active={active}
        onClick={onClick}
        title={description}
        className="flex min-h-7 min-w-0 flex-1 items-center gap-2 text-left"
      >
        {icon ? (
          <span className="grid size-4 shrink-0 place-items-center text-muted-foreground/80">
            {icon}
          </span>
        ) : null}
        <span className="min-w-0 flex-1 truncate text-pretty font-medium">{label}</span>
      </button>
      {badge}
      {typeof count === "number" && (
        <span
          className={cn(
            "shrink-0 rounded-md border bg-background px-1.5 py-0.5 font-mono text-[10px] tabular-nums",
            active
              ? "border-primary/30 text-foreground"
              : "border-border/80 text-muted-foreground",
          )}
        >
          {count}
        </span>
      )}
      {trailingAction}
    </div>
  );
}
