import { useEffect, useMemo, useRef, useState } from "react";
import {
  AlertCircle,
  ChevronDown,
  ChevronRight,
  FileText,
  Folder,
  FolderOpen,
  FolderTree,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { VirtualizedList } from "@/components/ui/virtualized-list";
import {
  buildGitHubImportFileTree,
  flattenGitHubImportFileTree,
  getGitHubImportFileManifestIssue,
} from "@/lib/githubImportFileTree";
import type { GitHubSkillPreviewFile } from "@/types";

interface GitHubImportFileTreeProps {
  files: GitHubSkillPreviewFile[] | null | undefined;
  rootName: string;
}

function formatByteSize(bytes: number, locale: string): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(value)} ${units[unitIndex]}`;
}

export function GitHubImportFileTree({
  files,
  rootName,
}: GitHubImportFileTreeProps) {
  const { t, i18n } = useTranslation();
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const issue = getGitHubImportFileManifestIssue(files);
  const model = useMemo(
    () => (issue ? null : buildGitHubImportFileTree(files ?? [])),
    [files, issue],
  );
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(
    () => new Set(model?.defaultExpandedPaths ?? []),
  );

  useEffect(() => {
    setExpandedPaths(new Set(model?.defaultExpandedPaths ?? []));
    scrollRef.current?.scrollTo?.({ top: 0, behavior: "auto" });
  }, [model]);

  const visibleRows = useMemo(
    () =>
      model
        ? flattenGitHubImportFileTree(model.roots, expandedPaths)
        : [],
    [expandedPaths, model],
  );

  if (!model) {
    return (
      <div
        className="flex h-full items-center justify-center px-4 py-10"
        data-testid="github-import-file-tree-error"
        role="alert"
      >
        <div className="flex max-w-md items-start gap-2 text-sm text-destructive-text">
          <AlertCircle className="mt-0.5 size-4 shrink-0" />
          <span>{t("marketplace.githubImportFileManifestError")}</span>
        </div>
      </div>
    );
  }

  const toggleDirectory = (path: string) => {
    setExpandedPaths((current) => {
      const next = new Set(current);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  return (
    <section
      aria-label={t("marketplace.githubImportDetailTabs.files")}
      className="flex h-full min-h-0 flex-col"
      data-testid="github-import-file-tree"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-3 text-ui-meta text-muted-foreground">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 tabular-nums">
          <span>
            {model.fileCount === 1
              ? t("marketplace.githubImportFileCountOne")
              : t("marketplace.githubImportFileCountMany", {
                  count: model.fileCount,
                })}
          </span>
          <span aria-hidden="true">·</span>
          <span>
            {model.directoryCount === 1
              ? t("marketplace.githubImportDirectoryCountOne")
              : t("marketplace.githubImportDirectoryCountMany", {
                  count: model.directoryCount,
                })}
          </span>
          <span aria-hidden="true">·</span>
          <span>{formatByteSize(model.totalByteLen, i18n.language)}</span>
        </div>
        <span>{t("marketplace.githubImportFileSnapshotHint")}</span>
      </div>

      <div className="flex h-9 shrink-0 items-center gap-2 border-b border-border/60 px-2 text-xs font-semibold">
        <FolderTree className="size-4 shrink-0 text-primary" />
        <span className="min-w-0 truncate" title={rootName}>
          {rootName}
        </span>
      </div>

      <div
        ref={scrollRef}
        className="min-h-0 flex-1 overflow-y-auto py-1"
        data-testid="github-import-file-tree-scroll"
      >
        <VirtualizedList
          items={visibleRows}
          itemHeight={32}
          fallbackHeight={384}
          scrollContainerRef={scrollRef}
          itemKey={(row) => row.node.path}
          renderItem={({ node, depth }) => {
            const isDirectory = node.kind === "directory";
            const expanded = isDirectory && expandedPaths.has(node.path);
            const content = (
              <>
                {isDirectory ? (
                  expanded ? (
                    <ChevronDown className="size-3.5 shrink-0" />
                  ) : (
                    <ChevronRight className="size-3.5 shrink-0" />
                  )
                ) : (
                  <span className="size-3.5 shrink-0" />
                )}
                {isDirectory ? (
                  expanded ? (
                    <FolderOpen className="size-4 shrink-0 text-muted-foreground" />
                  ) : (
                    <Folder className="size-4 shrink-0 text-muted-foreground" />
                  )
                ) : (
                  <FileText className="size-4 shrink-0 text-muted-foreground" />
                )}
                <span className="min-w-0 flex-1 truncate" title={node.path}>
                  {node.name}
                </span>
                <span className="shrink-0 pl-2 text-ui-micro tabular-nums text-muted-foreground">
                  {isDirectory
                    ? node.descendantFileCount === 1
                      ? t("marketplace.githubImportDirectoryFileCountOne")
                      : t("marketplace.githubImportDirectoryFileCountMany", {
                          count: node.descendantFileCount,
                        })
                    : formatByteSize(node.byteLen, i18n.language)}
                </span>
              </>
            );

            return isDirectory ? (
              <button
                type="button"
                className="flex h-8 w-full items-center gap-1.5 rounded-md pr-2 text-left text-xs text-foreground outline-none hover:bg-muted/40 focus-visible:ring-2 focus-visible:ring-ring"
                style={{ paddingLeft: `${depth * 16 + 8}px` }}
                aria-expanded={expanded}
                aria-label={
                  expanded
                    ? t("marketplace.githubImportCollapseDirectory", {
                        name: node.name,
                      })
                    : t("marketplace.githubImportExpandDirectory", {
                        name: node.name,
                      })
                }
                data-testid="github-import-file-tree-directory"
                onClick={() => toggleDirectory(node.path)}
              >
                {content}
              </button>
            ) : (
              <div
                className="flex h-8 items-center gap-1.5 pr-2 text-xs text-foreground"
                style={{ paddingLeft: `${depth * 16 + 8}px` }}
                data-testid="github-import-file-tree-file"
              >
                {content}
              </div>
            );
          }}
        />
      </div>
    </section>
  );
}
