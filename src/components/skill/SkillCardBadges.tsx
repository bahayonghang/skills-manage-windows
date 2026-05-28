import { Folder, FolderOpen, Globe, Link2, Lock } from "lucide-react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import type { ClaudeSourceKind } from "@/types";

export function SourceChip({ label }: { label: string }) {
  return (
    <span className="inline-flex max-w-full items-center rounded-md border border-border/60 bg-muted/35 px-1.5 py-0.5 font-mono text-[10px] leading-none text-muted-foreground/80">
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
    <div
      className={cn(
        "inline-flex items-center gap-1 text-xs font-medium",
        isSymlink ? "text-primary/80" : "text-muted-foreground",
      )}
    >
      {isSymlink ? (
        <Link2 className="size-3 shrink-0" />
      ) : (
        <FolderOpen className="size-3 shrink-0" />
      )}
      <div className="inline-flex items-center gap-1">
        <span>{primaryLabel}</span>
        <span
          aria-hidden="true"
          className="h-px w-3 shrink-0 rounded-full bg-current opacity-40"
        />
        <span className="sr-only"> - </span>
        <span>{secondaryLabel}</span>
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
        "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium ring-1",
        isPlugin
          ? "bg-amber-500/10 text-amber-700 ring-amber-500/20 dark:text-amber-300"
          : "bg-sky-500/10 text-sky-700 ring-sky-500/20 dark:text-sky-300",
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
    <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground ring-1 ring-border/70">
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
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium ring-1",
        isCentral
          ? "bg-sky-500/10 text-sky-700 ring-sky-500/20 dark:text-sky-300"
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
