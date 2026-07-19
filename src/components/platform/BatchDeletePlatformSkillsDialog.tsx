import { AlertTriangle, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { formatPathForDisplay } from "@/lib/path";
import type { ScannedSkill } from "@/types";

interface BatchDeletePlatformSkillsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  platformName: string;
  skills: ScannedSkill[];
  isDeleting?: boolean;
  onConfirm: () => Promise<void> | void;
}

export function BatchDeletePlatformSkillsDialog({
  open,
  onOpenChange,
  platformName,
  skills,
  isDeleting = false,
  onConfirm,
}: BatchDeletePlatformSkillsDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {t("platform.batchDeleteDialogTitle", { count: skills.length })}
          </DialogTitle>
          <DialogDescription>
            {t("platform.batchDeleteDialogDesc", {
              count: skills.length,
              platform: platformName,
            })}
          </DialogDescription>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          <div className="rounded-xl border border-destructive/25 bg-destructive/5 p-3 text-sm">
            <div className="flex items-start gap-2">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive-text" />
              <div className="space-y-1">
                <div className="font-medium text-foreground">
                  {t("platform.batchDeleteDialogWarningTitle")}
                </div>
                <p className="text-muted-foreground">
                  {t("platform.batchDeleteDialogWarning")}
                </p>
              </div>
            </div>
          </div>

          <div className="space-y-2">
            <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("platform.batchDeleteSelectedSection")}
            </div>
            <div className="max-h-72 space-y-2 overflow-auto rounded-xl border border-border bg-muted/20 p-3">
              {skills.map((skill) => (
                <div
                  key={skill.row_id ?? `${skill.id}::${skill.dir_path}`}
                  className="rounded-lg border border-border/70 bg-background/80 p-3"
                >
                  <div className="text-sm font-medium text-foreground">{skill.name}</div>
                  <div className="mt-1 truncate text-xs text-muted-foreground">
                    {formatPathForDisplay(skill.dir_path)}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </DialogBody>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            disabled={isDeleting}
            onClick={() => onOpenChange(false)}
          >
            {t("platform.duplicatesCancel")}
          </Button>
          <Button
            type="button"
            variant="destructive"
            disabled={isDeleting || skills.length === 0}
            onClick={() => void onConfirm()}
            data-testid="platform-batch-delete-confirm"
          >
            <Trash2 className="size-3.5" aria-hidden />
            {isDeleting
              ? t("platform.batchDeleting")
              : t("platform.batchDeleteConfirm", { count: skills.length })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
