import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
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
import {
  DuplicatePlatformSkillGroup,
  getPlatformSkillRowKey,
} from "@/lib/platformDuplicateSkills";
import type { ScannedSkill } from "@/types";

interface DuplicatePlatformSkillsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  groups: DuplicatePlatformSkillGroup[];
  platformName: string;
  isSubmitting?: boolean;
  onConfirm: (rows: ScannedSkill[]) => Promise<void> | void;
}

function sourcePath(skill: ScannedSkill): string {
  return skill.source_root || skill.dir_path;
}

/**
 * @deprecated 使用 `UpdateCenterDialog` 的"平台冗余" Tab 代替
 * （plans/update-mechanism-overhaul-plan.md P7）。本组件保留以兼容历史调用方，
 * 将在下一个 minor release 删除。
 */
export function DuplicatePlatformSkillsDialog({
  open,
  onOpenChange,
  groups,
  platformName,
  isSubmitting = false,
  onConfirm,
}: DuplicatePlatformSkillsDialogProps) {
  const { t } = useTranslation();
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());

  const writableRows = useMemo(
    () => groups.flatMap((group) => group.writableRows),
    [groups]
  );

  useEffect(() => {
    if (!open) return;
    setSelectedKeys(new Set(writableRows.map(getPlatformSkillRowKey)));
  }, [open, writableRows]);

  const selectedRows = writableRows.filter((row) =>
    selectedKeys.has(getPlatformSkillRowKey(row))
  );

  function toggleRow(row: ScannedSkill, checked: boolean) {
    const key = getPlatformSkillRowKey(row);
    setSelectedKeys((current) => {
      const next = new Set(current);
      if (checked) {
        next.add(key);
      } else {
        next.delete(key);
      }
      return next;
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>
            {t("platform.duplicatesDialogTitle", { count: groups.length })}
          </DialogTitle>
          <DialogDescription>
            {t("platform.duplicatesDialogDesc", { platform: platformName })}
          </DialogDescription>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          {groups.map((group) => (
            <section
              key={group.skillId}
              className="rounded-lg border border-border bg-muted/20 p-3 space-y-3"
            >
              <div>
                <h3 className="font-medium text-sm">{group.name}</h3>
                <p className="text-xs text-muted-foreground">{group.skillId}</p>
              </div>

              <div className="space-y-2">
                <p className="text-xs font-medium text-foreground">
                  {t("platform.duplicatesWritableSection")}
                </p>
                {group.writableRows.map((row) => {
                  const key = getPlatformSkillRowKey(row);
                  return (
                    <label
                      key={key}
                      className="flex gap-3 rounded-md border border-border/70 bg-background/80 p-2 text-sm"
                    >
                      <Checkbox
                        aria-label={t("platform.duplicatesSelectRowLabel", {
                          skill: row.name,
                          path: formatPathForDisplay(row.dir_path),
                        })}
                        checked={selectedKeys.has(key)}
                        disabled={isSubmitting}
                        onCheckedChange={(checked) => toggleRow(row, checked === true)}
                      />
                      <span className="min-w-0 space-y-1">
                        <span className="block font-medium">
                          {t("platform.duplicatesPlatformCopy")}
                        </span>
                        <span className="block truncate text-xs text-muted-foreground">
                          {t("platform.duplicatesDirectory", {
                            path: formatPathForDisplay(row.dir_path),
                          })}
                        </span>
                      </span>
                    </label>
                  );
                })}
              </div>

              <div className="space-y-2">
                <p className="text-xs font-medium text-foreground">
                  {t("platform.duplicatesPluginSection")}
                </p>
                {group.pluginRows.map((row) => (
                  <div
                    key={getPlatformSkillRowKey(row)}
                    className="rounded-md border border-dashed border-border bg-background/60 p-2 text-sm"
                  >
                    <p className="font-medium">{t("platform.duplicatesReadOnlyCopy")}</p>
                    <p className="truncate text-xs text-muted-foreground">
                      {t("platform.duplicatesSourceRoot", {
                        path: formatPathForDisplay(sourcePath(row)),
                      })}
                    </p>
                    <p className="truncate text-xs text-muted-foreground">
                      {t("platform.duplicatesDirectory", {
                        path: formatPathForDisplay(row.dir_path),
                      })}
                    </p>
                  </div>
                ))}
              </div>
            </section>
          ))}
        </DialogBody>

        <DialogFooter>
          <Button variant="outline" disabled={isSubmitting} onClick={() => onOpenChange(false)}>
            {t("platform.duplicatesCancel")}
          </Button>
          <Button
            variant="destructive"
            disabled={isSubmitting || selectedRows.length === 0}
            onClick={() => void onConfirm(selectedRows)}
          >
            {isSubmitting
              ? t("platform.duplicatesCleaning")
              : t("platform.duplicatesConfirm", { count: selectedRows.length })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
