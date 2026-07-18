import { useTranslation } from "react-i18next";
import { FolderOpen, LayoutGrid, Link2, type LucideIcon } from "lucide-react";

import { cn } from "@/lib/utils";
import {
  PLATFORM_ORIGIN_UNASSIGNED_REPO_KEY,
  type PlatformOriginFilter,
  type PlatformOriginNavModel,
} from "@/lib/platformSkillViewModel";

interface PlatformOriginNavProps {
  model: PlatformOriginNavModel;
  value: PlatformOriginFilter;
  onChange: (filter: PlatformOriginFilter) => void;
  className?: string;
}

function OriginNavItem({
  label,
  count,
  isSelected,
  isChild,
  icon: Icon,
  title,
  onClick,
}: {
  label: string;
  count: number;
  isSelected: boolean;
  isChild?: boolean;
  icon?: LucideIcon;
  title?: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={isSelected ? "true" : undefined}
      title={title}
      className={cn(
        "focus-ring flex w-full cursor-pointer items-center gap-2 rounded-md px-2.5 py-1.5 text-sm transition-colors",
        // 子项文本与父项标题（px-2.5 + size-3.5 图标 + gap-2）左对齐
        isChild && "pl-8 text-xs",
        isSelected
          ? "bg-primary/15 font-medium text-foreground"
          : "text-muted-foreground hover:bg-muted/40",
      )}
    >
      {Icon && (
        <Icon
          aria-hidden="true"
          className={cn(
            "size-3.5 shrink-0",
            isSelected ? "text-primary" : "text-muted-foreground",
          )}
        />
      )}
      <span className="min-w-0 flex-1 truncate text-left">{label}</span>
      <span className="text-xs tabular-nums opacity-75">{count}</span>
    </button>
  );
}

export function PlatformOriginNav({
  model,
  value,
  onChange,
  className,
}: PlatformOriginNavProps) {
  const { t } = useTranslation();

  return (
    <nav aria-label={t("platform.originNav.label")} className={className}>
      <div className="flex flex-col gap-0.5">
        <OriginNavItem
          label={t("platform.originNav.all")}
          count={model.total}
          icon={LayoutGrid}
          isSelected={value.kind === "all"}
          onClick={() => onChange({ kind: "all" })}
        />
        <OriginNavItem
          label={t("platform.originNav.central")}
          count={model.centralCount}
          icon={Link2}
          isSelected={value.kind === "central" && value.repoKey === undefined}
          title={`${t("platform.sourceCentral")} - ${t("platform.sourceSymlinkLabel")}`}
          onClick={() => onChange({ kind: "central" })}
        />
        {model.repos.map((repo) => (
          <OriginNavItem
            key={repo.key}
            isChild
            label={repo.label}
            count={repo.count}
            title={repo.label}
            isSelected={value.kind === "central" && value.repoKey === repo.key}
            onClick={() => onChange({ kind: "central", repoKey: repo.key })}
          />
        ))}
        {model.unassignedCentralCount > 0 && (
          <OriginNavItem
            isChild
            label={t("platform.originNav.unassigned")}
            count={model.unassignedCentralCount}
            isSelected={
              value.kind === "central" &&
              value.repoKey === PLATFORM_ORIGIN_UNASSIGNED_REPO_KEY
            }
            onClick={() =>
              onChange({
                kind: "central",
                repoKey: PLATFORM_ORIGIN_UNASSIGNED_REPO_KEY,
              })
            }
          />
        )}
        <OriginNavItem
          label={t("platform.originNav.standalone")}
          count={model.standaloneCount}
          icon={FolderOpen}
          isSelected={value.kind === "standalone"}
          title={`${t("platform.sourceStandalone")} - ${t("platform.sourceCopyLabel")}`}
          onClick={() => onChange({ kind: "standalone" })}
        />
      </div>
    </nav>
  );
}
