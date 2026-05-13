import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Bookmark, Sparkles, Tag } from "lucide-react";

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from "@/components/ui/command";
import type { SavedView, SkillRepositoryWithStats, SkillTag } from "@/types";

export interface CommandPaletteAction {
  id: string;
  label: string;
  shortcut?: string;
  onSelect: () => void;
  icon?: React.ReactNode;
}

export interface CommandPaletteV2Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;

  savedViews: SavedView[];
  tags: SkillTag[];
  repositories: SkillRepositoryWithStats[];
  actions: CommandPaletteAction[];

  onSelectSavedView: (view: SavedView) => void;
  onSelectTag: (tag: SkillTag) => void;
  onSelectRepository: (repository: SkillRepositoryWithStats) => void;
}

/**
 * 全局命令面板（⌘K / Ctrl+K）。
 *
 * 三段：Saved views / Tags / Repositories / Actions。每段限制 8 项避免列表过长。
 * 选中后自动关闭。
 */
export function CommandPaletteV2({
  open,
  onOpenChange,
  savedViews,
  tags,
  repositories,
  actions,
  onSelectSavedView,
  onSelectTag,
  onSelectRepository,
}: CommandPaletteV2Props) {
  const { t } = useTranslation();

  // 全局快捷键 ⌘K / Ctrl+K
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.key === "k" || e.key === "K") && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        onOpenChange(!open);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onOpenChange]);

  const visibleSavedViews = savedViews.slice(0, 8);
  const visibleTags = tags.slice(0, 8);
  const visibleRepositories = repositories.slice(0, 8);

  function handleSelect(callback: () => void) {
    return () => {
      callback();
      onOpenChange(false);
    };
  }

  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("central.v2.commandPaletteTitle")}
      description={t("central.v2.commandPaletteSearchPlaceholder")}
    >
      <CommandInput placeholder={t("central.v2.commandPaletteSearchPlaceholder")} />
      <CommandList>
        <CommandEmpty>{t("central.v2.commandPaletteEmpty")}</CommandEmpty>

        {actions.length > 0 && (
          <CommandGroup heading={t("central.v2.commandPaletteGroupActions")}>
            {actions.map((action) => (
              <CommandItem key={action.id} onSelect={handleSelect(action.onSelect)}>
                {action.icon ?? <Sparkles className="opacity-60" />}
                <span className="truncate">{action.label}</span>
                {action.shortcut ? <CommandShortcut>{action.shortcut}</CommandShortcut> : null}
              </CommandItem>
            ))}
          </CommandGroup>
        )}

        {visibleSavedViews.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading={t("central.v2.commandPaletteGroupViews")}>
              {visibleSavedViews.map((view) => (
                <CommandItem
                  key={view.id}
                  value={`saved-view ${view.name}`}
                  onSelect={handleSelect(() => onSelectSavedView(view))}
                >
                  <Bookmark className="opacity-60" />
                  <span className="truncate">{view.name}</span>
                  <CommandShortcut>{t("central.v2.commandPaletteHintSavedView")}</CommandShortcut>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}

        {visibleTags.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading={t("central.v2.tags")}>
              {visibleTags.map((tag) => (
                <CommandItem
                  key={tag.id}
                  value={`tag ${tag.name}`}
                  onSelect={handleSelect(() => onSelectTag(tag))}
                >
                  <Tag className="opacity-60" />
                  <span className="truncate">{tag.name}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}

        {visibleRepositories.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading={t("central.v2.commandPaletteGroupRepos")}>
              {visibleRepositories.map((repo) => (
                <CommandItem
                  key={repo.id}
                  value={`repo ${repo.name}`}
                  onSelect={handleSelect(() => onSelectRepository(repo))}
                >
                  <span className="truncate">{repo.name}</span>
                  {typeof repo.skill_count === "number" ? (
                    <CommandShortcut>{repo.skill_count}</CommandShortcut>
                  ) : null}
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}
      </CommandList>
    </CommandDialog>
  );
}
