import { useTranslation } from "react-i18next";
import { Pencil, Plus, Tags, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { FacetItem } from "./FacetItem";
import { FacetSection } from "./FacetSection";
import type { TagGroup } from "@/types";

export interface CentralTagGroupsSectionProps {
  tagGroups: TagGroup[];
  onCreate: () => void;
  onRename: (group: TagGroup) => void;
  onDelete: (group: TagGroup) => void;
}

/**
 * Sidebar 上方的 Tag Groups 管理段落（M3）。
 *
 * 列出已有分组（按 sort_order），每行 hover 时显示 rename/delete 按钮。
 * 底部 "+ 创建新分组" 触发外部 prompt（在 bridge hook 中处理）。
 *
 * 此段落不参与 facet 选择，仅做 CRUD 入口。Sidebar Tags 段落的分组渲染由
 * CentralSidebar 通过 tagGroups prop 自行处理。
 */
export function CentralTagGroupsSection({
  tagGroups,
  onCreate,
  onRename,
  onDelete,
}: CentralTagGroupsSectionProps) {
  const { t } = useTranslation();
  const sorted = [...tagGroups].sort((a, b) => a.sort_order - b.sort_order);

  return (
    <FacetSection
      title={t("central.v2.tagGroups")}
      icon={<Tags className="size-3.5" />}
      count={sorted.length}
      testId="central-tag-groups-section"
    >
      <div className="flex flex-col gap-1">
        {sorted.length === 0 ? (
          <div className="px-2 py-1.5 text-xs text-muted-foreground">
            {t("central.v2.tagGroupsEmpty")}
          </div>
        ) : (
          sorted.map((group) => (
            <FacetItem
              key={group.id}
              label={group.name}
              active={false}
              icon={
                <span
                  className="size-2.5 rounded-sm border border-border/60"
                  style={group.color ? { backgroundColor: group.color } : undefined}
                />
              }
              testId={`tag-group-row-${group.id}`}
              onClick={() => onRename(group)}
              trailingAction={
                <span className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="size-6"
                    aria-label={t("central.v2.tagGroupsRename")}
                    title={t("central.v2.tagGroupsRename")}
                    onClick={(e) => {
                      e.stopPropagation();
                      onRename(group);
                    }}
                  >
                    <Pencil className="size-3" />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="size-6 text-destructive-text hover:text-destructive-text"
                    aria-label={t("central.v2.tagGroupsDelete")}
                    title={t("central.v2.tagGroupsDelete")}
                    onClick={(e) => {
                      e.stopPropagation();
                      onDelete(group);
                    }}
                  >
                    <Trash2 className="size-3" />
                  </Button>
                </span>
              }
            />
          ))
        )}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={onCreate}
          className="mt-1 h-7 justify-start gap-2 text-xs"
        >
          <Plus className="size-3.5" />
          {t("central.v2.tagGroupsCreate")}
        </Button>
      </div>
    </FacetSection>
  );
}
