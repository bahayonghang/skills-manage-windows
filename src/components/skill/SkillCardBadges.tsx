import { Folder, FolderOpen, Globe, Link2, Lock } from "lucide-react";
import { useTranslation } from "react-i18next";

import { statusChipClass } from "@/lib/statusTone";
import { cn } from "@/lib/utils";
import type { ClaudeSourceKind } from "@/types";

const skillCardChipClassName =
  "inline-flex shrink-0 items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium";

export function SourceChip({ label }: { label: string }) {
  return (
    <span className="inline-flex max-w-full items-center rounded-md border border-border/60 bg-muted/35 px-1.5 py-0.5 font-mono text-xs leading-none text-muted-foreground">
      <span className="truncate">{label}</span>
    </span>
  );
}

export function SourceIndicator({ sourceType }: { sourceType: string }) {
  const { t, i18n } = useTranslation();
  const isSymlink = sourceType === "symlink";
  const isNative = sourceType === "native";
  const primaryLabel = isSymlink
    ? t("platform.sourceCentral")
    : t("platform.sourceStandalone");
  const secondaryLabel = isSymlink
    ? t("platform.sourceSymlinkLabel")
    : isNative
      ? t("platform.sourceNativeLabel", {
          defaultValue: i18n.language.startsWith("zh") ? "原生" : "native",
        })
      : t("platform.sourceCopyLabel");

  return (
    <span
      title={`${primaryLabel} — ${secondaryLabel}`}
      className={cn(
        skillCardChipClassName,
        "border",
        isSymlink
          ? statusChipClass.info
          : "border-border bg-muted/40 text-muted-foreground",
      )}
    >
      {isSymlink ? (
        <Link2 className="size-3 shrink-0" />
      ) : (
        <FolderOpen className="size-3 shrink-0" />
      )}
      {primaryLabel}
    </span>
  );
}

export function PluginSourceChip() {
  const { t, i18n } = useTranslation();

  return (
    <span
      className={cn(skillCardChipClassName, "border", statusChipClass.warning)}
    >
      <Lock className="size-3 shrink-0" />
      {t("platform.originPlugin", {
        defaultValue: i18n.language.startsWith("zh")
          ? "插件来源"
          : "Plugin source",
      })}
    </span>
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

export function ReadOnlyBadge() {
  const { t, i18n } = useTranslation();

  return (
    <span
      className={cn(
        skillCardChipClassName,
        "border border-border bg-muted/40 text-muted-foreground",
      )}
    >
      <Lock className="size-3 shrink-0" />
      {t("platform.readOnly", {
        defaultValue: i18n.language.startsWith("zh") ? "只读" : "Read-only",
      })}
    </span>
  );
}

export function ProjectSourceBadge({
  originBadge,
}: {
  originBadge: { kind: "central" | "project" | string; label: string };
}) {
  const isCentral = originBadge.kind === "central";

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium ring-1",
        isCentral
          ? "bg-info/10 text-info-foreground ring-info/20"
          : "bg-muted text-muted-foreground ring-border/70",
      )}
    >
      {isCentral ? (
        <Globe className="size-3 shrink-0" />
      ) : (
        <Folder className="size-3 shrink-0" />
      )}
      {originBadge.label}
    </span>
  );
}
