import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { Project } from "@/types";

interface ProjectRemoveDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  project: Project | null;
  isRemoving: boolean;
  onConfirm: (uninstallSkills: boolean) => void | Promise<void>;
}

export function ProjectRemoveDialog({
  open,
  onOpenChange,
  project,
  isRemoving,
  onConfirm,
}: ProjectRemoveDialogProps) {
  const { t } = useTranslation();
  const [uninstallSkills, setUninstallSkills] = useState(false);

  useEffect(() => {
    if (open) {
      setUninstallSkills(false);
    }
  }, [open]);

  if (!project) return null;

  const skillCount = project.skillCount ?? 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {t("projects.removeTitle", { name: project.name })}
          </DialogTitle>
          <DialogDescription>
            {t("projects.removeDescription")}
          </DialogDescription>
        </DialogHeader>

        <div className="text-xs text-muted-foreground font-mono break-all px-1">
          {project.path}
        </div>

        <label className="flex items-start gap-2 px-1 cursor-pointer select-none">
          <Checkbox
            checked={uninstallSkills}
            onCheckedChange={(value) => setUninstallSkills(value === true)}
            disabled={skillCount === 0}
            aria-label={t("projects.removeAlsoUninstall", { count: skillCount })}
          />
          <span className="text-sm leading-tight">
            <span className="block">
              {t("projects.removeAlsoUninstall", { count: skillCount })}
            </span>
            <span className="block text-xs text-muted-foreground mt-0.5">
              {t("projects.removeAlsoUninstallHint")}
            </span>
          </span>
        </label>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={isRemoving}
          >
            {t("common.cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={() => onConfirm(uninstallSkills)}
            disabled={isRemoving}
          >
            {isRemoving ? (
              <Loader2 className="size-3.5 mr-1 animate-spin" />
            ) : null}
            {t("projects.removeConfirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
