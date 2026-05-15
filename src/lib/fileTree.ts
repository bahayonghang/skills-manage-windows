import type { DirectoryTreeEntry, SkillsShFileEntry } from "@/types";

function directorySortRank(entry: DirectoryTreeEntry): number {
  return entry.file_type === "dir" ? 0 : entry.file_type === "symlink" ? 1 : 2;
}

function sortDirectoryEntries(entries: DirectoryTreeEntry[]) {
  entries.sort(
    (left, right) =>
      directorySortRank(left) - directorySortRank(right) ||
      left.name.localeCompare(right.name) ||
      left.path.localeCompare(right.path)
  );
  for (const entry of entries) {
    sortDirectoryEntries(entry.children);
  }
}

export function buildDirectoryTreeFromSkillsShEntries(
  entries: SkillsShFileEntry[]
): DirectoryTreeEntry[] {
  const roots: DirectoryTreeEntry[] = [];
  const byPath = new Map<string, DirectoryTreeEntry>();

  for (const entry of entries) {
    const normalizedPath = entry.path.replace(/\\/g, "/").replace(/^\/+|\/+$/g, "");
    if (!normalizedPath) continue;

    const segments = normalizedPath.split("/").filter(Boolean);
    const name = entry.name || segments[segments.length - 1] || normalizedPath;
    const node: DirectoryTreeEntry = byPath.get(normalizedPath) ?? {
      name,
      path: normalizedPath,
      file_type: entry.is_dir ? "dir" : "file",
      symlink_target: null,
      children: [],
    };
    node.name = name;
    node.file_type = entry.is_dir ? "dir" : "file";
    byPath.set(normalizedPath, node);

    const parentPath = segments.slice(0, -1).join("/");
    if (!parentPath) {
      if (!roots.some((root) => root.path === normalizedPath)) {
        roots.push(node);
      }
      continue;
    }

    let parent = byPath.get(parentPath);
    if (!parent) {
      parent = {
        name: segments[segments.length - 2] ?? parentPath,
        path: parentPath,
        file_type: "dir",
        symlink_target: null,
        children: [],
      };
      byPath.set(parentPath, parent);
    }
    if (!parent.children.some((child) => child.path === normalizedPath)) {
      parent.children.push(node);
    }
  }

  const childPaths = new Set(
    Array.from(byPath.values()).flatMap((entry) =>
      entry.children.map((child) => child.path)
    )
  );
  const normalizedRoots = Array.from(byPath.values()).filter(
    (entry) => !childPaths.has(entry.path)
  );
  sortDirectoryEntries(normalizedRoots);
  return normalizedRoots;
}
