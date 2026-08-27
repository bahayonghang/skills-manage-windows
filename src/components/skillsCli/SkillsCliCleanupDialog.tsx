import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  cleanupReasonI18nKey,
  defaultCleanupSelectedNames,
  type CleanupCandidate,
  type CleanupGroup,
} from "@/pages/skillsCliBatchModel";

export interface SkillsCliCleanupDialogProps {
  open: boolean;
  candidates: readonly CleanupCandidate[];
  busy: boolean;
  returnFocusRef?: RefObject<HTMLElement | null>;
  onOpenChange: (open: boolean) => void;
  onConfirm: (skillNames: string[]) => void;
}

function namesInGroup(
  candidates: readonly CleanupCandidate[],
  group: CleanupGroup,
): string[] {
  return candidates
    .filter((candidate) => candidate.group === group)
    .map((candidate) => candidate.name);
}

export function SkillsCliCleanupDialog({
  open,
  candidates,
  busy,
  returnFocusRef,
  onOpenChange,
  onConfirm,
}: SkillsCliCleanupDialogProps) {
  const { t } = useTranslation();
  const titleRef = useRef<HTMLHeadingElement | null>(null);
  const [selected, setSelected] = useState<ReadonlySet<string>>(() => new Set());
  const candidateKey = candidates
    .map((candidate) => `${candidate.name}:${candidate.group}`)
    .join("\0");

  useEffect(() => {
    if (!open) {
      return;
    }
    setSelected(new Set(defaultCleanupSelectedNames(candidates)));
    // Re-seed only when the dialog opens or the candidate identity changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- see above
  }, [open, candidateKey]);

  useEffect(() => {
    if (open) {
      titleRef.current?.focus();
    }
  }, [open]);

  const stale = useMemo(
    () => candidates.filter((candidate) => candidate.group === "stale"),
    [candidates],
  );
  const platformUnavailable = useMemo(
    () =>
      candidates.filter((candidate) => candidate.group === "platformUnavailable"),
    [candidates],
  );
  const platformSelected = platformUnavailable.some((candidate) =>
    selected.has(candidate.name),
  );
  const confirmDisabled = busy || selected.size === 0;

  function toggleName(name: string, next: boolean) {
    setSelected((current) => {
      const copy = new Set(current);
      if (next) {
        copy.add(name);
      } else {
        copy.delete(name);
      }
      return copy;
    });
  }

  function toggleGroup(group: CleanupGroup, next: boolean) {
    const names = namesInGroup(candidates, group);
    setSelected((current) => {
      const copy = new Set(current);
      for (const name of names) {
        if (next) {
          copy.add(name);
        } else {
          copy.delete(name);
        }
      }
      return copy;
    });
  }

  function groupChecked(group: CleanupGroup): boolean | "mixed" {
    const names = namesInGroup(candidates, group);
    if (names.length === 0) {
      return false;
    }
    const selectedCount = names.filter((name) => selected.has(name)).length;
    if (selectedCount === 0) {
      return false;
    }
    if (selectedCount === names.length) {
      return true;
    }
    return "mixed";
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (busy) {
          return;
        }
        onOpenChange(next);
      }}
    >
      <DialogContent
        className="sm:max-w-md"
        initialFocus={titleRef}
        finalFocus={returnFocusRef}
        aria-busy={busy}
      >
        <DialogHeader>
          <DialogTitle>
            <span ref={titleRef} tabIndex={-1} className="outline-none">
              {t("skillsCli.cleanup.title")}
            </span>
          </DialogTitle>
          <DialogDescription>{t("skillsCli.cleanup.description")}</DialogDescription>
        </DialogHeader>
        <DialogBody className="space-y-4">
          {stale.length > 0 ? (
            <CleanupGroupSection
              group="stale"
              title={t("skillsCli.cleanup.staleGroup")}
              countLabel={t("skillsCli.cleanup.staleCount", { count: stale.length })}
              candidates={stale}
              selected={selected}
              groupChecked={groupChecked("stale")}
              onToggleGroup={(next) => toggleGroup("stale", next)}
              onToggleName={toggleName}
            />
          ) : null}
          {platformUnavailable.length > 0 ? (
            <CleanupGroupSection
              group="platformUnavailable"
              title={t("skillsCli.cleanup.platformGroup")}
              countLabel={t("skillsCli.cleanup.platformCount", {
                count: platformUnavailable.length,
              })}
              candidates={platformUnavailable}
              selected={selected}
              groupChecked={groupChecked("platformUnavailable")}
              onToggleGroup={(next) => toggleGroup("platformUnavailable", next)}
              onToggleName={toggleName}
            />
          ) : null}
          {platformSelected ? (
            <p
              role="status"
              data-testid="skills-cli-cleanup-platform-risk"
              className="rounded-md border border-warning/30 bg-warning/10 p-2 text-sm text-warning-foreground"
            >
              {t("skillsCli.cleanup.platformRisk")}
            </p>
          ) : null}
        </DialogBody>
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            disabled={busy}
            onClick={() => onOpenChange(false)}
          >
            {t("common.cancel")}
          </Button>
          <Button
            type="button"
            variant="destructive"
            disabled={confirmDisabled}
            onClick={() => onConfirm([...selected])}
          >
            {t("skillsCli.cleanup.reviewUninstall")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function CleanupGroupSection({
  group,
  title,
  countLabel,
  candidates,
  selected,
  groupChecked,
  onToggleGroup,
  onToggleName,
}: {
  group: CleanupGroup;
  title: string;
  countLabel: string;
  candidates: readonly CleanupCandidate[];
  selected: ReadonlySet<string>;
  groupChecked: boolean | "mixed";
  onToggleGroup: (next: boolean) => void;
  onToggleName: (name: string, next: boolean) => void;
}) {
  const { t } = useTranslation();
  return (
    <section
      data-testid={`skills-cli-cleanup-group-${group}`}
      className="space-y-2"
    >
      <div className="flex items-center gap-2">
        <Checkbox
          checked={groupChecked === true}
          indeterminate={groupChecked === "mixed"}
          aria-label={t("skillsCli.cleanup.selectGroup", { group: title })}
          onCheckedChange={(value) => onToggleGroup(value === true)}
        />
        <h3 className="text-sm font-medium">{title}</h3>
        <span className="text-ui-meta tabular-nums text-muted-foreground">
          {countLabel}
        </span>
      </div>
      <ul className="space-y-2">
        {candidates.map((candidate) => (
          <li key={candidate.name} className="space-y-1">
            <label className="flex items-center gap-2 text-sm">
              <Checkbox
                checked={selected.has(candidate.name)}
                aria-label={t("skillsCli.cleanup.selectSkill", {
                  name: candidate.name,
                })}
                onCheckedChange={(value) =>
                  onToggleName(candidate.name, value === true)
                }
              />
              <span>{candidate.name}</span>
            </label>
            <ul className="pl-6 text-ui-meta text-muted-foreground">
              {candidate.reasons.map((reason) => (
                <li key={`${candidate.name}:${reason.platform}:${reason.reasonCode}`}>
                  {t("skillsCli.cleanup.reasonLine", {
                    platform: reason.platform,
                    reason: t(cleanupReasonI18nKey(reason.reasonCode)),
                  })}
                </li>
              ))}
            </ul>
          </li>
        ))}
      </ul>
    </section>
  );
}
