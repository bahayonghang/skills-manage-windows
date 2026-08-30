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
  unlinkMenuOpen: boolean;
  onUnlinkMenuOpenChange: (open: boolean) => void;
  onLink: (agentId: string) => void;
  onUnlink: () => void;
  onUnlinkPlatform: (agentId: string) => void;
  onUpdate: () => void;
  onExportSelected: () => void;
  onUninstall: () => void;
  onClear: () => void;
  mutationLockReason?: string;
  updateLockReason?: string;
}

export const ICON_HIT =
  "relative size-8 after:absolute after:left-1/2 after:top-1/2 after:size-10 after:-translate-x-1/2 after:-translate-y-1/2 after:content-['']";

const MENU_POPUP_CLASS = cn(
  "min-w-[16rem] rounded-lg bg-popover p-1 text-sm text-popover-foreground shadow-md ring-1 ring-foreground/10 outline-none",
  "data-[starting-style]:animate-in data-[starting-style]:fade-in-0 data-[starting-style]:zoom-in-95",
  "data-[ending-style]:animate-out data-[ending-style]:fade-out-0 data-[ending-style]:zoom-out-95",
  "animation-duration-100",
);

export function SkillsCliBatchBar({
  selectedCount,
  summaries,
  unlinkEnabled,
  busy,
  exporting,
  linkMenuOpen,
  onLinkMenuOpenChange,
  unlinkMenuOpen,
  onUnlinkMenuOpenChange,
  onLink,
  onUnlink,
  onUnlinkPlatform,
  onUpdate,
  onExportSelected,
  onUninstall,
  onClear,
  mutationLockReason,
  updateLockReason,
}: SkillsCliBatchBarProps) {
  const { t } = useTranslation();
  if (selectedCount <= 0) {
    return null;
  }
  const placementLocked = busy || Boolean(mutationLockReason);
  const updateLocked = busy || Boolean(updateLockReason);
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
      <Button
        type="button"
        variant="outline"
        className="min-h-10"
        disabled={updateLocked}
        title={updateLockReason}
        onClick={onUpdate}
        aria-label={t("skillsCli.batch.updateAria")}
        data-testid="skills-cli-batch-update"
      >
        {t("skillsCli.batch.update")}
      </Button>
      <MenuPrimitive.Root
        open={linkMenuOpen}
        onOpenChange={onLinkMenuOpenChange}
      >
        <MenuPrimitive.Trigger
          disabled={placementLocked || !anyLinkable}
          render={
            <Button
              type="button"
              variant="outline"
              className="min-h-10"
              title={mutationLockReason}
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
            <MenuPrimitive.Popup className={MENU_POPUP_CLASS}>
              {summaries.map((summary) => {
                const disabled = summary.linkableCount === 0 || placementLocked;
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
      <MenuPrimitive.Root
        open={unlinkMenuOpen}
        onOpenChange={onUnlinkMenuOpenChange}
      >
        <MenuPrimitive.Trigger
          disabled={placementLocked || !unlinkEnabled}
          render={
            <Button
              type="button"
              variant="outline"
              className="min-h-10"
              title={mutationLockReason}
              aria-label={t("skillsCli.batch.unlinkMenuAria")}
              data-testid="skills-cli-batch-unlink"
            />
          }
        >
          {t("skillsCli.batch.unlink")}
        </MenuPrimitive.Trigger>
        <MenuPrimitive.Portal>
          <MenuPrimitive.Positioner
            align="start"
            sideOffset={4}
            className="z-50 outline-none"
          >
            <MenuPrimitive.Popup className={MENU_POPUP_CLASS}>
              {summaries.map((summary) => {
                const disabled = summary.managedCount === 0 || placementLocked;
                return (
                  <MenuPrimitive.Item
                    key={summary.agentId}
                    disabled={disabled}
                    label={t("skillsCli.batch.unlinkTargetAria", {
                      name: summary.displayName,
                      managed: summary.managedCount,
                      copies: summary.directCopyCount,
                      blocked: summary.blockedCount,
                    })}
                    onClick={() => onUnlinkPlatform(summary.agentId)}
                    data-testid={`skills-cli-batch-unlink-${summary.agentId}`}
                    className={cn(
                      "flex cursor-pointer flex-col gap-0.5 rounded-md px-2.5 py-1.5 outline-none data-[highlighted]:bg-accent/60",
                      disabled && "cursor-default opacity-50",
                    )}
                  >
                    <span>{summary.displayName}</span>
                    <span className="text-ui-meta text-muted-foreground">
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
              <MenuPrimitive.Item
                disabled={placementLocked || !unlinkEnabled}
                label={t("skillsCli.batch.unlinkAllAria")}
                onClick={onUnlink}
                data-testid="skills-cli-batch-unlink-all"
                className={cn(
                  "flex cursor-pointer flex-col gap-0.5 rounded-md px-2.5 py-1.5 outline-none data-[highlighted]:bg-accent/60",
                  (placementLocked || !unlinkEnabled) && "cursor-default opacity-50",
                )}
              >
                {t("skillsCli.batch.unlinkAll")}
              </MenuPrimitive.Item>
            </MenuPrimitive.Popup>
          </MenuPrimitive.Positioner>
        </MenuPrimitive.Portal>
      </MenuPrimitive.Root>
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
        disabled={placementLocked}
        title={mutationLockReason}
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
