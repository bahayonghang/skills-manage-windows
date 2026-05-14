import {
  PackagePlus,
  Check,
  Link2,
  FolderOpen,
  Folder,
  Globe,
  ArrowUpRight,
  Plus,
  ChevronRight,
  X,
  Loader2,
  Lock,
  Trash2,
  Download,
} from "lucide-react";
import { memo, useMemo, type MouseEventHandler, type Ref } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import type { AgentWithStatus, CentralSkillUpdateState, ClaudeSourceKind } from "@/types";
import { cn } from "@/lib/utils";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberIds,
  getPlatformTargetMemberNames,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";

// ─── Platform Toggle Icon (internal) ──────────────────────────────────────────

const PlatformToggleIcon = memo(function PlatformToggleIcon({
  agent,
  skillName,
  isLinked,
  isToggling,
  isLocked,
  onToggle,
}: {
  agent: AgentWithStatus;
  skillName: string;
  isLinked: boolean;
  isToggling: boolean;
  isLocked: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();
  const memberNames = getPlatformTargetMemberNames(agent).join(", ");
  const displayName = isUniversalPlatformTarget(agent)
    ? t("platformTargets.universalShortLabel")
    : agent.display_name;
  const isDisabled = isToggling || isLocked;
  const title = isLocked
    ? `${displayName} - ${t("platformTargets.alwaysIncluded")} - ${memberNames}`
    : displayName;

  return (
    <button
      className={cn(
        "p-1 rounded-md transition-colors cursor-pointer",
        isLocked
          ? "text-primary cursor-default"
          : isLinked
          ? "text-primary hover:bg-primary/15"
          : "text-muted-foreground/40 hover:bg-muted/60 hover:text-muted-foreground",
        isToggling && "animate-pulse pointer-events-none"
      )}
      title={title}
      aria-label={
        isLocked
          ? title
          : t("central.toggleInstallLabel", { platform: displayName, skill: skillName })
      }
      disabled={isDisabled}
      onClick={onToggle}
    >
      <PlatformIcon agentId={agent.id} className="size-4 shrink-0" size={16} />
    </button>
  );
});
PlatformToggleIcon.displayName = "PlatformToggleIcon";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface UnifiedSkillCardProps {
  /** Core data — always required. */
  name: string;
  description?: string;
  className?: string;

  /** Click the card itself (platform variant navigates to detail). */
  onClick?: () => void;

  // ── discover variant ──
  checkbox?: { checked: boolean; onChange: () => void };
  isCentral?: boolean;
  platformBadge?: { id: string; name: string };
  projectBadge?: string;
  originBadge?: { kind: "central" | "project" | string; label: string };

  // ── central variant ──
  platformIcons?: {
    agents: AgentWithStatus[];
    linkedAgents: string[];
    lockedAgentIds?: string[];
    skillId: string;
    onToggle: (skillId: string, agentId: string) => void;
    togglingAgentId: string | null;
  };

  // ── platform variant ──
  sourceType?: "symlink" | "copy" | "native";
  originKind?: ClaudeSourceKind | null;
  isReadOnly?: boolean;

  // ── marketplace variant ──
  isInstalled?: boolean;
  tags?: { key: string; label: string }[];
  publisher?: string;

  // ── actions (pass only the ones relevant to the context) ──
  onDetail?: MouseEventHandler<HTMLButtonElement>;
  onInstallTo?: () => void;
  onUpdateCentral?: () => void;
  updateStatus?: CentralSkillUpdateState & { isUpdating?: boolean };
  onDeleteFromCentral?: () => void;
  onInstallToCentral?: () => void;
  onInstallToPlatform?: () => void;
  onUninstallFromPlatform?: () => void;
  uninstallFromLabel?: string;
  onInstall?: () => void;
  onRemove?: () => void;
  isLoading?: boolean;
  detailButtonRef?: Ref<HTMLButtonElement>;
}

// ─── UnifiedSkillCard ─────────────────────────────────────────────────────────

function UnifiedSkillCardComponent(props: UnifiedSkillCardProps) {
  const { t } = useTranslation();
  const {
    name,
    description,
    className,
    onClick,
    checkbox,
    isCentral,
    platformBadge,
    projectBadge,
    originBadge,
    platformIcons,
    sourceType,
    originKind,
    isReadOnly,
    isInstalled,
    tags,
    publisher,
    onDetail,
    onInstallTo,
    onUpdateCentral,
    updateStatus,
    onDeleteFromCentral,
    onInstallToCentral,
    onInstallToPlatform,
    onUninstallFromPlatform,
    uninstallFromLabel,
    onInstall,
    onRemove,
    isLoading,
    detailButtonRef,
  } = props;

  // Determine variant features
  const hasCheckbox = !!checkbox;
  const hasPlatformIcons = !!platformIcons;
  const hasActions = !!(
    onDetail ||
    onInstallTo ||
    onUpdateCentral ||
    onDeleteFromCentral ||
    onInstallToCentral ||
    onInstallToPlatform ||
    onUninstallFromPlatform ||
    onInstall ||
    onRemove
  );

  const targetAgents = useMemo(
    () => platformIcons?.agents.filter((a) => a.id !== "central") ?? [],
    [platformIcons?.agents]
  );
  const linkedAgentSet = useMemo(
    () => new Set(platformIcons?.linkedAgents ?? []),
    [platformIcons?.linkedAgents]
  );
  const lockedAgentSet = useMemo(
    () => new Set(platformIcons?.lockedAgentIds ?? []),
    [platformIcons?.lockedAgentIds]
  );

  // ── Platform variant: clickable card style ──
  if (onClick && !hasActions && !hasCheckbox && !hasPlatformIcons) {
    return (
      <button
        role="button"
        onClick={onClick}
        className={cn(
          "w-full h-full text-left focus:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-xl",
          className
        )}
        aria-label={t("platform.searchSkillLabel", { name })}
      >
        <div className="h-full flex flex-col rounded-xl bg-card ring-1 ring-border shadow-sm p-3 gap-3 transition-colors duration-150 hover:ring-primary/25 hover:bg-accent/30 cursor-pointer">
          <div className="flex flex-1 items-start justify-between gap-3">
            <div className="min-w-0 flex-1 space-y-1">
              <div className="font-medium text-sm text-foreground truncate">{name}</div>
              {description && (
                <p className="text-xs text-muted-foreground line-clamp-2 leading-relaxed">{description}</p>
              )}
              {sourceType && <SourceIndicator sourceType={sourceType} />}
            </div>
            <ChevronRight className="size-4 text-muted-foreground shrink-0 mt-0.5" />
          </div>
        </div>
      </button>
    );
  }

  // ── Default card style (central, discover, marketplace) ──
  return (
    <div
      className={cn(
        "rounded-xl bg-card ring-1 ring-border shadow-sm p-3 flex flex-col transition-colors",
        checkbox?.checked && "ring-primary/40 bg-primary/5",
        isLoading && "opacity-50",
        className
      )}
    >
      <div className="flex items-start gap-2.5">
        {/* Optional checkbox (discover) */}
        {hasCheckbox && (
          <div className="pt-0.5">
            <Checkbox
              checked={checkbox.checked}
              onCheckedChange={checkbox.onChange}
              aria-label={t("common.selectSkill")}
            />
          </div>
        )}

        {/* Main content */}
        <div className="flex-1 min-w-0 space-y-1.5">
          {/* Row 1: Name + icon actions */}
          <div className="flex items-center justify-between gap-2">
            {/* Skill name — clickable if onDetail provided */}
            {onDetail ? (
              <button
                ref={detailButtonRef}
                className="font-medium text-sm text-foreground truncate hover:text-primary hover:underline text-left min-w-0 flex-1"
                onClick={onDetail}
                aria-label={t("central.viewDetailsLabel", { name })}
              >
                {name}
              </button>
            ) : (
              <h3 className="text-sm font-medium truncate min-w-0 flex-1">{name}</h3>
            )}

            {/* Icon action buttons */}
            {hasActions && (
              <div className="flex items-center gap-0.5 shrink-0">
                {/* Install To... (central / platform / collection / marketplace) */}
                {onInstallTo && (
                  <button
                    onClick={onInstallTo}
                    disabled={isLoading}
                    title={t("central.installTo")}
                    aria-label={t("central.installLabel", { name })}
                    className="inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors text-muted-foreground hover:bg-primary/10 hover:text-primary disabled:opacity-50 disabled:cursor-default"
                  >
                    <PackagePlus className="size-4" />
                  </button>
                )}

                {onUpdateCentral && (
                  <button
                    onClick={onUpdateCentral}
                    disabled={isLoading || updateStatus?.status !== "update_available" || updateStatus?.isUpdating}
                    title={
                      updateStatus?.status === "update_available"
                        ? t("central.updateSkill")
                        : updateStatus?.error ?? t("central.checkUpdatesFirst")
                    }
                    aria-label={t("central.updateSkillLabel", { name })}
                    className="inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors text-muted-foreground hover:bg-primary/10 hover:text-primary disabled:opacity-50 disabled:cursor-default"
                  >
                    {updateStatus?.isUpdating ? <Loader2 className="size-4 animate-spin" /> : <Download className="size-4" />}
                  </button>
                )}

                {onDeleteFromCentral && (
                  <button
                    onClick={onDeleteFromCentral}
                    disabled={isLoading}
                    title={t("central.deleteSkill")}
                    aria-label={t("central.deleteSkillLabel", { name })}
                    data-testid={`delete-central-skill-${name}`}
                    className="inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors text-muted-foreground hover:bg-destructive/10 hover:text-destructive disabled:opacity-50 disabled:cursor-default"
                  >
                    <Trash2 className="size-4" />
                  </button>
                )}

                {/* Install to Central (discover) */}
                {onInstallToCentral && !isCentral && (
                  <button
                    onClick={onInstallToCentral}
                    disabled={isLoading}
                    title={t("common.installToCentral")}
                    aria-label={t("common.installToCentral")}
                    className="inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors text-muted-foreground hover:bg-primary/10 hover:text-primary disabled:opacity-50 disabled:cursor-default"
                  >
                    {isLoading ? <Loader2 className="size-4 animate-spin" /> : <ArrowUpRight className="size-4" />}
                  </button>
                )}

                {/* Install to Platform (discover) */}
                {onInstallToPlatform && (
                  <button
                    onClick={onInstallToPlatform}
                    disabled={isLoading}
                    title={t("common.installToPlatform")}
                    aria-label={t("common.installToPlatform")}
                    className="inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors text-muted-foreground hover:bg-primary/10 hover:text-primary disabled:opacity-50 disabled:cursor-default"
                  >
                    {isLoading ? <Loader2 className="size-4 animate-spin" /> : <Plus className="size-4" />}
                  </button>
                )}

                {onUninstallFromPlatform && (
                  <InlineConfirmAction
                    onConfirm={onUninstallFromPlatform}
                    isLoading={isLoading}
                    idleTitle={uninstallFromLabel ?? t("common.uninstall")}
                    idleAriaLabel={uninstallFromLabel ?? t("common.uninstall")}
                    confirmLabel={t("common.confirmDelete")}
                    icon={<X className="size-4" />}
                  />
                )}

                {/* Marketplace installed indicator (disabled Check icon) */}
                {onInstall && isInstalled && (
                  <button
                    disabled
                    title={t("marketplace.installed")}
                    aria-label={t("marketplace.installed")}
                    className="inline-flex h-8 w-8 items-center justify-center rounded-md text-primary cursor-default"
                  >
                    <Check className="size-4" />
                  </button>
                )}

                {/* Remove (collection) */}
                {onRemove && (
                  <InlineConfirmAction
                    onConfirm={onRemove}
                    isLoading={isLoading}
                    idleTitle={t("collection.removeSkillLabel", { name })}
                    idleAriaLabel={t("collection.removeSkillLabel", { name })}
                    confirmLabel={t("common.confirmDelete")}
                    icon={<X className="size-4" />}
                  />
                )}
              </div>
            )}
          </div>

          {/* Row 2: Description — full width, not compressed by actions */}
          {description && (
            <p className="text-xs text-muted-foreground line-clamp-2 leading-relaxed">{description}</p>
          )}

          {/* Row 3: Info badges */}
          <div className="flex flex-wrap items-center gap-1.5 empty:hidden">
            {originKind && <SourceOriginBadge originKind={originKind} />}
            {isReadOnly && <ReadOnlyBadge />}

            {/* Source indicator (platform) */}
            {sourceType && <SourceIndicator sourceType={sourceType} />}

            {originBadge && <ProjectSourceBadge originBadge={originBadge} />}

            {/* "Already in Central" badge */}
            {isCentral && (
              <span className="inline-flex items-center gap-1 text-xs text-muted-foreground bg-muted/50 px-1.5 py-0.5 rounded">
                <Globe className="size-3" />
                {t("common.alreadyCentral")}
              </span>
            )}

            {/* Platform badge (discover) */}
            {platformBadge && (
              <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                <PlatformIcon agentId={platformBadge.id} className="size-3" />
                {platformBadge.name}
              </span>
            )}

            {/* Project badge (discover) */}
            {projectBadge && (
              <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                <Folder className="size-3" />
                {projectBadge}
              </span>
            )}

            {/* Publisher (marketplace recommended) */}
            {publisher && (
              <span className="text-[10px] text-muted-foreground truncate">{publisher}</span>
            )}

            {updateStatus && updateStatus.status !== "up_to_date" && (
              <span
                className={cn(
                  "inline-flex items-center rounded-full px-1.5 py-0.5 text-[10px] font-medium ring-1",
                  updateStatus.status === "update_available"
                    ? "bg-primary/10 text-primary ring-primary/20"
                    : updateStatus.status === "error"
                      ? "bg-destructive/10 text-destructive ring-destructive/20"
                      : updateStatus.status === "remote_missing"
                        ? "bg-amber-500/10 text-amber-700 ring-amber-500/30 dark:text-amber-300"
                      : "bg-muted text-muted-foreground ring-border"
                )}
                title={updateStatus.error ?? undefined}
              >
                {t(`central.updateStatus.${updateStatus.status}`)}
              </span>
            )}

            {/* Tags (marketplace recommended) */}
            {tags && tags.length > 0 && (
              <div className="flex items-center gap-1">
                {tags.slice(0, 2).map((tag) => (
                  <span key={tag.key} className="text-[10px] bg-muted/60 text-muted-foreground px-1.5 py-0.5 rounded">
                    {tag.label}
                  </span>
                ))}
              </div>
            )}
          </div>

          {/* Row 3: Platform toggle icons (central) */}
          {hasPlatformIcons && targetAgents.length > 0 && (
            <div className="space-y-1 mt-auto pt-1">
              <div className="flex items-center gap-1.5">
                <span className="text-[10px] font-medium text-muted-foreground/70 uppercase tracking-wider w-14 shrink-0">
                  {t("central.platformTargetsLabel")}
                </span>
                <div className="flex items-center gap-0.5 flex-wrap">
                  {targetAgents.map((agent) => (
                    <PlatformToggleIcon
                      key={agent.id}
                      agent={agent}
                      skillName={name}
                      isLinked={getPlatformTargetMemberIds(agent).some((agentId) => linkedAgentSet.has(agentId))}
                      isToggling={getPlatformTargetMemberIds(agent).some((agentId) => platformIcons.togglingAgentId === agentId)}
                      isLocked={getPlatformTargetMemberIds(agent).some((agentId) => lockedAgentSet.has(agentId))}
                      onToggle={() => {
                        const [agentId] = getPlatformTargetInstallAgentIds(agent);
                        if (agentId) {
                          platformIcons.onToggle(platformIcons.skillId, agentId);
                        }
                      }}
                    />
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export const UnifiedSkillCard = memo(UnifiedSkillCardComponent);
UnifiedSkillCard.displayName = "UnifiedSkillCard";

// ─── Source Indicator (internal) ──────────────────────────────────────────────

function SourceIndicator({ sourceType }: { sourceType: string }) {
  const { t, i18n } = useTranslation();
  const isSymlink = sourceType === "symlink";
  const isNative = sourceType === "native";
  const primaryLabel = isSymlink ? t("platform.sourceCentral") : t("platform.sourceStandalone");
  const secondaryLabel = isSymlink
    ? t("platform.sourceSymlinkLabel")
    : isNative
      ? t("platform.sourceNativeLabel", {
          defaultValue: i18n.language.startsWith("zh") ? "原生" : "native",
        })
      : t("platform.sourceCopyLabel");

  return (
    <div
      className={cn(
        "inline-flex items-center gap-1 text-xs font-medium",
        isSymlink ? "text-primary/80" : "text-muted-foreground"
      )}
    >
      {isSymlink ? <Link2 className="size-3 shrink-0" /> : <FolderOpen className="size-3 shrink-0" />}
      <div className="inline-flex items-center gap-1">
        <span>{primaryLabel}</span>
        <span aria-hidden="true" className="h-px w-3 shrink-0 rounded-full bg-current opacity-40" />
        <span className="sr-only"> - </span>
        <span>{secondaryLabel}</span>
      </div>
    </div>
  );
}

function SourceOriginBadge({ originKind }: { originKind: ClaudeSourceKind }) {
  const { t, i18n } = useTranslation();
  const isPlugin = originKind === "plugin";

  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium ring-1",
        isPlugin
          ? "bg-amber-500/10 text-amber-700 ring-amber-500/20 dark:text-amber-300"
          : "bg-sky-500/10 text-sky-700 ring-sky-500/20 dark:text-sky-300"
      )}
    >
      {isPlugin
        ? t("platform.originPlugin", {
            defaultValue: i18n.language.startsWith("zh") ? "插件来源" : "Plugin source",
          })
        : t("platform.originUser", {
            defaultValue: i18n.language.startsWith("zh") ? "用户来源" : "User source",
          })}
    </span>
  );
}

function ReadOnlyBadge() {
  const { t, i18n } = useTranslation();

  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground ring-1 ring-border/70">
      <Lock className="size-3 shrink-0" />
      {t("platform.readOnly", {
        defaultValue: i18n.language.startsWith("zh") ? "只读" : "Read-only",
      })}
    </span>
  );
}

function ProjectSourceBadge({
  originBadge,
}: {
  originBadge: { kind: "central" | "project" | string; label: string };
}) {
  const isCentral = originBadge.kind === "central";

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium ring-1",
        isCentral
          ? "bg-sky-500/10 text-sky-700 ring-sky-500/20 dark:text-sky-300"
          : "bg-muted text-muted-foreground ring-border/70"
      )}
    >
      {isCentral ? <Globe className="size-3 shrink-0" /> : <Folder className="size-3 shrink-0" />}
      {originBadge.label}
    </span>
  );
}
