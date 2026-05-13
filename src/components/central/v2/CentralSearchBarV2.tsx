import { Search, X } from "lucide-react";
import type { TFunction } from "i18next";

import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { CentralQueryAst, CentralQueryFilter } from "@/types";

/**
 * V2 搜索栏：
 * - 输入框接受结构化语法（tag:foo repo:bar 等），由 view-model 解析
 * - 显示已解析出的 chips（每个 filter 一个 chip，可单独移除）
 * - invalid token 以警告 hint 形式提示
 * - 右侧 clear-all 按钮
 */

export interface CentralSearchBarV2Props {
  t: TFunction;
  value: string;
  onChange: (next: string) => void;
  queryAst: CentralQueryAst;
  /** 用户点击 chip 上的 × 时调用，传当前 filter 索引由父组件去重写 query string。 */
  onRemoveFilter: (filterIndex: number) => void;
  /** 清空全部搜索文本（不影响 sidebar 多选）。 */
  onClear: () => void;
  /** 命令面板触发按钮的 click，可选。 */
  onOpenPalette?: () => void;
  placeholder?: string;
  inputId?: string;
}

export function CentralSearchBarV2({
  t,
  value,
  onChange,
  queryAst,
  onRemoveFilter,
  onClear,
  onOpenPalette,
  placeholder,
  inputId,
}: CentralSearchBarV2Props) {
  const hasInvalid = queryAst.invalid.length > 0;
  return (
    <div data-testid="central-search-bar-v2" className="flex flex-col gap-2">
      <div className="relative flex w-full items-center">
        <Search className="pointer-events-none absolute left-3 size-4 text-muted-foreground/80" />
        <Input
          id={inputId}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          placeholder={placeholder ?? t("central.v2.searchPlaceholder")}
          className="pl-10 pr-24"
          data-testid="central-search-input-v2"
        />
        <div className="absolute right-2 flex items-center gap-1">
          {value.length > 0 && (
            <button
              type="button"
              onClick={onClear}
              aria-label={t("central.v2.selectionClear")}
              className="grid size-7 place-items-center rounded-md text-muted-foreground hover:bg-muted/60 hover:text-foreground"
            >
              <X className="size-4" />
            </button>
          )}
          {onOpenPalette && (
            <button
              type="button"
              data-testid="central-open-palette"
              onClick={onOpenPalette}
              aria-label={t("central.v2.commandPaletteOpen")}
              className="inline-flex items-center gap-1 rounded-md border border-border/80 bg-background px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground hover:border-border hover:bg-muted/40 hover:text-foreground"
            >
              ⌘K
            </button>
          )}
        </div>
      </div>

      {(queryAst.filters.length > 0 || hasInvalid) && (
        <div className="flex flex-wrap items-center gap-1.5 text-[11px]">
          {queryAst.filters.map((filter, idx) => (
            <FilterChip
              key={`${filter.kind}-${getChipValue(filter)}-${idx}`}
              filter={filter}
              onRemove={() => onRemoveFilter(idx)}
              t={t}
            />
          ))}
          {hasInvalid && (
            <span
              data-testid="central-search-invalid-hint"
              className="rounded-md border border-amber-500/40 bg-amber-500/10 px-1.5 py-0.5 text-amber-700 dark:text-amber-300"
            >
              {t("central.v2.searchInvalidTokens", { tokens: queryAst.invalid.join(", ") })}
            </span>
          )}
        </div>
      )}
    </div>
  );
}

function getChipValue(filter: CentralQueryFilter): string {
  if (filter.kind === "created" || filter.kind === "updated") {
    return `${filter.op}${filter.value}`;
  }
  return String(filter.value);
}

function FilterChip({
  filter,
  onRemove,
  t,
}: {
  filter: CentralQueryFilter;
  onRemove: () => void;
  t: TFunction;
}) {
  const value = getChipValue(filter);
  const labelKey = filter.negated ? "central.v2.filtersChipNegated" : "central.v2.filtersChip";
  return (
    <span
      data-testid={`filter-chip-${filter.kind}`}
      className={cn(
        "inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 font-mono",
        filter.negated
          ? "border-destructive/40 bg-destructive/10 text-destructive"
          : "border-primary/30 bg-primary/10 text-primary"
      )}
    >
      <span>{t(labelKey, { key: filter.kind, value })}</span>
      <button
        type="button"
        onClick={onRemove}
        aria-label={t("central.v2.filtersRemove", { key: filter.kind, value })}
        className="grid size-3.5 place-items-center rounded-sm hover:bg-foreground/10"
      >
        <X className="size-2.5" />
      </button>
    </span>
  );
}
