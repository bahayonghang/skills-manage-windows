import { useTranslation } from "react-i18next";

import type { ProviderHealth } from "@/types/usage";
import { formatUsageRelativeTime } from "@/components/usage/usageFormat";
import { cn } from "@/lib/utils";

interface ProviderHealthListProps {
  providers: ProviderHealth[];
  /** 当前选中的平台（provider displayName）；与 PlatformFilterBar 共享。 */
  activeSource?: string | null;
  /** 提供时，有数据的行可点击切换筛选；省略则为纯展示列表。 */
  onSelect?: (source: string | null) => void;
  className?: string;
}

/**
 * Provider 健康表 —— 一行一个 provider，左：状态点 + 名称；右：调用数 / 上次扫描时间。
 *
 * 与 SkillUsageView 内嵌占位的差异：
 * - 用 Tailwind 圆点 + 颜色编码区分 available 状态
 * - call_count = 0 但 available=true 时显示 0（而非"未检测到"），避免误导
 * - 传入 onSelect 时，有数据的行可点击，与顶部 PlatformFilterBar 同步高亮 active
 */
export function ProviderHealthList({
  providers,
  activeSource,
  onSelect,
  className,
}: ProviderHealthListProps) {
  const { t, i18n } = useTranslation();

  if (providers.length === 0) {
    return (
      <div
        className={cn(
          "rounded border border-dashed border-border/60 p-6",
          className,
        )}
      >
        <p className="text-center text-xs text-muted-foreground">
          {t("skillUsage.empty.placeholder")}
        </p>
      </div>
    );
  }

  const rowClass =
    "flex w-full items-center justify-between px-3 py-2 text-left";

  return (
    <ul className={cn("divide-y divide-border/60", className)}>
      {providers.map((p) => {
        const interactive = Boolean(onSelect) && p.callCount > 0;
        const active = activeSource != null && activeSource === p.displayName;
        const left = (
          <div className="flex items-center gap-2.5">
            <span
              aria-hidden
              className={cn(
                "size-2 rounded-full",
                p.available ? "bg-success" : "bg-muted-foreground/30",
              )}
            />
            <span
              className={cn(
                "text-sm",
                p.available ? "text-foreground" : "text-muted-foreground",
              )}
            >
              {p.displayName}
            </span>
          </div>
        );
        const right = (
          <div className="flex items-center gap-3 text-xs tabular-nums">
            {p.available ? (
              <>
                <span className="text-foreground">
                  {p.callCount.toLocaleString()}
                </span>
                {p.scannedAtMs > 0 && (
                  <span className="text-muted-foreground">
                    {formatUsageRelativeTime(p.scannedAtMs, i18n.language)}
                  </span>
                )}
              </>
            ) : (
              <span className="text-muted-foreground">
                {t("skillUsage.notDetected")}
              </span>
            )}
          </div>
        );

        return (
          <li
            key={p.providerId}
            data-testid={`provider-row-${p.providerId}`}
            data-active={active}
            className={cn(active && "bg-primary/5")}
          >
            {interactive ? (
              <button
                type="button"
                onClick={() => onSelect?.(p.displayName)}
                className={cn(
                  rowClass,
                  "transition-colors hover:bg-muted/40",
                  active && "text-primary",
                )}
              >
                {left}
                {right}
              </button>
            ) : (
              <div className={rowClass}>
                {left}
                {right}
              </div>
            )}
          </li>
        );
      })}
    </ul>
  );
}
