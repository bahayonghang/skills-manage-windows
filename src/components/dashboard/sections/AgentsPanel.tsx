import { useTranslation } from "react-i18next";
import { ChevronRight } from "lucide-react";

import { PanelHeader } from "@/components/dashboard/DashboardPanels";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  getPlatformTargetPathHint,
  isUniversalPlatformTarget,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";

interface AgentsPanelProps {
  onNavigate: (path: string) => void;
  visiblePlatformTargets: PlatformTarget[];
  enabledTargetsCount: number;
  skillsByAgent: Record<string, number>;
}

export function AgentsPanel({
  onNavigate,
  visiblePlatformTargets,
  enabledTargetsCount,
  skillsByAgent,
}: AgentsPanelProps) {
  const { t } = useTranslation();

  return (
    <Card className="surface-glass overflow-hidden rounded-3xl border-0 bg-transparent p-0 shadow-none">
      <PanelHeader
        title={t("dashboard.agents.title", {
          enabled: enabledTargetsCount,
          total: visiblePlatformTargets.length,
        })}
        description={t("dashboard.agents.description")}
        action={
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onNavigate("/settings/platforms")}
          >
            {t("dashboard.agents.manage")}
            <ChevronRight className="size-3.5" />
          </Button>
        }
      />
      <CardContent className="p-0">
        {visiblePlatformTargets.length > 0 ? (
          <ul>
            {visiblePlatformTargets.slice(0, 7).map((agent) => {
              const countAgentId = isUniversalPlatformTarget(agent)
                ? agent.install_agent_id
                : agent.id;
              const pathHint = getPlatformTargetPathHint(agent);
              const label = isUniversalPlatformTarget(agent)
                ? t("platformTargets.universalShortLabel")
                : agent.display_name;

              return (
                <li key={agent.id}>
                  <button
                    type="button"
                    onClick={() => onNavigate(`/platform/${agent.id}`)}
                    className="grid w-full grid-cols-[2rem_minmax(0,1fr)_auto_auto] items-center gap-3 border-t border-border/70 px-4 py-3 text-left transition-colors first:border-t-0 hover:bg-muted/25"
                    aria-label={t("dashboard.agents.openLabel", { name: label })}
                  >
                    <span className="grid size-8 place-items-center rounded-md border border-border/80 bg-background text-primary-text">
                      <PlatformIcon agentId={agent.id} className="size-4" />
                    </span>
                    <span className="min-w-0">
                      <span className="flex min-w-0 items-center gap-2">
                        <span className="truncate text-sm font-medium">
                          {label}
                        </span>
                        <span className="hidden rounded border border-border bg-muted/40 px-1.5 py-0.5 text-[0.65rem] font-medium uppercase tracking-wide text-muted-foreground sm:inline">
                          {agent.is_enabled
                            ? t("dashboard.agents.enabled")
                            : t("dashboard.agents.hidden")}
                        </span>
                      </span>
                      <span className="mt-1 block truncate text-xs text-muted-foreground">
                        {pathHint || t("dashboard.agents.noPath")}
                      </span>
                    </span>
                    <span className="font-display text-sm font-semibold tabular-nums">
                      {skillsByAgent[countAgentId] ?? 0}
                    </span>
                    <ChevronRight className="size-4 text-muted-foreground" />
                  </button>
                </li>
              );
            })}
          </ul>
        ) : (
          <div className="px-4 py-6 text-sm text-muted-foreground">
            {t("sidebar.noPlatforms")}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
