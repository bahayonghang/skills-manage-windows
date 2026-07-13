import { useState } from "react";
import { ChevronDown, ChevronRight, Eye, Search } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Input } from "@/components/ui/input";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { Switch } from "@/components/ui/switch";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import {
  isUniversalPlatformTarget,
  type PlatformTarget,
  type PlatformTargetGroup,
} from "@/lib/platformTargetGroups";
import { getPlatformPathHint } from "@/lib/path";
import type { PlatformCategoryKey } from "@/lib/platformVisibility";
import { matchesPlatformVisibilityQuery } from "./platformVisibilityUtils";

export interface PlatformVisibilityGroupItem {
  agents: PlatformTarget[];
  category: PlatformCategoryKey;
  description: string;
  enabledCount: number;
  groupVisible: boolean;
  title: string;
  totalCount: number;
}

interface PlatformVisibilitySettingsSectionProps {
  groups: PlatformVisibilityGroupItem[];
  isSearchActive: boolean;
  normalizedQuery: string;
  query: string;
  onQueryChange: (value: string) => void;
  onToggleCategory: (category: PlatformCategoryKey, visible: boolean) => void;
  onTogglePlatform: (agentId: string, enabled: boolean) => void;
}

export function PlatformVisibilitySettingsSection({
  groups,
  isSearchActive,
  normalizedQuery,
  query,
  onQueryChange,
  onToggleCategory,
  onTogglePlatform,
}: PlatformVisibilitySettingsSectionProps) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      sectionId="platform-visibility"
      title={t("settings.platformVisibility")}
      description={t("settings.platformVisibilityDesc")}
      icon={<Eye className="size-5 shrink-0 text-muted-foreground" />}
    >
        <div className="space-y-4">
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={query}
              onChange={(event) => onQueryChange(event.target.value)}
              placeholder={t("settings.platformSearchPlaceholder")}
              aria-label={t("settings.platformSearchLabel")}
              className="pl-9"
            />
          </div>

          <div className="rounded-lg border border-border/70 bg-muted/20 p-3 text-sm text-muted-foreground">
            <p>{t("settings.platformVisibilityScope")}</p>
          </div>

          {groups.length === 0 ? (
            <div className="rounded-lg border border-dashed border-border px-4 py-6 text-sm text-muted-foreground">
              {t("settings.platformSearchEmpty")}
            </div>
          ) : (
            groups.map((group) => (
              <PlatformVisibilityGroup
                key={group.category}
                category={group.category}
                title={group.title}
                description={group.description}
                agents={group.agents}
                enabledCount={group.enabledCount}
                totalCount={group.totalCount}
                groupVisible={group.groupVisible}
                isSearchActive={isSearchActive}
                normalizedQuery={normalizedQuery}
                onToggleGroup={(visible) =>
                  onToggleCategory(group.category, visible)
                }
                onToggleAgent={onTogglePlatform}
              />
            ))
          )}
        </div>
    </SettingsSection>
  );
}

interface PlatformVisibilityRowProps {
  agent: PlatformTarget;
  onToggle: (enabled: boolean) => void;
}

function PlatformVisibilityRow({ agent, onToggle }: PlatformVisibilityRowProps) {
  const { t } = useTranslation();
  const pathHint = getPlatformPathHint(agent.global_skills_dir);

  return (
    <div className="flex items-center gap-3 rounded-md border border-border/60 bg-background px-3 py-2">
      <PlatformIcon
        agentId={agent.id}
        className="size-4 shrink-0 text-muted-foreground"
      />
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">{agent.display_name}</div>
        <div className="truncate text-xs text-muted-foreground">
          {pathHint}
        </div>
      </div>
      <Switch
        checked={agent.is_enabled}
        onCheckedChange={onToggle}
        aria-label={t("settings.togglePlatformVisibilityLabel", {
          name: agent.display_name,
        })}
      />
    </div>
  );
}

interface PlatformVisibilityUniversalRowProps {
  agent: PlatformTargetGroup;
  normalizedQuery: string;
  onToggle: (agentId: string, enabled: boolean) => void;
}

function PlatformVisibilityUniversalRow({
  agent,
  normalizedQuery,
  onToggle,
}: PlatformVisibilityUniversalRowProps) {
  const { t } = useTranslation();
  const visibleMembers = agent.member_agents.filter((member) =>
    matchesPlatformVisibilityQuery(member, normalizedQuery)
  );
  const enabledMembers = agent.member_agents.filter((member) => member.is_enabled).length;
  const allMembersEnabled = agent.member_agents.every((member) => member.is_enabled);
  const pathHint = getPlatformPathHint(agent.global_skills_dir);

  return (
    <div className="rounded-md border border-border/60 bg-background p-3">
      <div className="flex items-center gap-3">
        <PlatformIcon
          agentId={agent.id}
          className="size-4 shrink-0 text-muted-foreground"
        />
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-medium">{agent.display_name}</div>
          <div className="truncate text-xs text-muted-foreground">{pathHint}</div>
          <div className="mt-0.5 text-xs text-muted-foreground">
            {t("settings.universalMembersSummary", {
              enabled: enabledMembers,
              total: agent.member_agents.length,
            })}
          </div>
        </div>
        <Switch
          checked={allMembersEnabled}
          onCheckedChange={(enabled) => {
            agent.member_agents.forEach((member) => onToggle(member.id, enabled));
          }}
          aria-label={t("settings.togglePlatformVisibilityLabel", {
            name: agent.display_name,
          })}
        />
      </div>
      {visibleMembers.length > 0 ? (
        <div className="mt-3 space-y-2 border-t border-border/60 pt-3">
          {visibleMembers.map((member) => (
            <PlatformVisibilityRow
              key={member.id}
              agent={member}
              onToggle={(enabled) => onToggle(member.id, enabled)}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

interface PlatformVisibilityGroupProps {
  agents: PlatformTarget[];
  category: PlatformCategoryKey;
  description: string;
  enabledCount: number;
  groupVisible: boolean;
  isSearchActive: boolean;
  normalizedQuery: string;
  title: string;
  totalCount: number;
  onToggleAgent: (agentId: string, enabled: boolean) => void;
  onToggleGroup: (visible: boolean) => void;
}

function PlatformVisibilityGroup({
  agents,
  category,
  description,
  enabledCount,
  groupVisible,
  isSearchActive,
  normalizedQuery,
  title,
  totalCount,
  onToggleAgent,
  onToggleGroup,
}: PlatformVisibilityGroupProps) {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(false);
  const detailsCollapsed = isCollapsed && !isSearchActive;
  const detailsId = `platform-visibility-${category}-details`;
  const ToggleIcon = detailsCollapsed ? ChevronRight : ChevronDown;

  return (
    <div className="rounded-lg border border-border/70">
      <div className="flex items-start justify-between gap-3 border-b border-border/70 px-4 py-3">
        <div className="flex min-w-0 flex-1 items-start gap-2">
          <button
            type="button"
            className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-md border border-border/60 bg-background text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            aria-controls={detailsId}
            aria-expanded={!detailsCollapsed}
            aria-label={t(
              detailsCollapsed
                ? "settings.expandPlatformGroupDetailsLabel"
                : "settings.collapsePlatformGroupDetailsLabel",
              { name: title }
            )}
            onClick={() => setIsCollapsed((value) => !value)}
          >
            <ToggleIcon className="size-4" />
          </button>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-sm font-medium">{title}</h3>
              <span className="text-xs text-muted-foreground">
                {t("settings.platformEnabledSummary", {
                  enabled: enabledCount,
                  total: totalCount,
                })}
              </span>
            </div>
            <p className="mt-1 text-xs text-muted-foreground">{description}</p>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <button
            type="button"
            className="rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
            aria-controls={detailsId}
            aria-expanded={!detailsCollapsed}
            onClick={() => setIsCollapsed((value) => !value)}
          >
            {t(
              detailsCollapsed
                ? "settings.platformGroupExpandDetails"
                : "settings.platformGroupCollapseDetails"
            )}
          </button>
          <span className="text-xs text-muted-foreground">
            {t("settings.platformGroupVisible")}
          </span>
          <Switch
            checked={groupVisible}
            onCheckedChange={onToggleGroup}
            aria-label={t("settings.toggleCategoryVisibilityLabel", { name: title })}
          />
        </div>
      </div>

      {!groupVisible && !isSearchActive ? (
        <div id={detailsId} className="px-4 py-3 text-xs text-muted-foreground">
          {t("settings.platformGroupHiddenSummary", {
            enabled: enabledCount,
            total: totalCount,
          })}
        </div>
      ) : detailsCollapsed ? (
        <div id={detailsId} className="px-4 py-3 text-xs text-muted-foreground">
          {t("settings.platformGroupCollapsedSummary", {
            enabled: enabledCount,
            total: totalCount,
            visibility: t(
              groupVisible
                ? "settings.platformGroupVisibilityOn"
                : "settings.platformGroupVisibilityOff"
            ),
          })}
        </div>
      ) : agents.length === 0 ? (
        <div id={detailsId} className="px-4 py-3 text-xs text-muted-foreground">
          {t("settings.noPlatformItems")}
        </div>
      ) : (
        <div id={detailsId} className="space-y-2 p-3">
          {agents.map((agent) =>
            isUniversalPlatformTarget(agent) ? (
              <PlatformVisibilityUniversalRow
                key={agent.id}
                agent={agent}
                normalizedQuery={normalizedQuery}
                onToggle={onToggleAgent}
              />
            ) : (
              <PlatformVisibilityRow
                key={agent.id}
                agent={agent}
                onToggle={(enabled) => onToggleAgent(agent.id, enabled)}
              />
            )
          )}
        </div>
      )}
    </div>
  );
}
