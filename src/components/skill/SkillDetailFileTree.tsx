import { useMemo, useState } from "react";
import {
  Braces,
  ChevronDown,
  ChevronRight,
  ExternalLink,
  File,
  FileCode2,
  FileText,
  FlaskConical,
  Folder,
  FolderOpen,
  Image as ImageIcon,
  Link2,
  Package,
  Settings2,
  type LucideIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type { DirectoryTreeEntry } from "@/types";
import { cn } from "@/lib/utils";

interface SkillDetailFileTreeProps {
  entries: DirectoryTreeEntry[];
  isLoading: boolean;
  onOpenPath: (path: string) => void;
}

type FileKind =
  | "directory"
  | "symlink"
  | "docs"
  | "data"
  | "web"
  | "python"
  | "rust"
  | "image"
  | "config"
  | "test"
  | "unknown";

interface FilePresentation {
  kind: FileKind;
  icon: LucideIcon;
  iconClassName: string;
}

const DOC_EXTENSIONS = new Set(["md", "mdx", "txt", "rst"]);
const DATA_EXTENSIONS = new Set(["json", "jsonc", "yaml", "yml", "toml", "xml"]);
const WEB_EXTENSIONS = new Set(["ts", "tsx", "js", "jsx", "mjs", "cjs", "css", "html"]);
const PYTHON_EXTENSIONS = new Set(["py", "pyi"]);
const IMAGE_EXTENSIONS = new Set([
  "avif",
  "bmp",
  "gif",
  "ico",
  "jpeg",
  "jpg",
  "png",
  "svg",
  "webp",
]);
const CONFIG_NAMES = new Set([
  "biome.json",
  "eslint.config.js",
  "eslint.config.mjs",
  "package.json",
  "pnpm-lock.yaml",
  "prettier.config.js",
  "tsconfig.json",
  "vite.config.ts",
]);

function classifyFile(entry: DirectoryTreeEntry): FilePresentation {
  const name = entry.name.toLowerCase();
  const extension = name.includes(".") ? name.split(".").pop() ?? "" : "";
  const hasChildren = entry.children.length > 0;

  if (entry.file_type === "dir" || (entry.file_type === "symlink" && hasChildren)) {
    return { kind: "directory", icon: Folder, iconClassName: "text-primary-text" };
  }
  if (entry.file_type === "symlink") {
    return { kind: "symlink", icon: Link2, iconClassName: "text-info-foreground" };
  }
  if (/(^|\.)(test|spec)\.[^.]+$/.test(name) || name.includes("__tests__")) {
    return { kind: "test", icon: FlaskConical, iconClassName: "text-success-foreground" };
  }
  if (name === "cargo.toml" || name === "cargo.lock") {
    return { kind: "rust", icon: Package, iconClassName: "text-warning-foreground" };
  }
  if (name.startsWith(".env") || CONFIG_NAMES.has(name) || name.startsWith(".")) {
    return { kind: "config", icon: Settings2, iconClassName: "text-info-foreground" };
  }
  if (DOC_EXTENSIONS.has(extension) || name.startsWith("readme") || name === "skill.md") {
    return { kind: "docs", icon: FileText, iconClassName: "text-primary-text" };
  }
  if (DATA_EXTENSIONS.has(extension)) {
    return { kind: "data", icon: Braces, iconClassName: "text-warning-foreground" };
  }
  if (PYTHON_EXTENSIONS.has(extension)) {
    return { kind: "python", icon: FileCode2, iconClassName: "text-success-foreground" };
  }
  if (extension === "rs") {
    return { kind: "rust", icon: FileCode2, iconClassName: "text-warning-foreground" };
  }
  if (WEB_EXTENSIONS.has(extension)) {
    return { kind: "web", icon: FileCode2, iconClassName: "text-info-foreground" };
  }
  if (IMAGE_EXTENSIONS.has(extension)) {
    return { kind: "image", icon: ImageIcon, iconClassName: "text-success-foreground" };
  }
  return { kind: "unknown", icon: File, iconClassName: "text-muted-foreground" };
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
  const [expanded, setExpanded] = useState(false);
  const hasChildren = entry.children.length > 0;
  const presentation = classifyFile(entry);
  const isDirectory = presentation.kind === "directory";
  const EntryIcon = isDirectory && expanded ? FolderOpen : presentation.icon;

  return (
    <div className="space-y-1">
      <div
        data-testid={`file-tree-entry-${entry.name}`}
        data-file-kind={presentation.kind}
        className="flex min-w-0 items-start gap-1 rounded-md py-1 pr-1 text-xs text-foreground hover:bg-muted/40"
        style={{ paddingLeft: `${depth * 14 + 6}px` }}
      >
        {isDirectory ? (
          <button
            type="button"
            onClick={() => setExpanded((value) => !value)}
            className="flex min-w-0 flex-1 items-start gap-1.5 rounded px-1 py-0.5 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background"
            aria-label={t(expanded ? "detail.collapseDirectory" : "detail.expandDirectory", {
              name: entry.name,
            })}
            aria-expanded={expanded}
          >
            {expanded ? (
              <ChevronDown className="mt-0.5 size-3 shrink-0 text-muted-foreground" />
            ) : (
              <ChevronRight className="mt-0.5 size-3 shrink-0 text-muted-foreground" />
            )}
            <EntryIcon className={cn("mt-0.5 size-3.5 shrink-0", presentation.iconClassName)} />
            <span className="min-w-0 break-all leading-relaxed">{entry.name}</span>
          </button>
        ) : (
          <button
            type="button"
            onClick={() => onOpenPath(entry.path)}
            className="flex min-w-0 flex-1 items-start gap-1.5 rounded px-1 py-0.5 text-left hover:text-primary-text focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background"
            title={entry.path}
            aria-label={t("detail.openTreePath", { name: entry.name })}
          >
            <span className="mt-0.5 block size-3 shrink-0" aria-hidden="true" />
            <EntryIcon className={cn("mt-0.5 size-3.5 shrink-0", presentation.iconClassName)} />
            <span className="min-w-0 break-all leading-relaxed">{entry.name}</span>
          </button>
        )}
        {isDirectory ? (
          <button
            type="button"
            onClick={() => onOpenPath(entry.path)}
            className="mt-0.5 shrink-0 rounded p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            title={entry.path}
            aria-label={t("detail.openTreePath", { name: entry.name })}
          >
            <ExternalLink className="size-3" />
          </button>
        ) : null}
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
  const sortedEntries = useMemo(
    () => [...(entries ?? [])].sort((left, right) => {
      const leftDirectory = classifyFile(left).kind === "directory";
      const rightDirectory = classifyFile(right).kind === "directory";
      if (leftDirectory !== rightDirectory) return leftDirectory ? -1 : 1;
      return left.name.localeCompare(right.name, undefined, { sensitivity: "base" });
    }),
    [entries]
  );

  return (
    <section aria-label={t("detail.fileTreeTitle")} aria-busy={isLoading}>
      <div className="mb-2 text-[0.72rem] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
        {t("detail.fileTreeTitle")}
      </div>
      <div className={cn("min-w-0", isLoading && "opacity-70")}>
        {isLoading ? (
          <p className="px-1 py-1 text-xs text-muted-foreground">{t("detail.fileTreeLoading")}</p>
        ) : sortedEntries.length === 0 ? (
          <p className="px-1 py-1 text-xs text-muted-foreground">{t("detail.fileTreeEmpty")}</p>
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
