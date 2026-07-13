import { GitPullRequestArrow, RefreshCw } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { SettingsSection } from "@/components/settings/SettingsSection";
import { Button } from "@/components/ui/button";
import type { UpdateCheckMode } from "@/pages/centralUpdateCheckMode";
import { cn } from "@/lib/utils";

interface CentralUpdateCheckModeSettingsSectionProps {
  mode: UpdateCheckMode;
  isLoading: boolean;
  onChange: (mode: UpdateCheckMode) => void;
}

export function CentralUpdateCheckModeSettingsSection({
  mode,
  isLoading,
  onChange,
}: CentralUpdateCheckModeSettingsSectionProps) {
  const { t } = useTranslation();
  return (
    <SettingsSection
      sectionId="central-update-check-mode"
      title={t("settings.centralUpdateCheckModeTitle")}
      description={t("settings.centralUpdateCheckModeDesc")}
      icon={<RefreshCw className="size-5 shrink-0 text-muted-foreground" />}
    >
      <div className="grid gap-3 md:grid-cols-2">
        <ModeButton
          active={mode === "regular"}
          disabled={isLoading}
          icon={<RefreshCw className="size-4" aria-hidden="true" />}
          title={t("settings.centralUpdateCheckModeRegular")}
          description={t("settings.centralUpdateCheckModeRegularDesc")}
          onClick={() => onChange("regular")}
        />
        <ModeButton
          active={mode === "sync"}
          disabled={isLoading}
          icon={<GitPullRequestArrow className="size-4" aria-hidden="true" />}
          title={t("settings.centralUpdateCheckModeSync")}
          description={t("settings.centralUpdateCheckModeSyncDesc")}
          onClick={() => onChange("sync")}
        />
      </div>
      <p className="mt-3 text-xs text-muted-foreground">
        {t("settings.centralUpdateCheckModeHint")}
      </p>
    </SettingsSection>
  );
}

function ModeButton({
  active,
  disabled,
  icon,
  title,
  description,
  onClick,
}: {
  active: boolean;
  disabled: boolean;
  icon: ReactNode;
  title: string;
  description: string;
  onClick: () => void;
}) {
  return (
    <Button
      type="button"
      variant="outline"
      disabled={disabled}
      aria-pressed={active}
      className={cn(
        "h-auto justify-start rounded-xl p-4 text-left",
        active && "border-primary bg-primary/10 text-foreground hover:bg-primary/10",
      )}
      onClick={onClick}
    >
      <span className="mr-3 mt-0.5 inline-flex size-8 shrink-0 items-center justify-center rounded-full border border-border bg-muted">
        {icon}
      </span>
      <span className="min-w-0">
        <span className="block text-sm font-semibold">{title}</span>
        <span className="mt-1 block whitespace-normal text-xs leading-5 text-muted-foreground">
          {description}
        </span>
      </span>
    </Button>
  );
}
