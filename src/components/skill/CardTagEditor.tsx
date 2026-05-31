import { useState, type MouseEvent } from "react";
import { Plus, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { getTagColor } from "@/lib/tagColor";

interface TagLite {
  id: string;
  name: string;
  color?: string | null;
}

export interface CardTagEditorProps {
  tags: TagLite[];
  allTags: TagLite[];
  onAdd: (tagId: string) => void;
  onCreate: (name: string) => void;
  onRemove: (tagId: string) => void;
}

/** 阻断卡片导航/选择的事件冒泡（参考 RepoTrailingActions 模式）。 */
const stop = (e: MouseEvent) => e.stopPropagation();

export function CardTagEditor({
  tags,
  allTags,
  onAdd,
  onCreate,
  onRemove,
}: CardTagEditorProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");

  const assigned = new Set(tags.map((tg) => tg.id));
  const candidates = allTags.filter(
    (tg) =>
      !assigned.has(tg.id) &&
      tg.name.toLowerCase().includes(query.trim().toLowerCase()),
  );
  const canCreate =
    query.trim().length > 0 &&
    !allTags.some((tg) => tg.name.toLowerCase() === query.trim().toLowerCase());

  return (
    <div className="flex flex-wrap items-center gap-1" onClick={stop}>
      {tags.map((tag) => {
        const color = getTagColor(tag);
        return (
          <span
            key={tag.id}
            style={color.style}
            className={cn(
              "group/tag inline-flex items-center gap-1 rounded-full border px-1.5 py-0.5 text-[10px] font-medium",
              color.className,
            )}
          >
            {tag.name}
            <button
              type="button"
              aria-label={t("central.cardTagRemove", { name: tag.name })}
              onClick={(e) => {
                stop(e);
                onRemove(tag.id);
              }}
              className="opacity-0 transition-opacity group-hover/tag:opacity-100"
            >
              <X className="size-2.5" />
            </button>
          </span>
        );
      })}

      <div className="relative">
        <button
          type="button"
          aria-label={t("central.cardTagAdd")}
          title={t("central.cardTagAdd")}
          onClick={(e) => {
            stop(e);
            setOpen((v) => !v);
          }}
          className="grid size-5 place-items-center rounded-full border border-dashed border-border/70 text-muted-foreground hover:border-primary/40 hover:text-primary"
        >
          <Plus className="size-3" />
        </button>
        {open && (
          <div
            className="absolute z-40 mt-1 w-48 rounded-lg border border-border bg-popover p-1.5 shadow-md"
            onMouseDown={(e) => e.preventDefault()}
          >
            <input
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("central.cardTagSearchPlaceholder")}
              className="mb-1 w-full rounded-md border border-border/70 bg-background px-2 py-1 text-xs"
            />
            <div className="max-h-40 overflow-y-auto">
              {candidates.map((tg) => (
                <button
                  key={tg.id}
                  type="button"
                  onClick={(e) => {
                    stop(e);
                    onAdd(tg.id);
                    setQuery("");
                    setOpen(false);
                  }}
                  className="block w-full truncate rounded-md px-2 py-1 text-left text-xs hover:bg-accent"
                >
                  {tg.name}
                </button>
              ))}
              {canCreate && (
                <button
                  type="button"
                  onClick={(e) => {
                    stop(e);
                    onCreate(query.trim());
                    setQuery("");
                    setOpen(false);
                  }}
                  className="block w-full truncate rounded-md px-2 py-1 text-left text-xs text-primary hover:bg-accent"
                >
                  {t("central.cardTagCreate", { name: query.trim() })}
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
