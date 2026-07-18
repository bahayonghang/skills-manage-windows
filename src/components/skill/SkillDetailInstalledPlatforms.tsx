import { X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { Button } from "@/components/ui/button";
import { formatPathForDisplay } from "@/lib/path";
import type {
  InstalledPlatformRow,
  InstalledPlatformUninstallRequest,
} from "./skillDetailInstalledPlatformsModel";

export function InstalledPlatformList({
  rows,
  skillName,
  onRequestUninstall,
}: {
  rows: InstalledPlatformRow[];
  skillName: string;
  onRequestUninstall: (request: InstalledPlatformUninstallRequest) => void;
}) {
  const { t } = useTranslation();

  if (rows.length === 0) {
    return null;
  }

  return (
    <div
      data-testid="detail-installed-platforms"
      className="mt-2 space-y-1.5 rounded-lg border border-border/60 bg-background/70 p-2"
    >
      <div className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {t("detail.installedPlatforms")}
      </div>
      <ul className="space-y-1">
        {rows.map((row) => {
          const removeLabel = t("detail.removeInstalledPlatformAriaLabel", {
            platform: row.displayName,
            skill: skillName,
          });
          return (
            <li
              key={row.agentId}
              data-testid={`detail-installed-platform-${row.agentId}`}
              className="group flex min-h-8 items-center gap-2 rounded-md border border-border/50 bg-muted/20 px-2 py-1"
              title={`${row.displayName} - ${formatPathForDisplay(row.installedPath)}`}
            >
              <PlatformIcon
                agentId={row.agentId}
                className="size-3.5 text-muted-foreground"
                size={14}
              />
              <div className="min-w-0 flex-1">
                <div className="truncate text-xs font-medium text-foreground">
                  {row.displayName}
                </div>
                <div className="truncate text-ui-meta text-muted-foreground">
                  {row.linkType}
                </div>
              </div>
              {row.isRemovable && (
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  className="size-6 text-muted-foreground opacity-0 transition-opacity hover:text-destructive-text focus:opacity-100 focus-visible:text-destructive-text group-hover:opacity-100 group-focus-within:opacity-100"
                  aria-label={removeLabel}
                  title={removeLabel}
                  onClick={() =>
                    onRequestUninstall({
                      agentId: row.agentId,
                      displayName: row.displayName,
                    })
                  }
                >
                  <X className="size-3.5" />
                </Button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
