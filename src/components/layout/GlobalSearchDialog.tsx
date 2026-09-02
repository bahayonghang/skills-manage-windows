import {
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useState,
} from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Blocks, Layers, LayoutDashboard, RefreshCw, Plus } from "lucide-react";

import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Button } from "@/components/ui/button";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useCollectionStore } from "@/stores/collectionStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useHotkey } from "@/hooks/useHotkey";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { formatBackendError } from "@/lib/backendError";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  getPlatformTargetLabel,
  getPlatformTargetMemberNames,
  getPlatformTargetPathHint,
} from "@/lib/platformTargetGroups";
import {
  buildSearchText,
  normalizeSearchQuery,
  scoreSearchMatch,
} from "@/lib/search";

interface GlobalSearchDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAction: (action: string) => void;
}

type SearchItem = {
  id: string;
  label: string;
  description?: string;
  groupKey: "central" | "collections" | "platforms" | "actions";
  groupLabel: string;
  icon: React.ReactNode;
  searchText: string;
  labelText: string;
  descriptionText: string;
  onSelect: () => void;
};

type ListSourceStatus = "loading" | "error" | "empty" | "ready";

type VisibleGroup = {
  key: SearchItem["groupKey"];
  heading: string;
  items: SearchItem[];
  status?: ListSourceStatus;
  error?: string | null;
  onRetry?: () => void;
};

function listSourceStatus(params: {
  hasLoaded: boolean;
  isLoading: boolean;
  error: string | null;
  totalCount: number;
}): ListSourceStatus {
  if (!params.hasLoaded) {
    if (params.error && !params.isLoading) {
      return "error";
    }
    return "loading";
  }
  if (params.totalCount === 0) {
    return "empty";
  }
  return "ready";
}

function SearchSourceStatus({
  status,
  error,
  onRetry,
}: {
  status: ListSourceStatus;
  error: string | null;
  onRetry: () => void;
}) {
  const { t } = useTranslation();

  switch (status) {
    case "loading":
      return (
        <div
          role="status"
          className="px-2 py-3 text-center text-sm text-muted-foreground"
        >
          {t("globalSearch.loading")}
        </div>
      );
    case "error":
      return (
        <div role="alert" className="space-y-2 px-2 py-3">
          <p className="text-sm text-destructive-text">
            {t("globalSearch.loadError", {
              error: formatBackendError(error, t),
            })}
          </p>
          <Button type="button" variant="outline" size="sm" onClick={onRetry}>
            {t("globalSearch.retry")}
          </Button>
        </div>
      );
    case "empty":
      return (
        <p className="px-2 py-3 text-center text-sm text-muted-foreground">
          {t("globalSearch.empty")}
        </p>
      );
    case "ready":
      return null;
    default: {
      const _exhaustive: never = status;
      return _exhaustive;
    }
  }
}

export function GlobalSearchDialog({
  open,
  onOpenChange,
  onAction,
}: GlobalSearchDialogProps) {
  const navigate = useNavigate();
  const { t } = useTranslation();

  const centralSkills = useCentralSkillsStore((s) => s.skills);
  const centralHasLoaded = useCentralSkillsStore((s) => s.hasLoaded);
  const centralLoading = useCentralSkillsStore((s) => s.isLoading);
  const centralError = useCentralSkillsStore((s) => s.error);
  const loadCentralSkills = useCentralSkillsStore((s) => s.loadCentralSkills);

  const collections = useCollectionStore((s) => s.collections);
  const collectionsHasLoaded = useCollectionStore((s) => s.hasLoaded);
  const collectionsLoading = useCollectionStore((s) => s.isLoading);
  const collectionsError = useCollectionStore((s) => s.error);
  const loadCollections = useCollectionStore((s) => s.loadCollections);

  const agents = usePlatformStore((s) => s.agents);
  const categoryVisibility =
    usePlatformStore((s) => s.categoryVisibility) ??
    DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const [query, setQuery] = useState("");
  const deferredQuery = useDeferredValue(query);
  const normalizedQuery = useMemo(
    () => normalizeSearchQuery(deferredQuery),
    [deferredQuery],
  );

  const close = useCallback(() => onOpenChange(false), [onOpenChange]);
  const retryCollections = useCallback(() => {
    void loadCollections();
  }, [loadCollections]);
  const retryCentral = useCallback(() => {
    void loadCentralSkills();
  }, [loadCentralSkills]);

  const groupMeta = useMemo(
    () => [
      {
        key: "central" as const,
        label: t("globalSearch.centralSkills"),
        initialLimit: 8,
      },
      {
        key: "collections" as const,
        label: t("globalSearch.collections"),
        initialLimit: 8,
      },
      {
        key: "platforms" as const,
        label: t("globalSearch.platforms"),
        initialLimit: 10,
      },
      {
        key: "actions" as const,
        label: t("globalSearch.actions"),
        initialLimit: 10,
      },
    ],
    [t],
  );

  useEffect(() => {
    if (!open) {
      setQuery("");
    }
  }, [open]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const collectionsState = useCollectionStore.getState();
    if (!collectionsState.hasLoaded && !collectionsState.isLoading) {
      void collectionsState.loadCollections();
    }
    const centralState = useCentralSkillsStore.getState();
    if (!centralState.hasLoaded && !centralState.isLoading) {
      void centralState.loadCentralSkills();
    }
  }, [open]);

  const centralStatus = listSourceStatus({
    hasLoaded: centralHasLoaded,
    isLoading: centralLoading,
    error: centralError,
    totalCount: centralSkills.length,
  });
  const collectionsStatus = listSourceStatus({
    hasLoaded: collectionsHasLoaded,
    isLoading: collectionsLoading,
    error: collectionsError,
    totalCount: collections.length,
  });

  // Build flat search items
  const items = useMemo<SearchItem[]>(() => {
    if (!open) return [];

    const result: SearchItem[] = [];

    // Central Skills
    for (const skill of centralSkills) {
      const labelText = skill.name.toLowerCase();
      const descriptionText = (skill.description ?? "").toLowerCase();
      result.push({
        id: `central-${skill.id}`,
        label: skill.name,
        description: skill.description,
        groupKey: "central",
        groupLabel: t("globalSearch.centralSkills"),
        icon: (
          <Blocks className="size-4 shrink-0 text-muted-foreground group-data-selected/command-item:text-primary" />
        ),
        searchText: buildSearchText([skill.name, skill.description]),
        labelText,
        descriptionText,
        onSelect: () => {
          close();
          navigate(`/skill/${skill.id}`);
        },
      });
    }

    // Collections
    for (const col of collections) {
      result.push({
        id: `collection-${col.id}`,
        label: col.name,
        description: col.description ?? undefined,
        groupKey: "collections",
        groupLabel: t("globalSearch.collections"),
        icon: (
          <Layers className="size-4 shrink-0 text-muted-foreground group-data-selected/command-item:text-primary" />
        ),
        searchText: buildSearchText([col.name, col.description]),
        labelText: col.name.toLowerCase(),
        descriptionText: (col.description ?? "").toLowerCase(),
        onSelect: () => {
          close();
          navigate("/collections", {
            state: { collectionContext: { collectionId: col.id } },
          });
        },
      });
    }

    // Platform Views
    const platformAgents = getPlatformTargetGroups(agents, categoryVisibility);
    for (const agent of platformAgents) {
      const displayPath = getPlatformTargetPathHint(agent);
      const label = getPlatformTargetLabel(agent, t, "short");
      const memberNames = getPlatformTargetMemberNames(agent);
      result.push({
        id: `platform-${agent.id}`,
        label,
        description: displayPath,
        groupKey: "platforms",
        groupLabel: t("globalSearch.platforms"),
        icon: (
          <PlatformIcon
            agentId={agent.id}
            className="size-4 text-muted-foreground group-data-selected/command-item:text-primary"
          />
        ),
        searchText: buildSearchText([
          label,
          agent.global_skills_dir,
          ...memberNames,
        ]),
        labelText: label.toLowerCase(),
        descriptionText: displayPath.toLowerCase(),
        onSelect: () => {
          close();
          navigate(`/platform/${agent.id}`);
        },
      });
    }

    // Actions
    result.push(
      {
        id: "action-dashboard",
        label: t("globalSearch.actionDashboard"),
        groupKey: "actions",
        groupLabel: t("globalSearch.actions"),
        icon: (
          <LayoutDashboard className="size-4 shrink-0 text-muted-foreground group-data-selected/command-item:text-primary" />
        ),
        searchText: buildSearchText([
          t("globalSearch.actionDashboard"),
          t("sidebar.dashboard"),
        ]),
        labelText: t("globalSearch.actionDashboard").toLowerCase(),
        descriptionText: "",
        onSelect: () => {
          close();
          navigate("/dashboard");
        },
      },
      {
        id: "action-rescan",
        label: t("globalSearch.actionRescan"),
        groupKey: "actions",
        groupLabel: t("globalSearch.actions"),
        icon: (
          <RefreshCw className="size-4 shrink-0 text-muted-foreground group-data-selected/command-item:text-primary" />
        ),
        searchText: buildSearchText([t("globalSearch.actionRescan")]),
        labelText: t("globalSearch.actionRescan").toLowerCase(),
        descriptionText: "",
        onSelect: () => {
          close();
          onAction("rescan");
        },
      },
      {
        id: "action-new-collection",
        label: t("globalSearch.actionNewCollection"),
        groupKey: "actions",
        groupLabel: t("globalSearch.actions"),
        icon: (
          <Plus className="size-4 shrink-0 text-muted-foreground group-data-selected/command-item:text-primary" />
        ),
        searchText: buildSearchText([t("globalSearch.actionNewCollection")]),
        labelText: t("globalSearch.actionNewCollection").toLowerCase(),
        descriptionText: "",
        onSelect: () => {
          close();
          onAction("new-collection");
        },
      },
    );

    return result;
  }, [
    centralSkills,
    collections,
    agents,
    categoryVisibility,
    navigate,
    close,
    open,
    onAction,
    t,
  ]);

  const visibleGroups = useMemo(() => {
    if (!open) return [];

    const rankedItems = (groupKey: SearchItem["groupKey"], limit: number) => {
      if (!normalizedQuery) {
        return items
          .filter((item) => item.groupKey === groupKey)
          .slice(0, limit);
      }

      return items
        .map((item) => ({
          item,
          score: scoreSearchMatch(
            normalizedQuery,
            item.labelText,
            item.descriptionText,
            item.searchText,
          ),
        }))
        .filter((entry) => Number.isFinite(entry.score))
        .sort((left, right) => {
          if (left.score !== right.score) {
            return left.score - right.score;
          }
          return left.item.label.localeCompare(right.item.label);
        })
        .slice(0, 50)
        .map((entry) => entry.item)
        .filter((item) => item.groupKey === groupKey);
    };

    const groups: VisibleGroup[] = [];

    for (const group of groupMeta) {
      const groupItems = rankedItems(group.key, group.initialLimit);

      if (group.key === "central") {
        const showStatus =
          centralStatus === "loading" ||
          centralStatus === "error" ||
          centralStatus === "empty";
        if (!showStatus && groupItems.length === 0) {
          continue;
        }
        groups.push({
          key: group.key,
          heading: group.label,
          items: centralStatus === "ready" ? groupItems : [],
          status: centralStatus,
          error: centralError,
          onRetry: retryCentral,
        });
        continue;
      }

      if (group.key === "collections") {
        const showStatus =
          collectionsStatus === "loading" ||
          collectionsStatus === "error" ||
          collectionsStatus === "empty";
        if (!showStatus && groupItems.length === 0) {
          continue;
        }
        groups.push({
          key: group.key,
          heading: group.label,
          items: collectionsStatus === "ready" ? groupItems : [],
          status: collectionsStatus,
          error: collectionsError,
          onRetry: retryCollections,
        });
        continue;
      }

      if (groupItems.length > 0) {
        groups.push({
          key: group.key,
          heading: group.label,
          items: groupItems,
        });
      }
    }

    return groups;
  }, [
    groupMeta,
    items,
    normalizedQuery,
    open,
    centralStatus,
    centralError,
    retryCentral,
    collectionsStatus,
    collectionsError,
    retryCollections,
  ]);

  const hideNoResults =
    centralStatus === "loading" ||
    centralStatus === "error" ||
    centralStatus === "empty" ||
    collectionsStatus === "loading" ||
    collectionsStatus === "error" ||
    collectionsStatus === "empty";

  // Cmd+K shortcut (also registered here so the dialog self-toggles)
  useHotkey("mod+k", () => onOpenChange(!open));

  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("globalSearch.title")}
      description={t("globalSearch.description")}
      className="sm:max-w-lg"
      showCloseButton={false}
    >
      <Command shouldFilter={false}>
        <CommandInput
          placeholder={t("globalSearch.placeholder")}
          value={query}
          onValueChange={setQuery}
        />
        <CommandList>
          {!hideNoResults && (
            <CommandEmpty>{t("globalSearch.noResults")}</CommandEmpty>
          )}
          {visibleGroups.map((group) => (
            <CommandGroup key={group.key} heading={group.heading}>
              {group.status && group.onRetry ? (
                <SearchSourceStatus
                  status={group.status}
                  error={group.error ?? null}
                  onRetry={group.onRetry}
                />
              ) : null}
              {group.items.map((item) => (
                <CommandItem
                  key={item.id}
                  value={`${item.label} ${item.description ?? ""}`}
                  onSelect={item.onSelect}
                >
                  {item.icon}
                  <div className="flex flex-col min-w-0">
                    <span className="truncate text-sm">{item.label}</span>
                    {item.description && (
                      <span className="truncate text-xs text-muted-foreground">
                        {item.description}
                      </span>
                    )}
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
          ))}
        </CommandList>
      </Command>
    </CommandDialog>
  );
}
