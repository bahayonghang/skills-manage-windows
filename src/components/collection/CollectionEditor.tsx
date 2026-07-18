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
import { useCollectionStore } from "@/stores/collectionStore";
import { Collection } from "@/types";

// ─── Props ────────────────────────────────────────────────────────────────────

interface CollectionEditorProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Pass a collection to edit it; null for create mode. */
  collection: Collection | null;
}

// ─── CollectionEditor ─────────────────────────────────────────────────────────

export function CollectionEditor({
  open,
  onOpenChange,
  collection,
}: CollectionEditorProps) {
  const { t } = useTranslation();
  const createCollection = useCollectionStore((s) => s.createCollection);
  const updateCollection = useCollectionStore((s) => s.updateCollection);

  const isEditMode = collection !== null;

  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Reset form when dialog opens.
  useEffect(() => {
    if (open) {
      setName(collection?.name ?? "");
      setDescription(collection?.description ?? "");
      setValidationError(null);
      setError(null);
    }
  }, [open, collection]);

  async function handleSubmit() {
    const trimmedName = name.trim();
    if (!trimmedName) {
      setValidationError(t("collectionEditor.nameRequired"));
      return;
    }

    setIsSubmitting(true);
    setValidationError(null);
    setError(null);

    try {
      if (isEditMode) {
        await updateCollection(collection.id, trimmedName, description.trim());
      } else {
        await createCollection(trimmedName, description.trim());
      }
      onOpenChange(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {isEditMode ? t("collectionEditor.editTitle") : t("collectionEditor.createTitle")}
          </DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4">
          <DialogDescription>
            {isEditMode
              ? t("collectionEditor.editDesc")
              : t("collectionEditor.createDesc")}
          </DialogDescription>

          {/* Name field */}
          <div className="space-y-1.5">
            <label htmlFor="collection-name" className="text-sm font-medium">
              {t("collectionEditor.nameLabel")} <span className="text-destructive-text">*</span>
            </label>
            <Input
              id="collection-name"
              placeholder={t("collectionEditor.namePlaceholder")}
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                if (validationError) setValidationError(null);
              }}
              disabled={isSubmitting}
              autoFocus
            />
            {validationError && (
              <p className="text-xs text-destructive-text" role="alert">
                {validationError}
              </p>
            )}
          </div>

          {/* Description field */}
          <div className="space-y-1.5">
            <label htmlFor="collection-description" className="text-sm font-medium">
              {t("collectionEditor.descLabel")}
            </label>
            <Input
              id="collection-description"
              placeholder={t("collectionEditor.descPlaceholder")}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              disabled={isSubmitting}
            />
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
            {t("collectionEditor.cancel")}
          </Button>
          <Button onClick={handleSubmit} disabled={isSubmitting}>
            {isSubmitting ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {isEditMode ? t("collectionEditor.saving") : t("collectionEditor.creating")}
              </>
            ) : isEditMode ? (
              t("collectionEditor.save")
            ) : (
              t("collectionEditor.create")
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
