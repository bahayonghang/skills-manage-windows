import type { MouseEventHandler } from "react";
import { Link2, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { Checkbox } from "@/components/ui/checkbox";
import { statusChipClass } from "@/lib/statusTone";
import { cn } from "@/lib/utils";
import type { SkillsCliPlacement } from "@/types";

import { cardShellClass } from "./UnifiedSkillCard.styles";
import type { SkillCardCheckbox } from "./UnifiedSkillCard.types";
import { CardActionButton } from "./UnifiedSkillCardParts";

export interface SkillCardDenseRowProps {
  name: string;
  className?: string;
  path?: string | null;
  placements: readonly SkillsCliPlacement[];
  checkbox?: SkillCardCheckbox;
  updateAvailable?: boolean;
  onDetail?: MouseEventHandler<HTMLButtonElement>;
  onManageLinks?: () => void;
  onUninstall?: () => void;
  isLoading?: boolean;
}

type DenseRowStatus = "copy" | "conflict" | "missing" | "unavailable";

function denseRowStatus(
  placements: readonly SkillsCliPlacement[],
): DenseRowStatus | null {
  let status: DenseRowStatus | null = null;
  for (const placement of placements) {
    switch (placement.state) {
      case "conflict":
        return "conflict";
      case "direct_copy":
        status = "copy";
        break;
      case "missing":
        if (status === null || status === "unavailable") {
          status = "missing";
        }
        break;
      case "unavailable":
        if (status === null) {
          status = "unavailable";
        }
        break;
      case "managed_link":
        break;
      default: {
        const _exhaustive: never = placement.state;
        return _exhaustive;
      }
    }
  }
  return status;
}

const DENSE_STATUS_TONE: Record<DenseRowStatus, "warning" | "error" | "info"> = {
  copy: "warning",
  conflict: "error",
  missing: "info",
  unavailable: "warning",
};

export function SkillCardDenseRow({
  name,
  className,
  path: rawPath,
  placements,
  checkbox,
  updateAvailable,
  onDetail,
  onManageLinks,
  onUninstall,
  isLoading,
}: SkillCardDenseRowProps) {
  const { t } = useTranslation();
  const managed = placements.filter(
    (placement) => placement.state === "managed_link",
  );
  const visible = managed.slice(0, 4);
  const extra = Math.max(0, managed.length - visible.length);
  const status = managed.length === 0 ? denseRowStatus(placements) : null;
  const path = rawPath?.trim() || t("skillsCli.pathUnknown");

  return (
    <div
      data-testid={`skills-cli-dense-card-${name}`}
      className={cn(
        cardShellClass(checkbox?.checked),
        "group/skill-card relative flex h-auto min-h-[76px] flex-col justify-center gap-1 px-3 py-2",
        isLoading && "opacity-50",
        className,
      )}
    >
      <div className="flex min-w-0 items-center gap-2">
        {checkbox && (
          <div
            className="shrink-0"
            onClick={(event) => event.stopPropagation()}
            onKeyDown={(event) => event.stopPropagation()}
          >
            <Checkbox
              checked={checkbox.checked}
              onCheckedChange={checkbox.onChange}
              aria-label={t("common.selectSkill")}
              className="relative size-5 after:absolute after:left-1/2 after:top-1/2 after:size-10 after:-translate-x-1/2 after:-translate-y-1/2 after:content-['']"
            />
          </div>
        )}
        {onDetail ? (
          <button
            type="button"
            className="min-h-7 min-w-0 flex-1 truncate rounded-md text-left text-sm font-semibold tracking-[-0.01em] text-foreground underline-offset-4 transition-[color,box-shadow] hover:text-primary hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            onClick={onDetail}
            aria-label={t("central.viewDetailsLabel", { name })}
            title={name}
          >
            {name}
          </button>
        ) : (
          <h3
            title={name}
            className="min-w-0 flex-1 truncate text-sm font-semibold tracking-[-0.01em] text-foreground"
          >
            {name}
          </h3>
        )}
        {updateAvailable && (
          <span
            className={cn(
              "shrink-0 rounded-full border px-1.5 py-0.5 text-xs font-medium",
              statusChipClass.warning,
            )}
          >
            {t("skillsCli.updateAvailable")}
          </span>
        )}
        <div
          className={cn(
            "flex shrink-0 items-center gap-2 opacity-0 transition-opacity duration-150",
            "group-hover/skill-card:opacity-100 group-focus-within/skill-card:opacity-100",
          )}
          onClick={(event) => event.stopPropagation()}
        >
          {onManageLinks && (
            <CardActionButton
              onClick={() => onManageLinks()}
              disabled={isLoading}
              title={t("skillsCli.manageLinks")}
              ariaLabel={t("skillsCli.manageLinksLabel", { name })}
              icon={<Link2 className="size-4" />}
            />
          )}
          {onUninstall && (
            <CardActionButton
              onClick={() => onUninstall()}
              disabled={isLoading}
              title={t("skillsCli.uninstall", { name })}
              ariaLabel={t("skillsCli.uninstall", { name })}
              testId={`uninstall-skills-cli-${name}`}
              danger
              icon={<Trash2 className="size-4" />}
            />
          )}
        </div>
      </div>
      <p
        title={path}
        className="truncate font-mono text-ui-meta text-muted-foreground"
      >
        {path}
      </p>
      <div className="flex min-h-5 items-center gap-1.5">
        {managed.length > 0 ? (
          <>
            {visible.map((item) => (
              <span
                key={item.agentId}
                title={item.displayName}
                className="inline-flex"
              >
                <PlatformIcon agentId={item.agentId} size={16} />
                <span className="sr-only">{item.displayName}</span>
              </span>
            ))}
            {extra > 0 && (
              <span className="text-xs font-medium tabular-nums text-muted-foreground">
                {t("skillsCli.morePlatforms", { count: extra })}
              </span>
            )}
          </>
        ) : status ? (
          <span
            className={cn(
              "rounded-full border px-1.5 py-0.5 text-xs font-medium",
              statusChipClass[DENSE_STATUS_TONE[status]],
            )}
          >
            {t(`skillsCli.placement.${status}`)}
          </span>
        ) : (
          <span
            className={cn(
              "rounded-full border px-1.5 py-0.5 text-xs font-medium",
              statusChipClass.info,
            )}
          >
            {t("skillsCli.placement.missing")}
          </span>
        )}
      </div>
    </div>
  );
}
