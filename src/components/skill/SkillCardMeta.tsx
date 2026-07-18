import { Folder, Globe } from "lucide-react";
import { useTranslation } from "react-i18next";

import { PlatformIcon } from "@/components/platform/PlatformIcon";
import {
  ProjectSourceBadge,
  ReadOnlyBadge,
  SourceChip,
  SourceIndicator,
  SourceOriginBadge,
} from "@/components/skill/SkillCardBadges";
import { cn } from "@/lib/utils";
import type { CentralSkillUpdateState, ClaudeSourceKind } from "@/types";

export interface SkillCardMetaProps {
  originKind?: ClaudeSourceKind;
  isReadOnly?: boolean;
  sourceType?: "symlink" | "copy" | "native";
  originBadge?: { kind: "central" | "project" | string; label: string };
  usageBadge?: number;
  isCentral?: boolean;
  platformBadge?: { id: string; name: string };
  projectBadge?: string;
  publisher?: string;
  updateStatus?: CentralSkillUpdateState & { isUpdating?: boolean };
  inventoryFlags?: { hasDuplicate: boolean; isOrphan: boolean } | undefined;
  inventorySkillId?: string;
  tags?: { key: string; label: string }[];
}

/**
 * Skill 卡片的 meta 行：徽章、publisher、tags 等次级信息，
 * 统一用 flex-wrap 排成一行，根据传入字段自动收敛展示。
 */
export function SkillCardMeta({
  originKind,
  isReadOnly,
  sourceType,
  originBadge,
  usageBadge,
  isCentral,
  platformBadge,
  projectBadge,
  publisher,
  updateStatus,
  inventoryFlags,
  inventorySkillId,
  tags,
}: SkillCardMetaProps) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-wrap items-center gap-1.5 empty:hidden">
      {originKind && <SourceOriginBadge originKind={originKind} />}
      {isReadOnly && <ReadOnlyBadge />}
      {sourceType && <SourceIndicator sourceType={sourceType} />}
      {originBadge && <ProjectSourceBadge originBadge={originBadge} />}

      {typeof usageBadge === "number" && usageBadge > 0 && (
        <span
          data-testid="usage-badge"
          title={t("skillUsage.badge.tooltip", {
            count: usageBadge,
            days: 30,
            defaultValue: `${usageBadge} calls in last 30 days`,
          })}
          className="inline-flex items-center gap-1 rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary"
        >
          <span className="tabular-nums">
            {t("skillUsage.badge.countShort", { count: usageBadge })}
          </span>
          <span aria-hidden="true" className="text-primary/45">
            ·
          </span>
          <span className="tabular-nums text-primary-text">
            {t("skillUsage.badge.periodShort", { days: 30 })}
          </span>
        </span>
      )}

      {isCentral && (
        <span className="inline-flex items-center gap-1 text-xs text-muted-foreground bg-muted/50 px-1.5 py-0.5 rounded">
          <Globe className="size-3" />
          {t("common.alreadyCentral")}
        </span>
      )}

      {platformBadge && (
        <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
          <PlatformIcon agentId={platformBadge.id} className="size-3" />
          {platformBadge.name}
        </span>
      )}

      {projectBadge && (
        <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
          <Folder className="size-3" />
          {projectBadge}
        </span>
      )}

      {publisher && <SourceChip label={publisher} />}

      {updateStatus && updateStatus.status !== "up_to_date" && (
        <span
          className={cn(
            "inline-flex items-center rounded-full px-1.5 py-0.5 text-xs font-medium ring-1",
            updateStatus.status === "update_available"
              ? "bg-primary/10 text-primary ring-primary/20"
              : updateStatus.status === "error"
                ? "bg-destructive/10 text-destructive-text ring-destructive/20"
                : updateStatus.status === "remote_missing"
                  ? "bg-warning/10 text-warning-foreground ring-warning/30"
                  : "bg-muted text-muted-foreground ring-border",
          )}
          title={updateStatus.error ?? undefined}
        >
          {t(`central.updateStatus.${updateStatus.status}`)}
        </span>
      )}

      {inventoryFlags?.hasDuplicate && (
        <span
          data-testid={
            inventorySkillId
              ? `skill-card-duplicate-badge-${inventorySkillId}`
              : undefined
          }
          className="inline-flex items-center rounded-full bg-warning/10 px-1.5 py-0.5 text-xs font-medium text-warning-foreground ring-1 ring-warning/30"
        >
          {t("central.updateCenter.badges.duplicate")}
        </span>
      )}
      {inventoryFlags?.isOrphan && (
        <span
          data-testid={
            inventorySkillId
              ? `skill-card-orphan-badge-${inventorySkillId}`
              : undefined
          }
          className="inline-flex items-center rounded-full bg-muted px-1.5 py-0.5 text-xs font-medium text-muted-foreground ring-1 ring-border"
        >
          {t("central.updateCenter.badges.orphan")}
        </span>
      )}

      {tags && tags.length > 0 && (
        <div className="flex items-center gap-1">
          {tags.slice(0, 2).map((tag) => (
            <span
              key={tag.key}
              className="text-ui-meta bg-muted/60 text-muted-foreground px-1.5 py-0.5 rounded"
            >
              {tag.label}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
