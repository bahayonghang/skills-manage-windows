import {
  Cable,
  FolderSearch,
  GitBranch,
  Info,
  Palette,
  PanelsTopLeft,
  type LucideIcon,
} from "lucide-react";

export type SettingsPageId =
  | "appearance"
  | "connections"
  | "platforms"
  | "integrations"
  | "skill-sources"
  | "about";

export interface SettingsPageDefinition {
  id: SettingsPageId;
  path: string;
  titleKey: string;
  descriptionKey: string;
  icon: LucideIcon;
  sectionIds: readonly string[];
}

export const DEFAULT_SETTINGS_PAGE_ID: SettingsPageId = "appearance";

export const SETTINGS_PAGES: readonly SettingsPageDefinition[] = [
  {
    id: "appearance",
    path: "/settings/appearance",
    titleKey: "settings.pages.appearance.title",
    descriptionKey: "settings.pages.appearance.description",
    icon: Palette,
    sectionIds: ["appearance"],
  },
  {
    id: "connections",
    path: "/settings/connections",
    titleKey: "settings.pages.connections.title",
    descriptionKey: "settings.pages.connections.description",
    icon: Cable,
    sectionIds: ["remote-targets"],
  },
  {
    id: "platforms",
    path: "/settings/platforms",
    titleKey: "settings.pages.platforms.title",
    descriptionKey: "settings.pages.platforms.description",
    icon: PanelsTopLeft,
    sectionIds: ["custom-platforms", "platform-visibility"],
  },
  {
    id: "integrations",
    path: "/settings/integrations",
    titleKey: "settings.pages.integrations.title",
    descriptionKey: "settings.pages.integrations.description",
    icon: GitBranch,
    sectionIds: ["github-pat", "ai-provider"],
  },
  {
    id: "skill-sources",
    path: "/settings/skill-sources",
    titleKey: "settings.pages.skillSources.title",
    descriptionKey: "settings.pages.skillSources.description",
    icon: FolderSearch,
    sectionIds: ["scan-directories"],
  },
  {
    id: "about",
    path: "/settings/about",
    titleKey: "settings.pages.about.title",
    descriptionKey: "settings.pages.about.description",
    icon: Info,
    sectionIds: ["about"],
  },
] as const;

const SETTINGS_PAGE_IDS = new Set<SettingsPageId>(
  SETTINGS_PAGES.map((page) => page.id)
);

const LEGACY_SECTION_TO_PAGE: Record<string, SettingsPageId> = {
  appearance: "appearance",
  "appearance-section": "appearance",
  "remote-targets": "connections",
  "remote-targets-section": "connections",
  "custom-platforms": "platforms",
  "custom-platforms-section": "platforms",
  "platform-visibility": "platforms",
  "platform-visibility-section": "platforms",
  platforms: "platforms",
  "github-pat": "integrations",
  "github-pat-section": "integrations",
  ai: "integrations",
  "ai-section": "integrations",
  "ai-provider": "integrations",
  "ai-provider-section": "integrations",
  "scan-directories": "skill-sources",
  "scan-directories-section": "skill-sources",
  "skill-sources": "skill-sources",
  "skill-sources-section": "skill-sources",
  about: "about",
  "about-section": "about",
};

export function getSettingsPageById(pageId: SettingsPageId) {
  return SETTINGS_PAGES.find((page) => page.id === pageId) ?? SETTINGS_PAGES[0];
}

export function isSettingsPageId(value: string): value is SettingsPageId {
  return SETTINGS_PAGE_IDS.has(value as SettingsPageId);
}

export function normalizeSettingsSubpath(pathname: string) {
  const raw = pathname.replace(/^\/?settings\/?/, "").split("/")[0] ?? "";
  return raw.trim();
}

export function getSettingsPagePath(pageId: SettingsPageId) {
  return getSettingsPageById(pageId).path;
}

export function resolveSettingsPageId({
  pathname,
  search,
  hash,
}: {
  pathname: string;
  search: string;
  hash: string;
}): SettingsPageId {
  const subpath = normalizeSettingsSubpath(pathname);
  if (subpath && isSettingsPageId(subpath)) {
    return subpath;
  }

  const params = new URLSearchParams(search);
  const legacySection = params.get("section")?.trim();
  if (legacySection && LEGACY_SECTION_TO_PAGE[legacySection]) {
    return LEGACY_SECTION_TO_PAGE[legacySection];
  }

  const legacyHash = hash.startsWith("#") ? hash.slice(1) : hash;
  if (legacyHash && LEGACY_SECTION_TO_PAGE[legacyHash]) {
    return LEGACY_SECTION_TO_PAGE[legacyHash];
  }

  return DEFAULT_SETTINGS_PAGE_ID;
}

export function getCanonicalSettingsPagePath(pageId: SettingsPageId) {
  return pageId === DEFAULT_SETTINGS_PAGE_ID ? "/settings" : getSettingsPagePath(pageId);
}
