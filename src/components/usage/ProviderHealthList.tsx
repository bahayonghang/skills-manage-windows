import { useTranslation } from "react-i18next";

import type { ProviderHealth } from "@/types/usage";
import { timeAgo } from "@/lib/timeAgo";
import { cn } from "@/lib/utils";

interface ProviderHealthListProps {
  providers: ProviderHealth[];
  className?: string;
}

/**
 * Provider 健康表 —— 一行一个 provider，左：状态点 + 名称；右：调用数 / 上次扫描时间。
 *
 * 与 SkillUsageView 内嵌占位的差异：
 * - 用 Tailwind 圆点 + 颜色编码区分 available 状态
 * - call_count = 0 但 available=true 时显示 0（而非"未检测到"），避免误导
 * - 单 provider 行可点击未来扩展（保留 div 便于扩展，目前不挂事件）
 */
export function ProviderHealthList({ providers, className }: ProviderHealthListProps) {
  const { t } = useTranslation();

  if (providers.length === 0) {
    return (
      <div className={cn("rounded border border-dashed border-border/60 p-6", className)}>
        <p className="text-center text-xs text-muted-foreground">
          {t("skillUsage.empty.placeholder")}
        </p>
      </div>
    );
  }

  return (
    <ul className={cn("divide-y divide-border/60", className)}>
      {providers.map((p) => (
        <li
          key={p.providerId}
          data-testid={`provider-row-${p.providerId}`}
          className="flex items-center justify-between px-3 py-2"
        >
          <div className="flex items-center gap-2.5">
            <span
              aria-hidden
              className={cn(
                "size-2 rounded-full",
                p.available ? "bg-emerald-500" : "bg-muted-foreground/30"
              )}
            />
            <span
              className={cn(
                "text-sm",
                p.available ? "text-foreground" : "text-muted-foreground"
              )}
            >
              {p.displayName}
            </span>
          </div>
          <div className="flex items-center gap-3 text-xs tabular-nums">
            {p.available ? (
              <>
                <span className="text-foreground">
                  {p.callCount.toLocaleString()}
                </span>
                {p.scannedAtMs > 0 && (
                  <span className="text-muted-foreground">
                    {timeAgo(p.scannedAtMs)}
                  </span>
                )}
              </>
            ) : (
              <span className="text-muted-foreground">
                {t("skillUsage.notDetected")}
              </span>
            )}
          </div>
        </li>
      ))}
    </ul>
  );
}
