import { useState, useEffect, useMemo, useCallback } from "react";
import { Loader2, Search, CheckSquare, XSquare } from "lucide-react";
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
import { Checkbox } from "@/components/ui/checkbox";
import { useSkillStore } from "@/stores/skillStore";
import { SkillWithLinks } from "@/types";

// ─── Props ────────────────────────────────────────────────────────────────────

interface SkillPickerDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Skill IDs already in the collection (will be pre-checked and marked). */
  existingSkillIds: string[];
  onAdd: (skillIds: string[]) => Promise<void>;
}

// ─── SkillPickerDialog ────────────────────────────────────────────────────────

export function SkillPickerDialog({
  open,
  onOpenChange,
  existingSkillIds,
  onAdd,
}: SkillPickerDialogProps) {
  const { t } = useTranslation();
  const [skills, setSkills] = useState<SkillWithLinks[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isAdding, setIsAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedSkillIds, setSelectedSkillIds] = useState<Set<string>>(new Set());

  const fetchCentralSkillsList = useSkillStore((s) => s.fetchCentralSkillsList);

  const loadSkills = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await fetchCentralSkillsList();
      setSkills(data ?? []);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  }, [fetchCentralSkillsList]);

  // Load central skills when dialog opens.
  useEffect(() => {
    if (open) {
      setSearchQuery("");
      setSelectedSkillIds(new Set());
      setError(null);
      void loadSkills();
    }
  }, [open, loadSkills]);

  // Filter skills by search and exclude skills already in collection.
  const filteredSkills = useMemo(() => {
    let list = skills.filter((s) => !existingSkillIds.includes(s.id));
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter(
        (s) =>
          s.name.toLowerCase().includes(q) ||
          s.description?.toLowerCase().includes(q)
      );
    }
    return list;
  }, [skills, existingSkillIds, searchQuery]);

  function handleToggle(skillId: string, checked: boolean) {
    setSelectedSkillIds((prev) => {
      const next = new Set(prev);
      if (checked) next.add(skillId);
      else next.delete(skillId);
      return next;
    });
  }

  const handleSelectAll = useCallback(() => {
    setSelectedSkillIds(new Set(filteredSkills.map((s) => s.id)));
  }, [filteredSkills]);

  const handleClearSelection = useCallback(() => {
    setSelectedSkillIds(new Set());
  }, []);

  async function handleAdd() {
    const ids = Array.from(selectedSkillIds);
    if (ids.length === 0) return;

    setIsAdding(true);
    setError(null);
    try {
      await onAdd(ids);
      onOpenChange(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsAdding(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="!w-[90vw] !max-w-5xl">
        <DialogHeader>
          <DialogTitle>{t("skillPicker.title")}</DialogTitle>
          <DialogClose />
        </DialogHeader>

        <DialogBody className="space-y-4 !overflow-visible !max-h-none">
          <DialogDescription>
            {t("skillPicker.desc")}
          </DialogDescription>

          {/* Search + Select All / Clear */}
          <div className="flex items-center gap-2">
            <div className="relative flex-1">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground pointer-events-none" />
              <Input
                placeholder={t("skillPicker.searchPlaceholder")}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-8"
                aria-label={t("skillPicker.searchPlaceholder")}
              />
            </div>
            {filteredSkills.length > 0 && (
              <div className="flex items-center gap-1 shrink-0">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleSelectAll}
                  disabled={isLoading}
                >
                  <CheckSquare className="size-3.5" />
                  <span>{t("common.selectAllVisible")}</span>
                </Button>
                {selectedSkillIds.size > 0 && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleClearSelection}
                  >
                    <XSquare className="size-3.5" />
                    <span>{t("common.deselectAll")}</span>
                  </Button>
                )}
              </div>
            )}
          </div>

          {/* Skill list */}
          <div
            className="max-h-[60vh] overflow-y-auto grid grid-cols-2 gap-2 border border-border rounded-md p-2"
            role="group"
            aria-label={t("skillPicker.selectSkills")}
          >
            {isLoading ? (
              <div className="col-span-2 flex items-center justify-center py-6 gap-2 text-muted-foreground text-sm">
                <Loader2 className="size-4 animate-spin" />
                {t("skillPicker.loading")}
              </div>
            ) : filteredSkills.length === 0 ? (
              <p className="col-span-2 text-sm text-muted-foreground text-center py-6">
                {skills.length === 0
                  ? t("skillPicker.noSkills")
                  : existingSkillIds.length === skills.length
                  ? t("skillPicker.allAdded")
                  : t("skillPicker.noMatch", { query: searchQuery })}
              </p>
            ) : (
              filteredSkills.map((skill) => {
                const isChecked = selectedSkillIds.has(skill.id);
                return (
                  <div
                    key={skill.id}
                    className="flex items-start gap-2.5 px-2 py-1.5 rounded hover:bg-hover-bg/20 cursor-pointer"
                    onClick={() => handleToggle(skill.id, !isChecked)}
                  >
                    <Checkbox
                      checked={isChecked}
                      onCheckedChange={(checked) => handleToggle(skill.id, !!checked)}
                      aria-label={skill.name}
                      className="mt-0.5"
                    />
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-medium">{skill.name}</div>
                      {skill.description && (
                        <div className="text-xs text-muted-foreground line-clamp-1 mt-0.5">
                          {skill.description}
                        </div>
                      )}
                    </div>
                  </div>
                );
              })
            )}
          </div>

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
            disabled={isAdding}
          >
            {t("skillPicker.cancel")}
          </Button>
          <Button
            onClick={handleAdd}
            disabled={isAdding || selectedSkillIds.size === 0}
          >
            {isAdding ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                {t("skillPicker.adding")}
              </>
            ) : selectedSkillIds.size > 0 ? (
              t("skillPicker.addCount", { count: selectedSkillIds.size })
            ) : (
              t("skillPicker.add")
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
