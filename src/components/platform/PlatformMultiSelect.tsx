import { useCallback, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Checkbox } from "@/components/ui/checkbox";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetLabel,
  getPlatformTargetMemberNames,
  getPlatformTargetTitleHint,
  isUniversalPlatformTarget,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";

// ─── usePlatformTargetSelection ───────────────────────────────────────────────
//
// Shared multi-select behavior for the install dialogs: selection state,
// disabled-target toggle guard, default-selection reset semantics, and the
// selected → install agent id derivation (universal groups map to their
// install agent; results are deduped).

export interface UsePlatformTargetSelectionOptions {
  /** Platform targets to select from (caller has already filtered `central`). */
  targets: PlatformTarget[];
  /** Targets for which toggling is a no-op and that are excluded from the install derivation. */
  isTargetDisabled?: (target: PlatformTarget) => boolean;
  /** Default-selection semantics for `reset()`; defaults to "all non-disabled targets". */
  isTargetDefaultSelected?: (target: PlatformTarget) => boolean;
}

export interface PlatformTargetSelection {
  selectedIds: Set<string>;
  isSelected: (target: PlatformTarget) => boolean;
  toggle: (agentId: string, checked: boolean) => void;
  reset: () => void;
  selectedInstallAgentIds: () => string[];
}

// Design keeps the selection hook co-located with the grid it drives; the
// react-refresh warning about mixed exports is intentional here.
// eslint-disable-next-line react-refresh/only-export-components
export function usePlatformTargetSelection(
  options: UsePlatformTargetSelectionOptions,
): PlatformTargetSelection {
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  // Latest-options ref keeps `toggle`/`reset` referentially stable so caller
  // open-effects can list them as dependencies without churn; both always
  // read the current targets and predicates when invoked.
  const optionsRef = useRef(options);
  optionsRef.current = options;

  const toggle = useCallback((agentId: string, checked: boolean) => {
    const { targets, isTargetDisabled } = optionsRef.current;
    const target = targets.find((candidate) => candidate.id === agentId);
    if (target && isTargetDisabled?.(target)) {
      return;
    }

    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(agentId);
      } else {
        next.delete(agentId);
      }
      return next;
    });
  }, []);

  const reset = useCallback(() => {
    const { targets, isTargetDisabled, isTargetDefaultSelected } =
      optionsRef.current;
    const shouldSelect =
      isTargetDefaultSelected ??
      ((target: PlatformTarget) => !isTargetDisabled?.(target));
    setSelectedIds(
      new Set(targets.filter(shouldSelect).map((target) => target.id)),
    );
  }, []);

  const isSelected = (target: PlatformTarget) => selectedIds.has(target.id);

  // Recomputed with the options passed on the current render (predicates such
  // as InstallDialog's targetMode-aware `isTargetDisabled` may change between
  // renders, so no caching).
  const selectedInstallAgentIds = () =>
    Array.from(
      new Set(
        options.targets
          .filter((target) => selectedIds.has(target.id))
          .filter((target) => !options.isTargetDisabled?.(target))
          .flatMap((target) => getPlatformTargetInstallAgentIds(target)),
      ),
    );

  return { selectedIds, isSelected, toggle, reset, selectedInstallAgentIds };
}

// ─── PlatformMultiSelectGrid ──────────────────────────────────────────────────

export interface PlatformMultiSelectGridProps {
  targets: PlatformTarget[];
  isSelected: (target: PlatformTarget) => boolean;
  isDisabled?: (target: PlatformTarget) => boolean;
  onToggle: (agentId: string, checked: boolean) => void;
  /** Trailing badges per row (linked/alwaysIncluded/willReplace…); caller owns the i18n copy. */
  renderBadges?: (target: PlatformTarget) => ReactNode;
  /** Render a PlatformIcon before the target name. */
  showIcon?: boolean;
  /** Label variant for target names; "short" renders the compact universal label. */
  labelVariant?: "full" | "short";
  emptyMessage: string;
  ariaLabel: string;
}

export function PlatformMultiSelectGrid({
  targets,
  isSelected,
  isDisabled,
  onToggle,
  renderBadges,
  showIcon = false,
  labelVariant = "full",
  emptyMessage,
  ariaLabel,
}: PlatformMultiSelectGridProps) {
  const { t } = useTranslation();

  return (
    <div
      className="grid grid-cols-2 gap-x-4 gap-y-2"
      role="group"
      aria-label={ariaLabel}
    >
      {targets.length === 0 ? (
        <p className="col-span-2 text-sm text-muted-foreground">
          {emptyMessage}
        </p>
      ) : (
        targets.map((target) => {
          const displayName = getPlatformTargetLabel(target, t, labelVariant);
          const disabled = isDisabled?.(target) ?? false;
          const checked = isSelected(target);

          return (
            <div key={target.id} className="flex items-center gap-2">
              <Checkbox
                checked={checked}
                disabled={disabled}
                onCheckedChange={(value) => onToggle(target.id, !!value)}
                aria-label={displayName}
              />
              {showIcon && (
                <PlatformIcon
                  agentId={target.id}
                  className="size-3.5 shrink-0"
                />
              )}
              <div
                className={`min-w-0 flex-1 select-none ${
                  disabled
                    ? "text-muted-foreground cursor-default"
                    : "text-foreground cursor-pointer"
                }`}
                title={getPlatformTargetTitleHint(target)}
                onClick={() => !disabled && onToggle(target.id, !checked)}
              >
                <div className="truncate text-sm">{displayName}</div>
                {isUniversalPlatformTarget(target) && (
                  <div className="truncate text-ui-meta text-muted-foreground">
                    {getPlatformTargetMemberNames(target).join(", ")}
                  </div>
                )}
              </div>
              {renderBadges?.(target)}
            </div>
          );
        })
      )}
    </div>
  );
}

// ─── InstallFailureList ───────────────────────────────────────────────────────

export interface InstallFailureItem {
  key: string;
  label: string;
}

export function InstallFailureList({
  failures,
}: {
  failures: InstallFailureItem[];
}) {
  return (
    <ul className="max-h-32 space-y-0.5 overflow-auto text-xs text-destructive">
      {failures.map((failure) => (
        <li key={failure.key}>{failure.label}</li>
      ))}
    </ul>
  );
}
