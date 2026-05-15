import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { Project } from "@/types";

interface ProjectRenameDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  project: Project | null;
  isRenaming: boolean;
  onConfirm: (name: string) => void | Promise<void>;
}

export function ProjectRenameDialog({
  open,
  onOpenChange,
  project,
  isRenaming,
  onConfirm,
}: ProjectRenameDialogProps) {
  const { t } = useTranslation();
  const [value, setValue] = useState("");

  useEffect(() => {
    if (open && project) {
      setValue(project.name);
    }
  }, [open, project]);

  if (!project) return null;

  const trimmed = value.trim();
  const canSubmit = trimmed.length > 0 && trimmed !== project.name && !isRenaming;

  const handleSubmit = () => {
    if (!canSubmit) return;
    void onConfirm(trimmed);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t("projects.renameTitle")}</DialogTitle>
        </DialogHeader>

        <Input
          value={value}
          autoFocus
          placeholder={t("projects.renamePlaceholder")}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              handleSubmit();
            }
          }}
          disabled={isRenaming}
          aria-label={t("projects.renamePlaceholder")}
        />

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={isRenaming}
          >
            {t("common.cancel")}
          </Button>
          <Button onClick={handleSubmit} disabled={!canSubmit}>
            {isRenaming ? (
              <Loader2 className="size-3.5 mr-1 animate-spin" />
            ) : null}
            {t("projects.renameSave")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
