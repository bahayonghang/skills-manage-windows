/**
 * Minimal file-manifest entry the read-only import tree needs.
 *
 * Both GitHub preview manifests (which additionally carry `sha256`) and local
 * archive manifests satisfy this shape, so the tree stays source-agnostic.
 */
export interface GitHubImportFileEntry {
  path: string;
  byteLen: number;
}

export type GitHubImportFileManifestIssue =
  | "missing"
  | "empty"
  | "invalid-path"
  | "invalid-size"
  | "duplicate-path"
  | "path-conflict"
  | "missing-skill-markdown";

export interface GitHubImportFileTreeNode {
  name: string;
  path: string;
  kind: "directory" | "file";
  byteLen: number;
  descendantFileCount: number;
  children: GitHubImportFileTreeNode[];
}

export interface GitHubImportFileTreeModel {
  roots: GitHubImportFileTreeNode[];
  fileCount: number;
  directoryCount: number;
  totalByteLen: number;
  defaultExpandedPaths: string[];
}

export interface GitHubImportFileTreeRow {
  node: GitHubImportFileTreeNode;
  depth: number;
}

function pathSegments(path: string): string[] | null {
  if (
    !path ||
    path.startsWith("/") ||
    path.endsWith("/") ||
    path.includes("\\")
  ) {
    return null;
  }
  const segments = path.split("/");
  return segments.some(
    (segment) => !segment || segment === "." || segment === "..",
  )
    ? null
    : segments;
}

export function getGitHubImportFileManifestIssue(
  files: GitHubImportFileEntry[] | null | undefined,
): GitHubImportFileManifestIssue | null {
  if (!Array.isArray(files)) return "missing";
  if (files.length === 0) return "empty";

  const filePaths = new Set<string>();
  const directoryPaths = new Set<string>();
  let hasSkillMarkdown = false;

  for (const file of files) {
    const segments = pathSegments(file.path);
    if (!segments) return "invalid-path";
    if (
      !Number.isSafeInteger(file.byteLen) ||
      file.byteLen < 0
    ) {
      return "invalid-size";
    }
    if (filePaths.has(file.path)) return "duplicate-path";
    filePaths.add(file.path);
    hasSkillMarkdown ||= file.path === "SKILL.md";

    for (let index = 1; index < segments.length; index += 1) {
      directoryPaths.add(segments.slice(0, index).join("/"));
    }
  }

  if ([...filePaths].some((path) => directoryPaths.has(path))) {
    return "path-conflict";
  }
  return hasSkillMarkdown ? null : "missing-skill-markdown";
}

function sortTree(nodes: GitHubImportFileTreeNode[]): void {
  nodes.sort(
    (left, right) =>
      Number(left.kind === "file") - Number(right.kind === "file") ||
      left.name.localeCompare(right.name) ||
      left.path.localeCompare(right.path),
  );
  nodes.forEach((node) => sortTree(node.children));
}

function aggregateDirectory(node: GitHubImportFileTreeNode): void {
  if (node.kind === "file") return;
  node.byteLen = 0;
  node.descendantFileCount = 0;
  for (const child of node.children) {
    aggregateDirectory(child);
    node.byteLen += child.byteLen;
    node.descendantFileCount += child.descendantFileCount;
  }
}

export function buildGitHubImportFileTree(
  files: GitHubImportFileEntry[],
): GitHubImportFileTreeModel {
  const issue = getGitHubImportFileManifestIssue(files);
  if (issue) {
    throw new Error(`Invalid GitHub import file manifest: ${issue}`);
  }

  const roots: GitHubImportFileTreeNode[] = [];
  const directories = new Map<string, GitHubImportFileTreeNode>();
  let totalByteLen = 0;

  for (const file of files) {
    const segments = pathSegments(file.path) as string[];
    let parentChildren = roots;

    for (let index = 0; index < segments.length - 1; index += 1) {
      const directoryPath = segments.slice(0, index + 1).join("/");
      let directory = directories.get(directoryPath);
      if (!directory) {
        directory = {
          name: segments[index],
          path: directoryPath,
          kind: "directory",
          byteLen: 0,
          descendantFileCount: 0,
          children: [],
        };
        directories.set(directoryPath, directory);
        parentChildren.push(directory);
      }
      parentChildren = directory.children;
    }

    parentChildren.push({
      name: segments[segments.length - 1],
      path: file.path,
      kind: "file",
      byteLen: file.byteLen,
      descendantFileCount: 1,
      children: [],
    });
    totalByteLen += file.byteLen;
  }

  sortTree(roots);
  roots.forEach(aggregateDirectory);

  return {
    roots,
    fileCount: files.length,
    directoryCount: directories.size,
    totalByteLen,
    defaultExpandedPaths: roots
      .filter((node) => node.kind === "directory")
      .map((node) => node.path),
  };
}

export function flattenGitHubImportFileTree(
  nodes: GitHubImportFileTreeNode[],
  expandedPaths: ReadonlySet<string>,
  depth = 0,
): GitHubImportFileTreeRow[] {
  const rows: GitHubImportFileTreeRow[] = [];
  for (const node of nodes) {
    rows.push({ node, depth });
    if (
      node.kind === "directory" &&
      expandedPaths.has(node.path) &&
      node.children.length > 0
    ) {
      rows.push(
        ...flattenGitHubImportFileTree(
          node.children,
          expandedPaths,
          depth + 1,
        ),
      );
    }
  }
  return rows;
}
