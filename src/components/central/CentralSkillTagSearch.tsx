import { useEffect, useMemo, useState } from "react";
import { ChevronDown, Tag as TagIcon } from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { cn } from "@/lib/utils";
import type { SkillTag } from "@/types";

type Tone = "all" | "uncategorized" | "updates" | "ai-review" | "tag";

type TagCount = {
  tag: SkillTag;
  count: number;
};

type ViewItem = {
  id: "all" | "uncategorized" | "updates" | "ai-review";
  label: string;
  tone: Exclude<Tone, "tag">;
  count: number;
};

const DOT_CLASS_BY_TONE: Record<Tone, string> = {
  all: "bg-muted-foreground/60",
  uncategorized: "bg-slate-400",
  updates: "bg-amber-500",
  "ai-review": "bg-violet-500",
  tag: "bg-emerald-500",
};

function isMac(): boolean {
  if (typeof navigator === "undefined") return false;
  const platform = navigator.platform || "";
  const userAgent = navigator.userAgent || "";
  return /mac/i.test(platform) || /Mac/i.test(userAgent);
}

function ShortcutHint({ children }: { children: string }) {
  return (
    <kbd className="pointer-events-none ml-1 hidden h-5 select-none items-center gap-1 rounded border border-border/70 bg-muted/60 px-1.5 font-mono text-[10px] font-medium text-muted-foreground sm:inline-flex">
      {children}
    </kbd>
  );
}

function Dot({ tone }: { tone: Tone }) {
  return (
    <span
      className={cn("inline-block size-2 shrink-0 rounded-full", DOT_CLASS_BY_TONE[tone])}
      aria-hidden="true"
    />
  );
}

function CountBadge({ value }: { value: number }) {
  return (
    <span className="ml-auto shrink-0 rounded-md bg-muted px-1.5 py-0.5 font-mono text-[11px] tabular-nums text-muted-foreground">
      {value}
    </span>
  );
}

export function CentralSkillTagSearch({
  tagFilter,
  setTagFilter,
  setCategorizeTab,
  tags,
  tagCounts,
  uncategorizedCount,
  updateAvailableSkillCount,
  aiReviewCount,
  totalSkillCount,
  t,
}: {
  tagFilter: string;
  setTagFilter: (value: string) => void;
  setCategorizeTab: (tab: "manual" | "ai" | "review") => void;
  tags: SkillTag[];
  tagCounts: TagCount[];
  uncategorizedCount: number;
  updateAvailableSkillCount: number;
  aiReviewCount: number;
  totalSkillCount: number;
  t: TFunction;
}) {
  const [open, setOpen] = useState(false);
  const mac = useMemo(() => isMac(), []);

  // Global shortcut: Cmd+K (mac) / Ctrl+K (others) toggles the panel.
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const isShortcut =
        (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k";
      if (!isShortcut) return;
      const target = event.target as HTMLElement | null;
      // Avoid intercepting edits in contenteditable surfaces.
      if (target?.isContentEditable) return;
      event.preventDefault();
      setOpen((prev) => !prev);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const viewItems: ViewItem[] = useMemo(
    () => [
      {
        id: "all",
        label: t("central.allTags"),
        tone: "all",
        count: totalSkillCount,
      },
      {
        id: "uncategorized",
        label: t("central.uncategorizedOnly"),
        tone: "uncategorized",
        count: uncategorizedCount,
      },
      {
        id: "updates",
        label: t("central.updatesAvailableOnly"),
        tone: "updates",
        count: updateAvailableSkillCount,
      },
      {
        id: "ai-review",
        label: t("central.aiReviewOnly"),
        tone: "ai-review",
        count: aiReviewCount,
      },
    ],
    [
      aiReviewCount,
      t,
      totalSkillCount,
      uncategorizedCount,
      updateAvailableSkillCount,
    ]
  );

  const sortedTagCounts = useMemo(() => {
    return [...tagCounts].sort((a, b) =>
      a.tag.name.localeCompare(b.tag.name, undefined, {
        numeric: true,
        sensitivity: "base",
      })
    );
  }, [tagCounts]);

  const triggerLabel = useMemo(() => {
    const view = viewItems.find((item) => item.id === tagFilter);
    if (view) return view.label;
    const tag = tags.find((entry) => entry.id === tagFilter);
    return tag?.name ?? t("central.allTags");
  }, [tagFilter, tags, t, viewItems]);

  function handleSelect(value: string) {
    setTagFilter(value);
    if (value === "ai-review") {
      setCategorizeTab("review");
    }
    setOpen(false);
  }

  const isUserTagSelected = !viewItems.some((item) => item.id === tagFilter);
  const triggerTone: Tone = isUserTagSelected
    ? "tag"
    : (viewItems.find((item) => item.id === tagFilter)?.tone ?? "all");

  return (
    <>
      <Button
        type="button"
        variant="outline"
        size="sm"
        data-testid="tag-search-trigger"
        title={t("central.tagFilterTriggerHint", {
          shortcut: mac ? "⌘K" : "Ctrl+K",
        })}
        onClick={() => setOpen(true)}
        className="h-9 max-w-[400px] gap-2 px-3 text-xs"
      >
        <TagIcon className="size-3.5 shrink-0 opacity-70" />
        <span className="text-muted-foreground/80">
          {t("central.tagFilterTriggerLabel")}
        </span>
        <span className="truncate font-medium text-foreground" title={triggerLabel}>
          {triggerLabel}
        </span>
        <Dot tone={triggerTone} />
        <ChevronDown className="size-3.5 shrink-0 text-muted-foreground" />
        <ShortcutHint>{mac ? "⌘K" : "Ctrl+K"}</ShortcutHint>
      </Button>

      <CommandDialog
        open={open}
        onOpenChange={setOpen}
        title={t("central.tagSearchTitle")}
        description={t("central.tagSearchDescription")}
      >
        <Command>
          <CommandInput placeholder={t("central.tagSearchPlaceholder")} />
          <CommandList>
            <CommandEmpty>{t("central.tagSearchEmpty")}</CommandEmpty>

            <CommandGroup heading={t("central.tagSearchViewsGroup")}>
              {viewItems.map((item) => (
                <CommandItem
                  key={item.id}
                  value={item.label}
                  data-checked={tagFilter === item.id}
                  data-testid={`tag-search-item-${item.id}`}
                  onSelect={() => handleSelect(item.id)}
                >
                  <Dot tone={item.tone} />
                  <span className="flex-1 truncate">{item.label}</span>
                  <CountBadge value={item.count} />
                </CommandItem>
              ))}
            </CommandGroup>

            {sortedTagCounts.length > 0 && (
              <CommandGroup heading={t("central.tagSearchTagsGroup")}>
                {sortedTagCounts.map(({ tag, count }) => (
                  <CommandItem
                    key={tag.id}
                    value={tag.name}
                    data-checked={tagFilter === tag.id}
                    data-testid={`tag-search-item-${tag.id}`}
                    onSelect={() => handleSelect(tag.id)}
                  >
                    <Dot tone="tag" />
                    <span className="flex-1 truncate">{tag.name}</span>
                    <CountBadge value={count} />
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
          </CommandList>
        </Command>
      </CommandDialog>
    </>
  );
}
