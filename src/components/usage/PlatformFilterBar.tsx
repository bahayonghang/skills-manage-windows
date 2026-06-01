import { useTranslation } from "react-i18next";

import type { ProviderHealth } from "@/types/usage";
import { cn } from "@/lib/utils";

interface PlatformFilterBarProps {
  providers: ProviderHealth[];
  selected: string | null;
  onSelect: (source: string | null) => void;
  className?: string;
}

/**
 * 平台筛选条 —— 顶部一排 pill：「全部平台」+ 全部 provider。
 *
 * 固定列出全部 provider（不过滤），顺序沿用数组顺序与 ProviderHealthList 对齐。
 * 无数据 provider（callCount === 0）灰显 + 禁用，让支持的工具集一目了然，
 * stub provider 永远灰显。选中态用 source = provider.displayName 驱动整页作用域。
 */
export function PlatformFilterBar({
  providers,
  selected,
  onSelect,
  className,
}: PlatformFilterBarProps) {
  const { t } = useTranslation();

  return (
    <div
      data-testid="platform-filter-bar"
      className={cn("flex flex-wrap items-center gap-2", className)}
    >
      <PlatformPill
        testId="platform-pill-all"
        label={t("skillUsage.platformFilter.all")}
        active={selected === null}
        onClick={() => onSelect(null)}
      />
      {providers.map((p) => {
        const disabled = p.callCount === 0;
        return (
          <PlatformPill
            key={p.providerId}
            testId={`platform-pill-${p.providerId}`}
            label={p.displayName}
            active={selected === p.displayName}
            disabled={disabled}
            title={disabled ? t("skillUsage.notDetected") : undefined}
            onClick={() => onSelect(p.displayName)}
          />
        );
      })}
    </div>
  );
}

interface PlatformPillProps {
  testId: string;
  label: string;
  active: boolean;
  disabled?: boolean;
  title?: string;
  onClick: () => void;
}

function PlatformPill({
  testId,
  label,
  active,
  disabled = false,
  title,
  onClick,
}: PlatformPillProps) {
  return (
    <button
      type="button"
      data-testid={testId}
      data-active={active}
      disabled={disabled}
      title={title}
      onClick={onClick}
      className={cn(
        "rounded-full border px-3 py-1 text-xs transition-colors",
        disabled && "cursor-not-allowed opacity-40",
        !disabled && active && "border-primary bg-primary/10 text-primary",
        !disabled &&
          !active &&
          "border-border text-muted-foreground hover:text-foreground",
      )}
    >
      {label}
    </button>
  );
}
