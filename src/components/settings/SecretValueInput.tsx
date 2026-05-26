import { Eye, EyeOff, Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

const SAVED_SECRET_MASK = "••••••••••••";

interface SecretValueInputProps {
  id: string;
  label: string;
  value: string;
  configured: boolean;
  disabled?: boolean;
  placeholder: string;
  revealScopeKey: string;
  inputShowLabel: string;
  inputHideLabel: string;
  savedRevealLabel: string;
  savedHideLabel: string;
  savedHiddenHint: string;
  savedRevealedHint: string;
  inputReplacementHint?: string;
  onChange: (value: string) => void;
  onRevealSaved: () => Promise<string | null>;
  onRevealError?: (error: string) => void;
}

export function SecretValueInput({
  id,
  label,
  value,
  configured,
  disabled = false,
  placeholder,
  revealScopeKey,
  inputShowLabel,
  inputHideLabel,
  savedRevealLabel,
  savedHideLabel,
  savedHiddenHint,
  savedRevealedHint,
  inputReplacementHint,
  onChange,
  onRevealSaved,
  onRevealError,
}: SecretValueInputProps) {
  const { t } = useTranslation();
  const [showInput, setShowInput] = useState(false);
  const [revealedSavedValue, setRevealedSavedValue] = useState<string | null>(null);
  const [isRevealing, setIsRevealing] = useState(false);
  const hasInput = value.length > 0;
  const isShowingSavedValue = !hasInput && Boolean(revealedSavedValue);

  useEffect(() => {
    if (hasInput) {
      setRevealedSavedValue(null);
    }
  }, [hasInput]);

  useEffect(() => {
    setShowInput(false);
    setRevealedSavedValue(null);
  }, [configured, revealScopeKey]);

  async function handleToggleSecretVisibility() {
    if (hasInput) {
      setShowInput((current) => !current);
      return;
    }

    if (!configured) {
      return;
    }

    if (revealedSavedValue) {
      setRevealedSavedValue(null);
      return;
    }

    onRevealError?.("");
    setIsRevealing(true);
    try {
      const revealed = await onRevealSaved();
      setRevealedSavedValue(revealed);
      if (!revealed) {
        onRevealError?.(t("settings.secretRevealMissing"));
      }
    } catch (error) {
      setRevealedSavedValue(null);
      onRevealError?.(String(error));
    } finally {
      setIsRevealing(false);
    }
  }

  const inputType = hasInput ? (showInput ? "text" : "password") : "text";
  const displayValue = hasInput ? value : revealedSavedValue ?? "";
  const displayPlaceholder = configured ? SAVED_SECRET_MASK : placeholder;
  const buttonLabel = hasInput
    ? showInput
      ? inputHideLabel
      : inputShowLabel
    : revealedSavedValue
      ? savedHideLabel
      : savedRevealLabel;
  const hint = hasInput
    ? inputReplacementHint
    : revealedSavedValue
      ? savedRevealedHint
      : configured
        ? savedHiddenHint
        : null;

  return (
    <div>
      <label htmlFor={id} className="mb-1 block text-xs text-muted-foreground">
        {label}
      </label>
      <div className="relative">
        <Input
          id={id}
          type={inputType}
          value={displayValue}
          placeholder={displayPlaceholder}
          readOnly={isShowingSavedValue}
          className="pr-10 font-mono text-sm placeholder:text-muted-foreground/80"
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
        />
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="absolute inset-y-0 right-0 h-full rounded-l-none px-2 text-muted-foreground hover:text-foreground"
          disabled={disabled || isRevealing || (!hasInput && !configured)}
          aria-label={buttonLabel}
          onClick={handleToggleSecretVisibility}
        >
          {isRevealing ? (
            <Loader2 className="size-4 animate-spin" />
          ) : hasInput ? (
            showInput ? (
              <EyeOff className="size-4" />
            ) : (
              <Eye className="size-4" />
            )
          ) : revealedSavedValue ? (
            <EyeOff className="size-4" />
          ) : (
            <Eye className="size-4" />
          )}
        </Button>
      </div>
      {hint ? <p className="mt-1 text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  );
}
