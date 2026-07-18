import { useState, useEffect } from "react";
import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogBody,
  DialogFooter,
  DialogClose,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

// ─── Props ────────────────────────────────────────────────────────────────────

interface AddDirectoryDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdd: (path: string) => Promise<void>;
}

// ─── AddDirectoryDialog ───────────────────────────────────────────────────────

export function AddDirectoryDialog({
  open,
  onOpenChange,
  onAdd,
}: AddDirectoryDialogProps) {
  const { t } = useTranslation();
  const [path, setPath] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Reset form when dialog opens.
  useEffect(() => {
    if (open) {
      setPath("");
      setValidationError(null);
      setError(null);
    }
  }, [open]);

  async function handleSubmit() {
    const trimmedPath = path.trim();
    if (!trimmedPath) {
      setValidationError(t("addDir.pathRequired"));
      return;
    }

    setIsSubmitting(true);
    setValidationError(null);
    setError(null);

    try {
      await onAdd(trimmedPath);
      onOpenChange(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsSubmitting(false);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter" && !isSubmitting) {
      handleSubmit();
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("addDir.title")}</DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          <DialogDescription>
            {t("addDir.desc")}
          </DialogDescription>

          {/* Path field */}
          <div className="space-y-1.5">
            <label htmlFor="dir-path" className="text-sm font-medium">
              {t("addDir.pathLabel")} <span className="text-destructive-text">*</span>
            </label>
            <Input
              id="dir-path"
              placeholder={t("addDir.pathPlaceholder")}
              value={path}
              onChange={(e) => {
                setPath(e.target.value);
                if (validationError) setValidationError(null);
              }}
              onKeyDown={handleKeyDown}
              disabled={isSubmitting}
              autoFocus
            />
            {validationError && (
              <p className="text-xs text-destructive-text" role="alert">
                {validationError}
              </p>
            )}
          </div>

          {/* Backend error */}
          {error && (
            <p className="text-xs text-destructive-text" role="alert">
              {error}
            </p>
          )}
        </DialogBody>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={isSubmitting}
          >
            {t("addDir.cancel")}
          </Button>
          <Button onClick={handleSubmit} disabled={isSubmitting}>
            {isSubmitting ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("addDir.adding")}
              </>
            ) : (
              t("addDir.add")
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
