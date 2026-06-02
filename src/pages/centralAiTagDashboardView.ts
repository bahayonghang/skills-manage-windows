import { useEffect, useMemo } from "react";

import {
  buildAiTagProgressItems,
  buildAiTagRateProfile,
} from "@/lib/centralAiTagDashboard";
import { useSettingsStore } from "@/stores/settingsStore";
import type { AiTagJob, SkillWithLinks } from "@/types";

export function useCentralAiTagDashboardView({
  aiTagJob,
  skills,
}: {
  aiTagJob: AiTagJob;
  skills: readonly SkillWithLinks[];
}) {
  const aiSettings = useSettingsStore((state) => state.aiSettings);
  const aiSettingsLoaded = useSettingsStore((state) => state.aiSettingsLoaded);
  const isLoadingAiSettings = useSettingsStore((state) => state.isLoadingAiSettings);
  const loadAiSettings = useSettingsStore((state) => state.loadAiSettings);

  useEffect(() => {
    if (!aiSettingsLoaded && !isLoadingAiSettings) {
      void loadAiSettings();
    }
  }, [aiSettingsLoaded, isLoadingAiSettings, loadAiSettings]);

  return {
    aiTagProgressItems: useMemo(
      () => buildAiTagProgressItems(aiTagJob, skills),
      [aiTagJob, skills]
    ),
    aiTagRateProfile: useMemo(
      () => buildAiTagRateProfile(aiSettings),
      [aiSettings]
    ),
  };
}
