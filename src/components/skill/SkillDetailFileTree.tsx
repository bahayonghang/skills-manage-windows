import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, FileText, FolderOpen, Link2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { DirectoryTreeEntry } from "@/types";
import { cn } from "@/lib/utils";

interface SkillDetailFileTreeProps {
  entries: DirectoryTreeEntry[];
  isLoading: boolean;
  onOpenPath: (path: string) => void;
}

function TreeNode({
  entry,
  depth,
  onOpenPath,
}: {
  entry: DirectoryTreeEntry;
  depth: number;
  onOpenPath: (path: string) => void;
}) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(depth < 1);
  const hasChildren = entry.children.length > 0;
  const isDirectory = entry.file_type === "dir" || (entry.file_type === "symlink" && hasChildren);
  const icon = isDirectory
    ? <FolderOpen className="size-3.5 shrink-0 text-muted-foreground" />
    : entry.file_type === "symlink"
      ? <Link2 className="size-3.5 shrink-0 text-muted-foreground" />
      : <FileText className="size-3.5 shrink-0 text-muted-foreground" />;

  return (
    <div className="space-y-1">
      <div
        className="flex items-start gap-1.5 rounded-md px-2 py-1 text-xs text-foreground hover:bg-muted/40"
        style={{ paddingLeft: `${depth * 14 + 8}px` }}
      >
        {hasChildren ? (
          <button
            type="button"
            onClick={() => setExpanded((value) => !value)}
            className="mt-0.5 shrink-0 rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
            aria-label={expanded ? t("common.collapse") : t("common.expand")}
          >
            {expanded ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
          </button>
        ) : (
          <span className="mt-0.5 block size-4 shrink-0" />
        )}
        <button
          type="button"
          onClick={() => onOpenPath(entry.path)}
          className="flex min-w-0 flex-1 items-start gap-1.5 text-left hover:text-primary"
          title={entry.path}
        >
          {icon}
          <span className="min-w-0 break-all leading-relaxed">{entry.name}</span>
        </button>
      </div>
      {expanded && hasChildren ? (
        <div className="space-y-1">
          {entry.children.map((child) => (
            <TreeNode
              key={child.path}
              entry={child}
              depth={depth + 1}
              onOpenPath={onOpenPath}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function SkillDetailFileTree({
  entries,
  isLoading,
  onOpenPath,
}: SkillDetailFileTreeProps) {
  const { t } = useTranslation();
  const sortedEntries = useMemo(() => entries ?? [], [entries]);

  return (
    <section aria-label={t("detail.fileTreeTitle")}>
      <div className="mb-2 text-[10px] uppercase tracking-wider text-muted-foreground/70">
        {t("detail.fileTreeTitle")}
      </div>
      <div
        className={cn(
          "rounded-lg border border-border/70 bg-muted/20 p-2",
          isLoading && "opacity-70"
        )}
      >
        {isLoading ? (
          <p className="px-2 py-1 text-xs text-muted-foreground">{t("detail.fileTreeLoading")}</p>
        ) : sortedEntries.length === 0 ? (
          <p className="px-2 py-1 text-xs text-muted-foreground">{t("detail.fileTreeEmpty")}</p>
        ) : (
          <div className="space-y-1">
            {sortedEntries.map((entry) => (
              <TreeNode
                key={entry.path}
                entry={entry}
                depth={0}
                onOpenPath={onOpenPath}
              />
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
