import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { SkillsCliLinkTargetSummary } from "@/pages/skillsCliBatchModel";

export interface SkillsCliBatchBarProps {
  selectedCount: number;
  summaries: readonly SkillsCliLinkTargetSummary[];
  unlinkEnabled: boolean;
  busy: boolean;
  exporting: boolean;
  linkMenuOpen: boolean;
  onLinkMenuOpenChange: (open: boolean) => void;
  onLink: (agentId: string) => void;
  onUnlink: () => void;
  onExportSelected: () => void;
  onUninstall: () => void;
  onClear: () => void;
}

const ICON_HIT =
  "relative size-8 after:absolute after:left-1/2 after:top-1/2 after:size-10 after:-translate-x-1/2 after:-translate-y-1/2 after:content-['']";

export function SkillsCliBatchBar({
  selectedCount,
  summaries,
  unlinkEnabled,
  busy,
  exporting,
  linkMenuOpen,
  onLinkMenuOpenChange,
  onLink,
  onUnlink,
  onExportSelected,
  onUninstall,
  onClear,
}: SkillsCliBatchBarProps) {
  const { t } = useTranslation();
  if (selectedCount <= 0) {
    return null;
  }
  const mutationsLocked = busy;
  const anyLinkable = summaries.some((item) => item.linkableCount > 0);

  return (
    <div
      role="region"
      aria-label={t("skillsCli.batch.barAria")}
      aria-busy={busy || exporting}
      data-testid="skills-cli-batch-bar"
      className="flex flex-wrap items-center gap-2 border-t border-border bg-background/95 px-4 py-2"
    >
      <p className="text-sm font-medium tabular-nums">
        {t("skillsCli.batch.selectedCount", { count: selectedCount })}
      </p>
      <MenuPrimitive.Root
        open={linkMenuOpen}
        onOpenChange={onLinkMenuOpenChange}
      >
        <MenuPrimitive.Trigger
          disabled={mutationsLocked || !anyLinkable}
          render={
            <Button
              type="button"
              variant="outline"
              className="min-h-10"
              aria-label={t("skillsCli.batch.linkMenuAria")}
              data-testid="skills-cli-batch-link"
            />
          }
        >
          {t("skillsCli.batch.linkToPlatform")}
        </MenuPrimitive.Trigger>
        <MenuPrimitive.Portal>
          <MenuPrimitive.Positioner
            align="start"
            sideOffset={4}
            className="z-50 outline-none"
          >
            <MenuPrimitive.Popup
              className={cn(
                "min-w-[16rem] rounded-lg bg-popover p-1 text-sm text-popover-foreground shadow-md ring-1 ring-foreground/10 outline-none",
                "data-[starting-style]:animate-in data-[starting-style]:fade-in-0 data-[starting-style]:zoom-in-95",
                "data-[ending-style]:animate-out data-[ending-style]:fade-out-0 data-[ending-style]:zoom-out-95",
                "animation-duration-100",
              )}
            >
              {summaries.map((summary) => {
                const disabled = summary.linkableCount === 0 || mutationsLocked;
                return (
                  <MenuPrimitive.Item
                    key={summary.agentId}
                    disabled={disabled}
                    label={t("skillsCli.batch.linkTargetAria", {
                      name: summary.displayName,
                      linkable: summary.linkableCount,
                      managed: summary.managedCount,
                      copies: summary.directCopyCount,
                      blocked: summary.blockedCount,
                    })}
                    onClick={() => onLink(summary.agentId)}
                    data-testid={`skills-cli-batch-link-${summary.agentId}`}
                    className={cn(
                      "flex cursor-pointer flex-col gap-0.5 rounded-md px-2.5 py-1.5 outline-none data-[highlighted]:bg-accent/60",
                      disabled && "cursor-default opacity-50",
                    )}
                  >
                    <span>{summary.displayName}</span>
                    <span className="text-ui-meta text-muted-foreground">
                      {t("skillsCli.batch.linkable", {
                        count: summary.linkableCount,
                      })}
                      {" · "}
                      {t("skillsCli.batch.managed", {
                        count: summary.managedCount,
                      })}
                      {" · "}
                      {t("skillsCli.batch.directCopy", {
                        count: summary.directCopyCount,
                      })}
                      {" · "}
                      {t("skillsCli.batch.blocked", {
                        count: summary.blockedCount,
                      })}
                    </span>
                  </MenuPrimitive.Item>
                );
              })}
            </MenuPrimitive.Popup>
          </MenuPrimitive.Positioner>
        </MenuPrimitive.Portal>
      </MenuPrimitive.Root>
      <Button
        type="button"
        variant="outline"
        className="min-h-10"
        disabled={mutationsLocked || !unlinkEnabled}
        onClick={onUnlink}
      >
        {t("skillsCli.batch.unlink")}
      </Button>
      <Button
        type="button"
        variant="outline"
        className="min-h-10"
        disabled={busy || exporting || selectedCount === 0}
        onClick={onExportSelected}
      >
        {t("skillsCli.batch.exportSelected")}
      </Button>
      <Button
        type="button"
        variant="destructive"
        className="min-h-10"
        disabled={mutationsLocked}
        onClick={onUninstall}
      >
        {t("skillsCli.batch.uninstall")}
      </Button>
      <button
        type="button"
        className={cn(
          ICON_HIT,
          "ml-auto rounded-md text-muted-foreground hover:bg-muted/60 hover:text-foreground focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        )}
        onClick={onClear}
        aria-label={t("skillsCli.batch.clear")}
      >
        <X className="mx-auto size-4" />
      </button>
    </div>
  );
}
