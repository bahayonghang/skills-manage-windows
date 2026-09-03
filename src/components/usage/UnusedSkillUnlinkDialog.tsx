import { useEffect, useMemo, useState } from "react";
import { Loader2, Unlink } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
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
import {
  centralTargets,
  platformTargets,
  type UnlinkTarget,
} from "@/components/usage/unusedUnlinkTargets";
import { formatBackendError } from "@/lib/backendError";
import { cn } from "@/lib/utils";
import { unlinkActionKey } from "@/stores/usageStore";
import type {
  UnusedSkillEntry,
  UnusedUnlinkRequest,
  UnusedUnlinkResult,
} from "@/types/usage";

interface UnusedSkillUnlinkDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Central 或平台条目的原始数据（由面板从当前报告查找传入，刷新后自然收敛） */
  entry: UnusedSkillEntry;
  onUnlinkAgents: (
    targets: UnusedUnlinkRequest[],
  ) => Promise<UnusedUnlinkResult[]>;
  /** 进行中的 unlink 动作 key 集合，逐行驱动 spinner */
  pendingUnlinkKeys?: Record<string, boolean>;
}

/** 选择/结果统一以 rowId 优先，Central 行无 rowId 时退化为 agentId。 */
function targetKey(target: UnlinkTarget): string {
  return target.rowId ?? target.agentId;
}

function resultKey(result: UnusedUnlinkResult): string {
  return result.rowId ?? result.agentId;
}

/**
 * Unused skills 的 unlink 弹窗：单弹窗服务 Central 与平台两种条目，内部先归一为
 * UnlinkTarget 模型（Central 取 entry.agents，平台取 entry.installs 全量）。
 * 默认不勾选任何项；全选只作用于 disabledReason === null 的项；确认按钮 destructive
 * 并展示选中数量，选中 ≥1 才可用、执行中 loading。部分失败时弹窗保留，失败行内联
 * 原因并重置勾选为仅失败项（便于直接重试）。
 */
export function UnusedSkillUnlinkDialog({
  open,
  onOpenChange,
  entry,
  onUnlinkAgents,
  pendingUnlinkKeys = {},
}: UnusedSkillUnlinkDialogProps) {
  const { t } = useTranslation();
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
  const [results, setResults] = useState<UnusedUnlinkResult[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);

  const targets = useMemo(
    () =>
      entry.origin === "central"
        ? centralTargets(entry)
        : platformTargets(entry),
    [entry],
  );
  const enabledKeys = useMemo(
    () =>
      targets
        .filter((target) => target.disabledReason === null)
        .map(targetKey),
    [targets],
  );

  useEffect(() => {
    if (!open) return;
    setSelectedKeys(new Set());
    setResults(null);
    setBusy(false);
    setDialogError(null);
  }, [open]);

  const selectedCount = enabledKeys.filter((key) =>
    selectedKeys.has(key),
  ).length;
  const allSelected =
    enabledKeys.length > 0 && selectedCount === enabledKeys.length;
  const someSelected = selectedCount > 0 && !allSelected;

  function toggleAll(checked: boolean) {
    setSelectedKeys((prev) => {
      const next = new Set(prev);
      for (const key of enabledKeys) {
        if (checked) next.add(key);
        else next.delete(key);
      }
      return next;
    });
  }

  function toggleTarget(target: UnlinkTarget, checked: boolean) {
    setSelectedKeys((prev) => {
      const next = new Set(prev);
      if (checked) next.add(targetKey(target));
      else next.delete(targetKey(target));
      return next;
    });
  }

  async function handleConfirm() {
    const selected = targets.filter(
      (target) =>
        target.disabledReason === null && selectedKeys.has(targetKey(target)),
    );
    if (selected.length === 0 || busy) return;
    setBusy(true);
    setDialogError(null);
    try {
      const nextResults = await onUnlinkAgents(
        selected.map((target) => ({
          skillId: target.skillId,
          agentId: target.agentId,
          rowId: target.rowId,
        })),
      );
      setResults(nextResults);
      if (nextResults.every((result) => result.ok)) {
        onOpenChange(false);
      } else {
        // 部分失败：同一次提交内把勾选重置为仅失败项，避免错误先于计数更新。
        setSelectedKeys(
          new Set(nextResults.filter((result) => !result.ok).map(resultKey)),
        );
      }
    } catch (error) {
      setDialogError(
        t("skillUsage.unused.unlink.error", {
          error: formatBackendError(error, t),
        }),
      );
    } finally {
      setBusy(false);
    }
  }

  const failedByKey = new Map(
    (results ?? [])
      .filter((result) => !result.ok)
      .map((result) => [resultKey(result), result]),
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        data-testid="unused-unlink-dialog"
        className="sm:max-w-md"
      >
        <DialogHeader>
          <DialogTitle>
            {t("skillUsage.unused.unlink.dialog.title", {
              skill: entry.name,
            })}
          </DialogTitle>
          <DialogDescription>
            {entry.origin === "central"
              ? t("skillUsage.unused.unlink.dialog.descriptionCentral")
              : t("skillUsage.unused.unlink.dialog.descriptionPlatform")}
          </DialogDescription>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-3">
          {targets.length === 0 ? (
            <p className="px-1 py-2 text-sm text-muted-foreground">
              {t("skillUsage.unused.noAgents")}
            </p>
          ) : (
            <>
              <label
                data-testid="unused-unlink-select-all"
                className={cn(
                  "flex cursor-pointer items-center gap-2 rounded-lg border border-border bg-muted/30 px-3 py-2 text-sm font-medium",
                  busy && "pointer-events-none opacity-60",
                )}
              >
                <Checkbox
                  checked={allSelected}
                  indeterminate={someSelected}
                  disabled={busy || enabledKeys.length === 0}
                  onCheckedChange={(checked) => toggleAll(!!checked)}
                  aria-label={t("skillUsage.unused.unlink.dialog.selectAll")}
                />
                <span>{t("skillUsage.unused.unlink.dialog.selectAll")}</span>
              </label>

              <ul className="max-h-[22rem] space-y-1 overflow-auto pr-1">
                {targets.map((target) => {
                  const disabled = target.disabledReason !== null;
                  const key = targetKey(target);
                  const failed = failedByKey.get(key);
                  const pending =
                    pendingUnlinkKeys[
                      unlinkActionKey(
                        target.agentId,
                        target.skillId,
                        target.rowId,
                      )
                    ] ?? false;
                  return (
                    <li
                      key={key}
                      data-testid={
                        disabled
                          ? `unused-unlink-option-disabled-${target.agentId}`
                          : `unused-unlink-option-${target.agentId}`
                      }
                      title={
                        disabled && target.disabledReason
                          ? t(
                              `skillUsage.unused.unlink.${target.disabledReason}`,
                            )
                          : undefined
                      }
                      className={cn(
                        "flex items-center gap-2 rounded-lg border border-border bg-background px-3 py-2",
                        disabled && "opacity-60",
                      )}
                    >
                      <label
                        className={cn(
                          "flex min-w-0 flex-1 cursor-pointer items-center gap-2",
                          disabled && "cursor-not-allowed",
                        )}
                      >
                        <Checkbox
                          checked={selectedKeys.has(key)}
                          disabled={disabled || busy}
                          onCheckedChange={(checked) =>
                            toggleTarget(target, !!checked)
                          }
                          aria-label={target.agentId}
                        />
                        <span className="truncate text-sm text-foreground">
                          {target.agentId}
                        </span>
                        {pending && (
                          <Loader2
                            aria-hidden
                            className="size-3.5 animate-spin motion-reduce:animate-none"
                          />
                        )}
                      </label>
                      {disabled && target.disabledReason && (
                        <span className="shrink-0 text-xs text-muted-foreground">
                          {t(
                            `skillUsage.unused.unlink.${target.disabledReason}`,
                          )}
                        </span>
                      )}
                      {failed && (
                        <span
                          data-testid={`unused-unlink-error-${target.agentId}`}
                          className="shrink-0 text-xs text-destructive-text"
                        >
                          {failed.error}
                        </span>
                      )}
                    </li>
                  );
                })}
              </ul>

              {dialogError && (
                <p
                  className="px-1 text-xs text-destructive-text"
                  role="alert"
                >
                  {dialogError}
                </p>
              )}
            </>
          )}
        </DialogBody>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={busy}
          >
            {t("skillUsage.unused.unlink.dialog.cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={() => void handleConfirm()}
            disabled={busy || selectedCount < 1}
            data-testid="unused-unlink-confirm"
          >
            {busy ? (
              <>
                <Loader2 className="size-3.5 animate-spin motion-reduce:animate-none" />
                {t("common.loading")}
              </>
            ) : (
              <>
                <Unlink className="size-3.5" />
                {t("skillUsage.unused.unlink.dialog.confirm", {
                  count: selectedCount,
                })}
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
