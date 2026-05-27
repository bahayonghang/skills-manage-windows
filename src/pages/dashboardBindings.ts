import { useEffect, useRef } from "react";

import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useCollectionStore } from "@/stores/collectionStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";

type DashboardLogQuery = {
  limit: number;
  offset: number;
};

async function noopAsync() {
  return undefined;
}

async function noopUnsubscribeFactory() {
  return () => undefined;
}

export function useDashboardBindings() {
  const platformAgents = usePlatformStore((state) => state.agents) ?? [];
  const skillsByAgent = usePlatformStore((state) => state.skillsByAgent) ?? {};
  const collectionCount = usePlatformStore((state) => state.collectionCount) ?? 0;
  const dashboardCentralSummary = usePlatformStore(
    (state) => state.dashboardCentralSummary,
  );
  const categoryVisibility = usePlatformStore((state) => state.categoryVisibility);
  const lastScanAt = usePlatformStore((state) => state.lastScanAt);
  const scanState = usePlatformStore((state) => state.scanState);
  const isPlatformLoading = usePlatformStore((state) => state.isLoading) ?? false;
  const isPlatformRefreshing =
    usePlatformStore((state) => state.isRefreshing) ?? false;

  const centralSkills = useCentralSkillsStore((state) => state.skills) ?? [];
  const repositories = useCentralSkillsStore((state) => state.repositories) ?? [];
  const aiTagReviews = useCentralSkillsStore((state) => state.aiTagReviews) ?? [];
  const aiTagJob = useCentralSkillsStore((state) => state.aiTagJob);
  const updateStatuses = useCentralSkillsStore((state) => state.updateStatuses) ?? {};
  const updateJob = useCentralSkillsStore((state) => state.updateJob);
  const centralError = useCentralSkillsStore((state) => state.error);
  const subscribeAiTagProgress =
    useCentralSkillsStore((state) => state.subscribeAiTagProgress) ??
    noopUnsubscribeFactory;
  const subscribeUpdateProgress =
    useCentralSkillsStore((state) => state.subscribeUpdateProgress) ??
    noopUnsubscribeFactory;

  const collections = useCollectionStore((state) => state.collections) ?? [];
  const isCollectionsLoading =
    useCollectionStore((state) => state.isLoading) ?? false;
  const collectionsError = useCollectionStore((state) => state.error);
  const loadCollections =
    useCollectionStore((state) => state.loadCollections) ?? noopAsync;

  const registries = useMarketplaceStore((state) => state.registries) ?? [];
  const isMarketplaceLoading =
    useMarketplaceStore((state) => state.isLoading) ?? false;
  const marketplaceError = useMarketplaceStore((state) => state.error);
  const loadRegistries =
    useMarketplaceStore((state) => state.loadRegistries) ?? noopAsync;

  const logEntries = useOperationLogStore((state) => state.entries) ?? [];
  const logTotal = useOperationLogStore((state) => state.total) ?? 0;
  const isLogsLoading = useOperationLogStore((state) => state.isLoading) ?? false;
  const logsError = useOperationLogStore((state) => state.error);
  const loadLogs =
    useOperationLogStore((state) => state.loadLogs) ??
    (async (_query: DashboardLogQuery) => undefined);

  const activeTarget = useTargetStore((state) => state.activeTarget);
  const targets = useTargetStore((state) => state.targets) ?? [];

  return {
    platformAgents,
    skillsByAgent,
    collectionCount,
    dashboardCentralSummary,
    categoryVisibility,
    lastScanAt,
    scanState,
    isPlatformLoading,
    isPlatformRefreshing,
    centralSkills,
    repositories,
    aiTagReviews,
    aiTagJob,
    updateStatuses,
    updateJob,
    centralError,
    subscribeAiTagProgress,
    subscribeUpdateProgress,
    collections,
    isCollectionsLoading,
    collectionsError,
    loadCollections,
    registries,
    isMarketplaceLoading,
    marketplaceError,
    loadRegistries,
    logEntries,
    logTotal,
    isLogsLoading,
    logsError,
    loadLogs,
    activeTarget,
    targets,
  };
}

export function useDashboardBootstrap({
  collectionsLength,
  isCollectionsLoading,
  loadCollections,
  registriesLength,
  isMarketplaceLoading,
  loadRegistries,
  loadLogs,
  subscribeAiTagProgress,
  subscribeUpdateProgress,
  recentLogLimit,
}: {
  collectionsLength: number;
  isCollectionsLoading: boolean;
  loadCollections: () => Promise<void>;
  registriesLength: number;
  isMarketplaceLoading: boolean;
  loadRegistries: () => Promise<void>;
  loadLogs: (query: DashboardLogQuery) => Promise<unknown>;
  subscribeAiTagProgress: () => Promise<() => void>;
  subscribeUpdateProgress: () => Promise<() => void>;
  recentLogLimit: number;
}) {
  const requestedCollectionsRef = useRef(false);
  const requestedRegistriesRef = useRef(false);
  const requestedLogsRef = useRef(false);

  useEffect(() => {
    if (
      requestedCollectionsRef.current ||
      isCollectionsLoading ||
      collectionsLength > 0
    ) {
      return;
    }

    requestedCollectionsRef.current = true;
    void loadCollections();
  }, [collectionsLength, isCollectionsLoading, loadCollections]);

  useEffect(() => {
    if (
      requestedRegistriesRef.current ||
      isMarketplaceLoading ||
      registriesLength > 0
    ) {
      return;
    }

    requestedRegistriesRef.current = true;
    void loadRegistries();
  }, [isMarketplaceLoading, loadRegistries, registriesLength]);

  useEffect(() => {
    if (requestedLogsRef.current) return;

    requestedLogsRef.current = true;
    void loadLogs({ limit: recentLogLimit, offset: 0 });
  }, [loadLogs, recentLogLimit]);

  useEffect(() => {
    let aiUnsubscribe: (() => void) | undefined;
    let updateUnsubscribe: (() => void) | undefined;
    let disposed = false;

    void subscribeAiTagProgress().then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      aiUnsubscribe = unsubscribe;
    });

    void subscribeUpdateProgress().then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      updateUnsubscribe = unsubscribe;
    });

    return () => {
      disposed = true;
      aiUnsubscribe?.();
      updateUnsubscribe?.();
    };
  }, [subscribeAiTagProgress, subscribeUpdateProgress]);
}
