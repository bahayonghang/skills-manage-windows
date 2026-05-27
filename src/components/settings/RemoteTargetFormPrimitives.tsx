import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { SshAuthMethod } from "@/types";

interface FormActionButtonsProps {
  isSaving: boolean;
  onCancel: () => void;
  onSave: () => void;
}

export function FormActionButtons({
  isSaving,
  onCancel,
  onSave,
}: FormActionButtonsProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-2 sm:flex-row md:col-span-2 md:items-end">
      <Button
        type="button"
        variant="outline"
        className="flex-1"
        disabled={isSaving}
        onClick={onCancel}
      >
        {t("common.cancel")}
      </Button>
      <Button
        type="button"
        className="flex-1"
        disabled={isSaving}
        onClick={onSave}
      >
        {isSaving ? <Loader2 className="size-3.5 animate-spin" /> : null}
        {t("targets.saveChanges")}
      </Button>
    </div>
  );
}

interface TargetTextFieldProps {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export function TargetTextField({
  id,
  label,
  value,
  onChange,
  placeholder,
}: TargetTextFieldProps) {
  return (
    <div className="space-y-1">
      <label htmlFor={id} className="text-xs text-muted-foreground">
        {label}
      </label>
      <Input
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
      />
    </div>
  );
}

interface SshAuthMethodButtonsProps {
  id: string;
  value: SshAuthMethod;
  onChange: (value: SshAuthMethod) => void;
}

export function SshAuthMethodButtons({
  id,
  value,
  onChange,
}: SshAuthMethodButtonsProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-1">
      <span id={id} className="text-xs text-muted-foreground">
        {t("targets.authMethodLabel")}
      </span>
      <div
        className="grid grid-cols-2 gap-1 rounded-md border border-border p-1"
        aria-labelledby={id}
      >
        {(["key", "password"] as const).map((method) => (
          <button
            key={method}
            type="button"
            className={`rounded px-2 py-2 text-xs transition-colors ${
              value === method
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-muted hover:text-foreground"
            }`}
            aria-pressed={value === method}
            onClick={() => onChange(method)}
          >
            {method === "key"
              ? t("targets.authKey")
              : t("targets.authPassword")}
          </button>
        ))}
      </div>
    </div>
  );
}
