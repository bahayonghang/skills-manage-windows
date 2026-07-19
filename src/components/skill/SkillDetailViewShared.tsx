import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Bot, Check, Code, FileText, Lock } from "lucide-react";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { cn } from "@/lib/utils";
import {
  getPlatformTargetLabel,
  getPlatformTargetMemberNames,
} from "@/lib/platformTargetGroups";
import type { AgentWithStatus, ClaudeSourceKind } from "@/types";
import type { PreviewTab } from "./skillDetailViewTypes";

export function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <div className="mb-2 text-xs font-semibold uppercase tracking-[0.12em] text-muted-foreground">
      {children}
    </div>
  );
}

export function MetadataRow({
  label,
  value,
  icon,
}: {
  label: string;
  value: string;
  icon?: ReactNode;
}) {
  return (
    <div className="space-y-0.5">
      <div className="text-xs font-medium uppercase tracking-[0.12em] text-muted-foreground">
        {label}
      </div>
      <div className="flex min-w-0 items-start gap-1.5 break-all font-mono text-xs leading-relaxed text-foreground">
        {icon}
        <span className="min-w-0 break-all">{value}</span>
      </div>
    </div>
  );
}

export function SourceOriginBadge({
  originKind,
}: {
  originKind: ClaudeSourceKind;
}) {
  const { t, i18n } = useTranslation();
  const isPlugin = originKind === "plugin";

  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ring-1",
        isPlugin
          ? "bg-warning/10 text-warning-foreground ring-warning/20"
          : "bg-info/10 text-info-foreground ring-info/20",
      )}
    >
      {isPlugin
        ? t("platform.originPlugin", {
            defaultValue: i18n.language.startsWith("zh")
              ? "插件来源"
              : "Plugin source",
          })
        : t("platform.originUser", {
            defaultValue: i18n.language.startsWith("zh")
              ? "用户来源"
              : "User source",
          })}
    </span>
  );
}

export function ReadOnlySourceBadge() {
  const { t, i18n } = useTranslation();

  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground ring-1 ring-border/70">
      <Lock className="size-3 shrink-0" />
      {t("detail.readOnlySource", {
        defaultValue: i18n.language.startsWith("zh")
          ? "只读来源"
          : "Read-only source",
      })}
    </span>
  );
}

interface PlatformToggleIconProps {
  agent: AgentWithStatus;
  skillName: string;
  isInstalled: boolean;
  isLoading: boolean;
  isLocked: boolean;
  onToggle: () => void;
}

export function PlatformToggleIcon({
  agent,
  skillName,
  isInstalled,
  isLoading,
  isLocked,
  onToggle,
}: PlatformToggleIconProps) {
  const { t } = useTranslation();
  const displayName = getPlatformTargetLabel(agent, t, "short");
  const memberNames = getPlatformTargetMemberNames(agent).join(", ");
  const title = isLocked
    ? `${displayName} - ${t("platformTargets.alwaysIncluded")} - ${memberNames}`
    : `${displayName}${isInstalled ? ` - ${t("central.linked")}` : ""}`;

  return (
    <button
      className={cn(
        "relative cursor-pointer rounded-md p-1.5 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background",
        isLocked
          ? "cursor-default text-primary-text"
          : isInstalled
            ? "text-primary-text hover:bg-primary/15"
            : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
        isLoading && "pointer-events-none animate-pulse",
      )}
      title={title}
      data-install-state={isLocked ? "locked" : isInstalled ? "installed" : "uninstalled"}
      aria-pressed={isInstalled}
      aria-busy={isLoading}
      aria-label={
        isLocked
          ? title
          : t("central.toggleInstallLabel", {
              platform: displayName,
              skill: skillName,
            })
      }
      disabled={isLoading || isLocked}
      onClick={onToggle}
    >
      <PlatformIcon agentId={agent.id} className="size-4 shrink-0" size={16} />
      {isLocked ? (
        <Lock
          aria-hidden="true"
          className="absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full bg-background text-primary-text"
        />
      ) : isInstalled ? (
        <Check
          aria-hidden="true"
          className="absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full bg-background text-success-foreground"
        />
      ) : null}
    </button>
  );
}

interface TabToggleProps {
  activeTab: PreviewTab;
  onChange: (tab: PreviewTab) => void;
}

export function TabToggle({ activeTab, onChange }: TabToggleProps) {
  const { t } = useTranslation();

  return (
    <div className="flex gap-0.5 rounded-lg border border-border bg-muted/40 p-0.5">
      <button
        role="tab"
        aria-selected={activeTab === "markdown"}
        onClick={() => onChange("markdown")}
        className={cn(
          "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
          activeTab === "markdown"
            ? "bg-background text-foreground shadow-sm"
            : "text-muted-foreground hover:text-foreground",
        )}
      >
        <FileText className="size-3.5" />
        {t("detail.markdown")}
      </button>
      <button
        role="tab"
        aria-selected={activeTab === "raw"}
        onClick={() => onChange("raw")}
        className={cn(
          "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
          activeTab === "raw"
            ? "bg-background text-foreground shadow-sm"
            : "text-muted-foreground hover:text-foreground",
        )}
      >
        <Code className="size-3.5" />
        {t("detail.rawSource")}
      </button>
      <button
        role="tab"
        aria-selected={activeTab === "explanation"}
        onClick={() => onChange("explanation")}
        className={cn(
          "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
          activeTab === "explanation"
            ? "bg-background text-foreground shadow-sm"
            : "text-muted-foreground hover:text-foreground",
        )}
      >
        <Bot className="size-3.5" />
        {t("detail.aiExplanation")}
      </button>
    </div>
  );
}

export const inspectorCardClassName =
  "min-w-0 space-y-3 overflow-x-hidden rounded-lg bg-muted/20 p-3 ring-1 ring-border/70";
export const inspectorFieldLabelClassName =
  "text-xs font-medium uppercase tracking-[0.12em] text-muted-foreground";
export const inspectorSelectClassName =
  "w-full min-w-0 rounded-md border border-input bg-background px-2 py-1.5 text-xs focus-visible:border-ring focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50";
export const inspectorActionButtonClassName = "w-full justify-center";
