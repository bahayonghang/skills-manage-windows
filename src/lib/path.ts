export function normalizePathSeparators(path: string): string {
  return path.replace(/\\/g, "/");
}

function stripWindowsExtendedPathPrefix(path: string): string {
  const normalized = normalizePathSeparators(path.trim());
  if (/^\/\/\?\/UNC\//i.test(normalized)) {
    return `//${normalized.slice("//?/UNC/".length)}`;
  }
  if (/^\/\/\?\//i.test(normalized)) {
    return normalized.slice("//?/".length);
  }
  return normalized;
}

export function arePathsEquivalent(left: string | undefined, right: string | undefined): boolean {
  if (!left || !right) {
    return false;
  }

  const normalize = (path: string) => {
    const normalized = stripWindowsExtendedPathPrefix(path).replace(/\/+$/, "");
    return isWindowsPath(path) ? normalized.toLocaleLowerCase("en-US") : normalized;
  };

  return normalize(left) === normalize(right);
}

export function isWindowsPath(path: string): boolean {
  const normalized = normalizePathSeparators(path);
  return /^[A-Za-z]:\//.test(normalized) || normalized.startsWith("//") || path.includes("\\");
}

export function formatPathForDisplay(path: string): string {
  if (!path) {
    return path;
  }
  const normalized = stripWindowsExtendedPathPrefix(path);
  return isWindowsPath(normalized) ? normalized.replace(/\//g, "\\") : normalized;
}

export function compactHomePath(path: string): string {
  const normalized = stripWindowsExtendedPathPrefix(path);
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

export function deriveHomeDir(path: string): string | undefined {
  const normalized = stripWindowsExtendedPathPrefix(path).replace(/\/+$/, "");
  if (normalized === "~" || normalized.startsWith("~/")) {
    return "~";
  }

  const homePatterns = [
    /^((?:\/Users|\/home)\/[^/]+)(?:\/.*)?$/,
    /^([A-Za-z]:\/Users\/[^/]+)(?:\/.*)?$/,
    /^(\/\/[^/]+\/[^/]+)(?:\/.*)?$/,
  ];

  for (const pattern of homePatterns) {
    const match = normalized.match(pattern);
    if (match?.[1]) {
      return formatPathForDisplay(match[1]);
    }
  }

  return undefined;
}

export function joinPathForDisplay(basePath: string, relativePath: string): string {
  const normalizedBase = normalizePathSeparators(basePath).replace(/\/+$/, "");
  const normalizedRelative = normalizePathSeparators(relativePath).replace(/^\/+/, "");

  if (!normalizedBase) {
    return formatPathForDisplay(normalizedRelative);
  }

  return formatPathForDisplay(`${normalizedBase}/${normalizedRelative}`);
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

export function getPlatformPathHint(path: string): string {
  return compactHomePath(path);
}
