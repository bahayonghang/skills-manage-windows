import type { ReactNode } from "react";
import { Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { DeleteCentralSkillPreview } from "@/types";
import type { MissingDecision } from "@/components/central/repositorySyncUtils";

export function ActionGroup({
  disabled,
  children,
}: {
  disabled?: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className={`flex flex-wrap gap-1 text-xs ${disabled ? "opacity-60" : ""}`}
    >
      {children}
    </div>
  );
}

export function ActionButton({
  selected,
  disabled,
  onClick,
  children,
}: {
  selected: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className={`rounded-lg border px-2 py-1 ${
        selected
          ? "border-primary bg-primary text-primary-foreground"
          : "border-border text-muted-foreground"
      }`}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

export function RenameField({
  value,
  error,
  onChange,
}: {
  value: string;
  error?: string;
  onChange: (value: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="mt-2 space-y-1">
      <input
        className={`w-full rounded-lg border bg-background px-3 py-2 text-sm ${
          error ? "border-destructive" : "border-border"
        }`}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        aria-label={t("central.repositorySyncRenameLabel")}
        aria-invalid={Boolean(error)}
      />
      {error && (
        <p className="text-xs text-destructive-text" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

export function DeleteOldSkillAction({
  selected,
  disabled,
  reason,
  onClick,
}: {
  selected: boolean;
  disabled: boolean;
  reason: string | null;
  onClick: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="mt-3 border-t border-border/70 pt-3">
      <Button
        type="button"
        variant={selected ? "destructive" : "outline"}
        size="sm"
        disabled={disabled}
        onClick={onClick}
      >
        <Trash2 className="size-3.5" />
        {t("central.repositorySyncDeleteOldSkill")}
      </Button>
      <p className="mt-1 text-xs text-muted-foreground">
        {reason ?? t("central.repositorySyncDeleteOldSkillDesc")}
      </p>
    </div>
  );
}

export function MissingChoice({
  decision,
  item,
  onChange,
}: {
  decision: MissingDecision;
  item: DeleteCentralSkillPreview | null;
  onChange: (decision: MissingDecision) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="grid grid-cols-2 rounded-xl border border-border/70 bg-muted/20 p-1 text-xs">
      {(["keep", "delete"] as MissingDecision[]).map((next) => (
        <button
          key={next}
          type="button"
          className={`rounded-lg px-3 py-1.5 font-medium ${
            decision === next
              ? next === "delete"
                ? "bg-destructive text-destructive-foreground"
                : "bg-primary text-primary-foreground"
              : "text-muted-foreground hover:bg-background"
          }`}
          disabled={next === "delete" && !item}
          onClick={() => onChange(next)}
        >
          {t(
            next === "delete"
              ? "central.remoteMissingDelete"
              : "central.remoteMissingKeep",
          )}
        </button>
      ))}
    </div>
  );
}

export function EmptyTab() {
  const { t } = useTranslation();
  return (
    <p className="rounded-xl border border-dashed border-border bg-muted/20 p-4 text-sm text-muted-foreground">
      {t("central.repositorySyncTabEmpty")}
    </p>
  );
}
