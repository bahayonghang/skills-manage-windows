import { useCallback } from "react";
import { GitBranch, PackagePlus, FolderArchive } from "lucide-react";
import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import {
  menuPopupClassName,
  menuItemClassName,
} from "@/components/central/centralMenuClassNames";

export type SkillImportIntent = "github" | "local_zip";

export interface SkillImportLauncherProps {
  t: TFunction;
  /** True when the active target is SSH/WSL — ZIP intent is disabled. */
  isRemoteTarget: boolean;
  /** Fired when the user picks an import source. */
  onOpenIntent: (intent: SkillImportIntent) => void;
}

/**
 * Unified "Add Skill" entry point.
 *
 * Replaces the single GitHub-import button in the Central header with a
 * compact dropdown that routes to either the GitHub wizard (existing flow)
 * or the local ZIP wizard (new flow). The launcher never opens a modal
 * inside another modal; it only fires `onOpenIntent` so the parent can
 * mount the right wizard.
 *
 * The launcher is also the single entry point future deep-link prefill
 * will target.
 */
export function SkillImportLauncher({
  t,
  isRemoteTarget,
  onOpenIntent,
}: SkillImportLauncherProps) {
  const handleSelect = useCallback(
    (intent: SkillImportIntent) => {
      onOpenIntent(intent);
    },
    [onOpenIntent],
  );

  return (
    <MenuPrimitive.Root>
      <MenuPrimitive.Trigger
        render={
          <Button
            variant="outline"
            className="h-9 rounded-xl pl-3 pr-3.5 transition-[scale,background-color,border-color,box-shadow,color] active:scale-[0.96]"
            data-testid="central-add-skill-launcher"
          >
            <PackagePlus className="size-3.5" />
            {t("central.addSkillLauncher.label")}
          </Button>
        }
      />
      <MenuPrimitive.Portal>
        <MenuPrimitive.Positioner
          align="end"
          sideOffset={6}
          className="z-50 outline-none"
        >
          <MenuPrimitive.Popup
            data-testid="central-add-skill-launcher-menu"
            className={menuPopupClassName()}
          >
            <MenuPrimitive.Item
              data-testid="central-add-skill-github"
              onClick={() => handleSelect("github")}
              className={menuItemClassName()}
            >
              <GitBranch className="size-3.5 shrink-0" />
              <div className="flex flex-col">
                <span className="text-sm font-medium">
                  {t("central.addSkillLauncher.github")}
                </span>
                <span className="text-xs text-muted-foreground">
                  {t("central.addSkillLauncher.githubDesc")}
                </span>
              </div>
            </MenuPrimitive.Item>
            <div role="separator" className="my-1 h-px bg-border/60" />
            <MenuPrimitive.Item
              data-testid="central-add-skill-local-zip"
              onClick={() => handleSelect("local_zip")}
              disabled={isRemoteTarget}
              className={menuItemClassName()}
            >
              <FolderArchive className="size-3.5 shrink-0" />
              <div className="flex flex-col">
                <span className="text-sm font-medium">
                  {t("central.addSkillLauncher.localZip")}
                </span>
                <span className="text-xs text-muted-foreground">
                  {isRemoteTarget
                    ? t("central.addSkillLauncher.localZipRemoteDisabled")
                    : t("central.addSkillLauncher.localZipDesc")}
                </span>
              </div>
            </MenuPrimitive.Item>
          </MenuPrimitive.Popup>
        </MenuPrimitive.Positioner>
      </MenuPrimitive.Portal>
    </MenuPrimitive.Root>
  );
}
