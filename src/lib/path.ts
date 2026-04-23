export function normalizePathSeparators(path: string): string {
  return path.replace(/\\/g, "/");
}

export function compactHomePath(path: string): string {
  const normalized = normalizePathSeparators(path);
  if (normalized === "~") {
    return "~";
  }
  if (normalized.startsWith("~/")) {
    return normalized;
  }

  const homePatterns = [
    /^\/Users\/[^/]+\/?(.*)$/,
    /^\/home\/[^/]+\/?(.*)$/,
    /^[A-Za-z]:\/Users\/[^/]+\/?(.*)$/,
  ];

  for (const pattern of homePatterns) {
    const match = normalized.match(pattern);
    if (!match) {
      continue;
    }

    const rest = match[1];
    return rest ? `~/${rest}` : "~";
  }

  return normalized;
}

export function getPathBasename(path: string): string | undefined {
  const normalized = normalizePathSeparators(path).replace(/\/+$/, "");
  if (!normalized || normalized === "~") {
    return undefined;
  }

  const parts = normalized.split("/");
  return parts[parts.length - 1] || undefined;
}

export function describeSkillsPattern(path: string): string {
  const compact = compactHomePath(path);
  return compact.startsWith("~/") ? compact.slice(2) : compact;
}
