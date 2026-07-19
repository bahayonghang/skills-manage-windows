import { Cpu, Pencil, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { formatPathForDisplay } from "@/lib/path";
import type { AgentWithStatus } from "@/types";

interface CustomPlatformsSettingsSectionProps {
  customAgents: AgentWithStatus[];
  platformError: string | null;
  removingAgent: string | null;
  onAddPlatform: () => void;
  onEditPlatform: (agent: AgentWithStatus) => void;
  onRemovePlatform: (agentId: string) => void;
}

export function CustomPlatformsSettingsSection({
  customAgents,
  platformError,
  removingAgent,
  onAddPlatform,
  onEditPlatform,
  onRemovePlatform,
}: CustomPlatformsSettingsSectionProps) {
  const { t } = useTranslation();

  return (
    <SettingsSection
      sectionId="custom-platforms"
      title={t("settings.customPlatforms")}
      description={t("settings.customPlatformsDesc")}
      icon={<Cpu className="size-5 shrink-0 text-muted-foreground" />}
      action={
        <Button
          variant="outline"
          size="sm"
          onClick={onAddPlatform}
          aria-label={t("settings.addPlatformAriaLabel")}
        >
          <Plus className="size-3.5" />
          <span>{t("settings.addPlatform")}</span>
        </Button>
      }
    >
        {platformError && (
          <p className="text-xs text-destructive-text mb-3" role="alert">
            {platformError}
          </p>
        )}

        {customAgents.length === 0 ? (
          <p className="text-sm text-muted-foreground py-4 text-center">
            {t("settings.noPlatforms")}
          </p>
        ) : (
          <div className="rounded-lg border border-border overflow-hidden">
            {customAgents.map((agent) => (
              <CustomPlatformRow
                key={agent.id}
                agent={agent}
                onEdit={() => onEditPlatform(agent)}
                onRemove={() => onRemovePlatform(agent.id)}
                isRemoving={removingAgent === agent.id}
              />
            ))}
          </div>
        )}
    </SettingsSection>
  );
}

interface CustomPlatformRowProps {
  agent: AgentWithStatus;
  onEdit: () => void;
  onRemove: () => void;
  isRemoving: boolean;
}

function CustomPlatformRow({
  agent,
  onEdit,
  onRemove,
  isRemoving,
}: CustomPlatformRowProps) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center gap-3 py-2.5 px-4 border-b border-border/50 last:border-0">
      <Cpu className="size-4 text-muted-foreground shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium truncate">{agent.display_name}</div>
        <div className="text-xs text-muted-foreground truncate mt-0.5">
          {formatPathForDisplay(agent.global_skills_dir)}
        </div>
      </div>
      <div className="flex items-center gap-2 shrink-0">
        <Button
          variant="outline"
          size="sm"
          onClick={onEdit}
          aria-label={t("settings.editPlatformLabel", { name: agent.display_name })}
        >
          <Pencil className="size-3.5" />
          <span>{t("common.edit")}</span>
        </Button>
        <InlineConfirmAction
          onConfirm={onRemove}
          isLoading={isRemoving}
          idleAriaLabel={t("settings.removePlatformLabel", { name: agent.display_name })}
          idleTitle={t("settings.removePlatformLabel", { name: agent.display_name })}
          confirmLabel={t("common.confirmDelete")}
          icon={<Trash2 className="size-3.5" />}
        />
      </div>
    </div>
  );
}
