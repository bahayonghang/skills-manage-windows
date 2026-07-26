import { type ReactNode } from "react";

import { Input } from "@/components/ui/input";

const SAVED_SECRET_MASK = "••••••••••••";

interface SecretValueInputProps {
  id: string;
  label: string;
  labelAction?: ReactNode;
  value: string;
  configured: boolean;
  disabled?: boolean;
  placeholder: string;
  savedHiddenHint: string;
  inputReplacementHint?: string;
  onChange: (value: string) => void;
}

export function SecretValueInput({
  id,
  label,
  labelAction,
  value,
  configured,
  disabled = false,
  placeholder,
  savedHiddenHint,
  inputReplacementHint,
  onChange,
}: SecretValueInputProps) {
  const hasInput = value.length > 0;
  const displayPlaceholder = configured ? SAVED_SECRET_MASK : placeholder;
  const hint = hasInput
    ? inputReplacementHint
    : configured
      ? savedHiddenHint
      : null;

  return (
    <div>
      <div className="mb-1 flex flex-wrap items-center justify-between gap-2">
        <label htmlFor={id} className="block text-xs text-muted-foreground">
          {label}
        </label>
        {labelAction}
      </div>
      <Input
        id={id}
        type="password"
        value={value}
        placeholder={displayPlaceholder}
        className="font-mono text-sm placeholder:text-muted-foreground"
        disabled={disabled}
        autoComplete="new-password"
        onChange={(event) => onChange(event.target.value)}
      />
      {hint ? <p className="mt-1 text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  );
}
