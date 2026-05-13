import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Bot, Code, FileText, Lock } from "lucide-react";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { cn } from "@/lib/utils";
import {
  getPlatformTargetMemberNames,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";
import type { AgentWithStatus, ClaudeSourceKind } from "@/types";
import type { PreviewTab } from "./skillDetailViewTypes";

export function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <div className="mb-2 text-[10px] font-bold uppercase tracking-widest text-muted-foreground/80">
      {children}
    </div>
  );
}

export function MetadataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="space-y-0.5">
      <div className="text-[10px] uppercase tracking-wider text-muted-foreground/70">{label}</div>
      <div className="break-all font-mono text-xs leading-relaxed text-foreground">{value}</div>
    </div>
  );
}

export function SourceOriginBadge({ originKind }: { originKind: ClaudeSourceKind }) {
  const { t, i18n } = useTranslation();
  const isPlugin = originKind === "plugin";

  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium ring-1",
        isPlugin
          ? "bg-amber-500/10 text-amber-700 ring-amber-500/20 dark:text-amber-300"
          : "bg-sky-500/10 text-sky-700 ring-sky-500/20 dark:text-sky-300"
      )}
    >
      {isPlugin
        ? t("platform.originPlugin", {
            defaultValue: i18n.language.startsWith("zh") ? "插件来源" : "Plugin source",
          })
        : t("platform.originUser", {
            defaultValue: i18n.language.startsWith("zh") ? "用户来源" : "User source",
          })}
    </span>
  );
}

export function ReadOnlySourceBadge() {
  const { t, i18n } = useTranslation();

  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground ring-1 ring-border/70">
      <Lock className="size-3 shrink-0" />
      {t("detail.readOnlySource", {
        defaultValue: i18n.language.startsWith("zh") ? "只读来源" : "Read-only source",
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
  const displayName = isUniversalPlatformTarget(agent)
    ? t("platformTargets.universalShortLabel")
    : agent.display_name;
  const memberNames = getPlatformTargetMemberNames(agent).join(", ");
  const title = isLocked
    ? `${displayName} - ${t("platformTargets.alwaysIncluded")} - ${memberNames}`
    : `${displayName}${isInstalled ? ` - ${t("central.linked")}` : ""}`;

  return (
    <button
      className={cn(
        "cursor-pointer rounded-md p-1.5 transition-colors",
        isLocked
          ? "cursor-default text-primary"
          : isInstalled
            ? "text-primary hover:bg-primary/15"
            : "text-muted-foreground/40 hover:bg-muted/60 hover:text-muted-foreground",
        isLoading && "pointer-events-none animate-pulse"
      )}
      title={title}
      aria-label={
        isLocked
          ? title
          : t("central.toggleInstallLabel", { platform: displayName, skill: skillName })
      }
      disabled={isLoading || isLocked}
      onClick={onToggle}
    >
      <PlatformIcon agentId={agent.id} className="size-4 shrink-0" size={16} />
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
            : "text-muted-foreground hover:text-foreground"
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
            : "text-muted-foreground hover:text-foreground"
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
            : "text-muted-foreground hover:text-foreground"
        )}
      >
        <Bot className="size-3.5" />
        {t("detail.aiExplanation")}
      </button>
    </div>
  );
}

export const inspectorCardClassName =
  "min-w-0 space-y-3 overflow-x-hidden rounded-lg border border-border/70 bg-muted/20 p-3";
export const inspectorFieldLabelClassName =
  "text-[10px] font-medium uppercase tracking-wider text-muted-foreground/70";
export const inspectorSelectClassName =
  "w-full min-w-0 rounded-md border border-border bg-background px-2 py-1.5 text-xs";
export const inspectorActionButtonClassName = "w-full justify-center";
