import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import { ArrowUpDown, Check, SlidersHorizontal } from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type {
  PlatformGroupBy,
  PlatformSortDirection,
  PlatformSortField,
} from "@/lib/platformSkillViewModel";

export function PlatformSkillSortMenu({
  t,
  sortField,
  sortDirection,
  sortFieldOptions,
  sortDirectionOptions,
  onChange,
}: {
  t: TFunction;
  sortField: PlatformSortField;
  sortDirection: PlatformSortDirection;
  sortFieldOptions: Array<{ value: PlatformSortField; label: string }>;
  sortDirectionOptions: Array<{ value: PlatformSortDirection; label: string }>;
  onChange: (next: {
    field: PlatformSortField;
    direction: PlatformSortDirection;
  }) => void;
}) {
  const fieldLabel =
    sortFieldOptions.find((option) => option.value === sortField)?.label ?? sortField;
  const directionLabel =
    sortDirectionOptions.find((option) => option.value === sortDirection)?.label ??
    sortDirection;

  return (
    <MenuPrimitive.Root>
      <MenuPrimitive.Trigger
        render={
          <Button
            variant="outline"
            size="sm"
            className="h-9 shrink-0"
            data-testid="platform-toolbar-sort"
            aria-label={t("platform.toolbarSortAriaLabel")}
            title={t("platform.toolbarSortCurrent", {
              field: fieldLabel,
              direction: directionLabel,
            })}
          >
            <ArrowUpDown className="size-3.5" />
            <span className="hidden text-xs sm:inline">{fieldLabel}</span>
            <span aria-hidden className="text-[10px] text-muted-foreground">
              {sortDirection === "asc" ? "↑" : "↓"}
            </span>
          </Button>
        }
      />
      <MenuPrimitive.Portal>
        <MenuPrimitive.Positioner align="end" sideOffset={6} className="z-50 outline-none">
          <MenuPrimitive.Popup className={menuPopupClassName("min-w-[210px]")}>
            {sortFieldOptions.flatMap((field) =>
              sortDirectionOptions.map((direction) => {
                const active =
                  field.value === sortField && direction.value === sortDirection;
                return (
                  <MenuPrimitive.Item
                    key={`${field.value}-${direction.value}`}
                    data-testid={`platform-toolbar-sort-${field.value}-${direction.value}`}
                    onClick={() =>
                      onChange({ field: field.value, direction: direction.value })
                    }
                    className={menuItemClassName(
                      active && "bg-accent/60 text-accent-foreground"
                    )}
                  >
                    <span>{field.label}</span>
                    <span className="ml-auto text-[10px] text-muted-foreground">
                      {direction.label}
                    </span>
                    {active && <Check className="size-3" aria-hidden />}
                  </MenuPrimitive.Item>
                );
              })
            )}
          </MenuPrimitive.Popup>
        </MenuPrimitive.Positioner>
      </MenuPrimitive.Portal>
    </MenuPrimitive.Root>
  );
}

export function PlatformSkillViewMenu({
  t,
  groupBy,
  groupByOptions,
  onChangeGroupBy,
}: {
  t: TFunction;
  groupBy: PlatformGroupBy;
  groupByOptions: Array<{ value: PlatformGroupBy; label: string }>;
  onChangeGroupBy: (value: PlatformGroupBy) => void;
}) {
  const hasActiveGroup = groupBy !== "none";

  return (
    <MenuPrimitive.Root>
      <MenuPrimitive.Trigger
        render={
          <Button
            variant="outline"
            size="sm"
            className="h-9 shrink-0"
            data-testid="platform-toolbar-view"
            aria-label={t("platform.toolbarViewAriaLabel")}
          >
            <SlidersHorizontal className="size-3.5" />
            <span className="hidden text-xs sm:inline">{t("platform.toolbarView")}</span>
            {hasActiveGroup && (
              <span
                data-testid="platform-toolbar-view-dot"
                aria-label={t("platform.toolbarViewBadgeActive")}
                className="size-1.5 rounded-full bg-primary"
              />
            )}
          </Button>
        }
      />
      <MenuPrimitive.Portal>
        <MenuPrimitive.Positioner align="end" sideOffset={6} className="z-50 outline-none">
          <MenuPrimitive.Popup className={menuPopupClassName("min-w-[210px]")}>
            <MenuPrimitive.Group>
              <MenuPrimitive.GroupLabel className={menuLabelClassName()}>
                {t("platform.toolbarViewSectionGroup")}
              </MenuPrimitive.GroupLabel>
              {groupByOptions.map((option) => {
                const active = option.value === groupBy;
                return (
                  <MenuPrimitive.Item
                    key={option.value}
                    data-testid={`platform-toolbar-view-group-${option.value}`}
                    onClick={() => onChangeGroupBy(option.value)}
                    className={menuItemClassName(
                      active && "bg-accent/60 text-accent-foreground"
                    )}
                  >
                    <span>{option.label}</span>
                    {active && <Check className="ml-auto size-3" aria-hidden />}
                  </MenuPrimitive.Item>
                );
              })}
            </MenuPrimitive.Group>
          </MenuPrimitive.Popup>
        </MenuPrimitive.Positioner>
      </MenuPrimitive.Portal>
    </MenuPrimitive.Root>
  );
}

function menuPopupClassName(extra?: string): string {
  return cn(
    "rounded-lg bg-popover p-1 text-sm text-popover-foreground shadow-md ring-1 ring-foreground/10 outline-none",
    "data-[starting-style]:animate-in data-[starting-style]:fade-in-0 data-[starting-style]:zoom-in-95",
    "data-[ending-style]:animate-out data-[ending-style]:fade-out-0 data-[ending-style]:zoom-out-95",
    "animation-duration-100",
    extra
  );
}

function menuItemClassName(extra?: string | false): string {
  return cn(
    "flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-1.5 outline-none data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground",
    extra
  );
}

function menuLabelClassName(): string {
  return "px-2 pt-1 pb-0.5 text-[10px] uppercase tracking-wide text-muted-foreground/80";
}
