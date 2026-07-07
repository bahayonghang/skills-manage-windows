import type { RefObject } from "react";
import { useTranslation } from "react-i18next";
import {
  Copy,
  Download,
  ExternalLink,
  FolderOpen,
  Loader2,
  Monitor,
  Plus,
  RefreshCw,
  Tag,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { repositoryDisplayName } from "@/lib/centralConflictSource";
import { cn } from "@/lib/utils";
import { formatPathForDisplay } from "@/lib/path";
import {
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberIds,
} from "@/lib/platformTargetGroups";
import type {
  AgentWithStatus,
  CentralSkillUpdateState,
  Collection,
  ProjectUsingSkill,
  SkillDetail,
  SkillInstallation,
  SkillRepositoryWithStats,
  SkillTag,
} from "@/types";
import {
  inspectorActionButtonClassName,
  inspectorCardClassName,
  inspectorFieldLabelClassName,
  inspectorSelectClassName,
  MetadataRow,
  PlatformToggleIcon,
  ReadOnlySourceBadge,
  SectionLabel,
  SourceOriginBadge,
} from "./SkillDetailViewShared";
import {
  InstalledPlatformList,
} from "./SkillDetailInstalledPlatforms";
import {
  buildInstalledPlatformRows,
  type InstalledPlatformUninstallRequest,
} from "./skillDetailInstalledPlatformsModel";
import { SkillDetailFileTree } from "./SkillDetailFileTree";
import type { SourceMetadata } from "./skillDetailViewTypes";
import type { DirectoryTreeEntry } from "@/types";

interface SkillDetailSidebarProps {
  isFileMode: boolean;
  sourceMetadata?: SourceMetadata;
  detail: SkillDetail | null;
  isRemoteTarget: boolean;
  onOpenSourcePath: () => void;
  onOpenDetailDirectory: () => void;
  directoryTree: DirectoryTreeEntry[];
  isDirectoryLoading: boolean;
  onOpenFileTreePath: (path: string) => void;
  updateStatus?: CentralSkillUpdateState;
  isCheckingUpdates: boolean;
  isUpdatingThisSkill: boolean;
  onCheckSkillUpdate: () => void;
  onUpdateSkill: () => void;
  repositories: SkillRepositoryWithStats[];
  tags: SkillTag[];
  isMetadataUpdating: boolean;
  selectedRepositoryId: string;
  onSelectedRepositoryChange: (value: string) => void;
  onAssignRepository: () => void;
  selectedTagId: string;
  onSelectedTagChange: (value: string) => void;
  onAssignTag: () => void;
  targetAgents: AgentWithStatus[];
  lobsterAgents: AgentWithStatus[];
  codingAgents: AgentWithStatus[];
  installationMap: Map<string, SkillInstallation>;
  sharedRootAgentIds: Set<string>;
  installingAgentId: string | null;
  onToggleInstall: (agentId: string) => void;
  onOpenGitHubRepository: (url: string) => void;
  onRequestUninstallInstalledPlatform: (request: InstalledPlatformUninstallRequest) => void;
  skillCollections: Collection[];
  addToCollectionButtonRef: RefObject<HTMLButtonElement | null>;
  onOpenCollectionPicker: () => void;
  /** 3.8: 反向视图——本 skill 装在哪些项目下。仅在中央非只读 skill 上有值。 */
  projectsUsingSkill?: ProjectUsingSkill[];
  isLoadingProjectsUsingSkill?: boolean;
}

type RepositoryLinkLike = Pick<
  SkillRepositoryWithStats,
  "is_unknown" | "owner" | "repo" | "url"
>;

function buildGitHubRepositoryUrl(repository?: RepositoryLinkLike | null): string | null {
  if (!repository || repository.is_unknown) return null;
  const explicitUrl = repository.url?.trim();
  if (explicitUrl) return explicitUrl;
  const owner = repository.owner?.trim();
  const repo = repository.repo?.trim();
  if (!owner || !repo) return null;
  return `https://github.com/${owner}/${repo}`;
}

function PlatformInstallStatusGroups({
  lobsterAgents,
  codingAgents,
  installationMap,
  sharedRootAgentIds,
  installingAgentId,
  onToggleInstall,
  skillName,
}: {
  lobsterAgents: AgentWithStatus[];
  codingAgents: AgentWithStatus[];
  installationMap: Map<string, SkillInstallation>;
  sharedRootAgentIds: Set<string>;
  installingAgentId: string | null;
  onToggleInstall: (agentId: string) => void;
  skillName: string;
}) {
  const { t } = useTranslation();

  const renderAgentRow = (agents: AgentWithStatus[], categoryLabel: string) => {
    if (agents.length === 0) {
      return null;
    }

    return (
      <div className="flex items-center gap-1">
        <span className="w-12 shrink-0 text-[11px] font-medium uppercase tracking-wider text-muted-foreground/70">
          {categoryLabel}
        </span>
        <div className="flex flex-wrap items-center gap-0.5">
          {agents.map((agent) => {
            const memberIds = getPlatformTargetMemberIds(agent);
            const isInstalled =
              memberIds.some((agentId) => installationMap.has(agentId))
              || memberIds.some((agentId) => sharedRootAgentIds.has(agentId));
            const isLocked = memberIds.some((agentId) => sharedRootAgentIds.has(agentId));
            const isLoading = memberIds.some((agentId) => installingAgentId === agentId);

            return (
              <PlatformToggleIcon
                key={agent.id}
                agent={agent}
                skillName={skillName}
                isInstalled={isInstalled}
                isLoading={isLoading}
                isLocked={isLocked}
                onToggle={() => {
                  const [agentId] = getPlatformTargetInstallAgentIds(agent);
                  if (agentId) {
                    onToggleInstall(agentId);
                  }
                }}
              />
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <>
      {renderAgentRow(lobsterAgents, t("sidebar.categoryLobster"))}
      {renderAgentRow(codingAgents, t("sidebar.categoryCoding"))}
    </>
  );
}

function SourceMetadataSection({
  sourceMetadata,
  isRemoteTarget,
  onOpenSourcePath,
}: {
  sourceMetadata: SourceMetadata;
  isRemoteTarget: boolean;
  onOpenSourcePath: () => void;
}) {
  const { t } = useTranslation();

  return (
    <section aria-label={t("detail.metadataRegion")}>
      <SectionLabel>{t("detail.metadata")}</SectionLabel>
      <div className="space-y-2.5">
        <div className="space-y-0.5">
          <div className="text-[11px] uppercase tracking-wider text-muted-foreground/70">
            {t("common.platform")}
          </div>
          <div className="inline-flex items-center gap-1 break-all font-mono text-xs leading-relaxed text-foreground">
            <Monitor className="size-3.5" />
            <span>{sourceMetadata.platformName}</span>
          </div>
        </div>
        <div className="space-y-0.5">
          <div className="text-[11px] uppercase tracking-wider text-muted-foreground/70">
            {t("common.project")}
          </div>
          <div className="inline-flex items-center gap-1 break-all font-mono text-xs leading-relaxed text-foreground">
            <FolderOpen className="size-3.5" />
            <span>{sourceMetadata.projectName}</span>
          </div>
        </div>
        <div className="space-y-0.5">
          <div className="text-[11px] uppercase tracking-wider text-muted-foreground/70">
            {t("common.filePath")}
          </div>
          <button
            type="button"
            onClick={onOpenSourcePath}
            className="cursor-pointer text-left font-mono text-xs leading-relaxed text-foreground break-all hover:text-primary hover:underline"
          >
            {isRemoteTarget && <Copy className="mr-1 inline size-3 shrink-0" />}
            {sourceMetadata.filePath}
          </button>
        </div>
      </div>
    </section>
  );
}

export function SkillDetailSidebar({
  isFileMode,
  sourceMetadata,
  detail,
  isRemoteTarget,
  onOpenSourcePath,
  onOpenDetailDirectory,
  directoryTree,
  isDirectoryLoading,
  onOpenFileTreePath,
  updateStatus,
  isCheckingUpdates,
  isUpdatingThisSkill,
  onCheckSkillUpdate,
  onUpdateSkill,
  repositories,
  tags,
  isMetadataUpdating,
  selectedRepositoryId,
  onSelectedRepositoryChange,
  onAssignRepository,
  selectedTagId,
  onSelectedTagChange,
  onAssignTag,
  targetAgents,
  lobsterAgents,
  codingAgents,
  installationMap,
  sharedRootAgentIds,
  installingAgentId,
  onToggleInstall,
  onOpenGitHubRepository,
  onRequestUninstallInstalledPlatform,
  skillCollections,
  addToCollectionButtonRef,
  onOpenCollectionPicker,
  projectsUsingSkill = [],
  isLoadingProjectsUsingSkill = false,
}: SkillDetailSidebarProps) {
  const { t, i18n } = useTranslation();
  const repositoryLabel = repositoryDisplayName(detail?.repository);
  const githubRepositoryUrl = buildGitHubRepositoryUrl(detail?.repository);
  const installedPlatformRows = buildInstalledPlatformRows(
    targetAgents,
    installationMap,
    sharedRootAgentIds
  );

  return (
    <aside
      data-testid="skill-detail-right-sidebar"
      className="scrollbar-subtle w-full min-w-0 shrink-0 space-y-5 overflow-x-hidden overflow-y-auto border-t border-border p-4 lg:w-72 lg:border-t-0 lg:border-l xl:w-80"
    >
      {isFileMode && sourceMetadata ? (
        <SourceMetadataSection
          sourceMetadata={sourceMetadata}
          isRemoteTarget={isRemoteTarget}
          onOpenSourcePath={onOpenSourcePath}
        />
      ) : detail ? (
        <>
          {(detail.source_kind || detail.is_read_only) && (
            <section
              aria-label={t("detail.sourceStatusRegion", {
                defaultValue: i18n.language.startsWith("zh") ? "来源状态" : "Source status",
              })}
            >
              <SectionLabel>
                {t("detail.sourceStatus", {
                  defaultValue: i18n.language.startsWith("zh") ? "来源状态" : "Source status",
                })}
              </SectionLabel>
              <div className="space-y-2 rounded-lg border border-border/70 bg-muted/30 p-3">
                <div className="flex flex-wrap items-center gap-1.5">
                  {detail.source_kind && <SourceOriginBadge originKind={detail.source_kind} />}
                  {detail.is_read_only && <ReadOnlySourceBadge />}
                </div>
                {detail.is_read_only ? (
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    {t("detail.readOnlyDesc", {
                      defaultValue: i18n.language.startsWith("zh")
                        ? "插件安装的副本仅供查看，不能在这里安装、卸载或调整技能集。"
                        : "Plugin-installed copies are display-only here, so install, uninstall, and collection changes are unavailable.",
                    })}
                  </p>
                ) : detail.source_kind === "user" ? (
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    {t("detail.userManagedDesc", {
                      defaultValue: i18n.language.startsWith("zh")
                        ? "此 Claude 用户副本会保留正常的安装状态与技能集管理能力。"
                        : "This Claude user copy keeps the normal install-state and collection-management controls.",
                    })}
                  </p>
                ) : null}
              </div>
            </section>
          )}

          <section aria-label={t("detail.metadataRegion")}>
            <SectionLabel>{t("detail.metadata")}</SectionLabel>
            <div className="space-y-3">
              {detail.dir_path && (
                <div className={inspectorCardClassName}>
                  <MetadataRow
                    label={t("detail.localDirectory")}
                    value={detail.dir_path}
                  />
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={onOpenDetailDirectory}
                    className={cn(
                      inspectorActionButtonClassName,
                      "h-9 justify-start gap-2 px-3 text-xs font-medium",
                    )}
                  >
                    {isRemoteTarget ? (
                      <Copy className="size-3.5 shrink-0" />
                    ) : (
                      <FolderOpen className="size-3.5 shrink-0" />
                    )}
                    <span className="min-w-0 truncate">
                      {isRemoteTarget
                        ? t("detail.copyRemoteFolderPath")
                        : t("detail.openFolder")}
                    </span>
                  </Button>
                </div>
              )}

              {githubRepositoryUrl && repositoryLabel && (
                <div className={inspectorCardClassName}>
                  <MetadataRow label={t("detail.githubRepository")} value={repositoryLabel} />
                  <button
                    type="button"
                    onClick={() => onOpenGitHubRepository(githubRepositoryUrl)}
                    className={cn(
                      inspectorActionButtonClassName,
                      "inline-flex h-9 items-center justify-start gap-2 rounded-md border border-input bg-background px-3 text-xs font-medium shadow-xs transition-[color,box-shadow] hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                    )}
                  >
                    <ExternalLink className="size-3.5 shrink-0" />
                    <span className="min-w-0 truncate">{t("detail.openGithubRepo")}</span>
                  </button>
                  {detail.source_path && (
                    <MetadataRow label={t("detail.repositoryPath")} value={detail.source_path} />
                  )}
                </div>
              )}

              <details
                data-testid="detail-technical-details"
                className={cn(inspectorCardClassName, "group")}
              >
                <summary className="cursor-pointer text-xs font-medium text-muted-foreground transition-colors hover:text-foreground">
                  {t("detail.technicalDetails")}
                </summary>
                <div className="mt-3 space-y-2.5">
                  <MetadataRow label={t("detail.filePath")} value={detail.file_path} />
                  {detail.canonical_path && (
                    <MetadataRow label={t("detail.canonical")} value={detail.canonical_path} />
                  )}
                  {detail.source_root && (
                    <MetadataRow label={t("detail.sourceRoot")} value={detail.source_root} />
                  )}
                  {detail.source && (
                    <MetadataRow label={t("detail.source")} value={detail.source} />
                  )}
                  <MetadataRow
                    label={t("detail.scannedAt")}
                    value={new Date(detail.scanned_at).toLocaleString()}
                  />
                </div>
              </details>
            </div>
          </section>

          <SkillDetailFileTree
            entries={directoryTree}
            isLoading={isDirectoryLoading}
            onOpenPath={onOpenFileTreePath}
          />

          {detail.is_central && !detail.is_read_only && (
            <section aria-label={t("detail.updateStatusRegion")}>
              <SectionLabel>{t("detail.updateStatus")}</SectionLabel>
              <div data-testid="detail-update-status-card" className={inspectorCardClassName}>
                <div className="min-w-0 space-y-2">
                  <span className="inline-flex rounded-full bg-background px-2 py-0.5 text-[11px] font-medium text-muted-foreground ring-1 ring-border/70">
                    {updateStatus
                      ? t(`central.updateStatus.${updateStatus.status}`)
                      : t("central.updateStatus.not_checked")}
                  </span>
                </div>
                <div data-testid="detail-update-actions" className="grid grid-cols-2 gap-2">
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    disabled={isCheckingUpdates || isUpdatingThisSkill}
                    onClick={onCheckSkillUpdate}
                    className={inspectorActionButtonClassName}
                  >
                    {isCheckingUpdates ? (
                      <Loader2 className="size-3.5 animate-spin" />
                    ) : (
                      <RefreshCw className="size-3.5" />
                    )}
                    {t("central.checkUpdatesShort")}
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="secondary"
                    disabled={updateStatus?.status !== "update_available" || isUpdatingThisSkill}
                    onClick={onUpdateSkill}
                    className={inspectorActionButtonClassName}
                  >
                    {isUpdatingThisSkill ? (
                      <Loader2 className="size-3.5 animate-spin" />
                    ) : (
                      <Download className="size-3.5" />
                    )}
                    {t("central.updateShort")}
                  </Button>
                </div>
                {updateStatus?.source_url && (
                  <MetadataRow label={t("detail.updateSource")} value={updateStatus.source_url} />
                )}
                {updateStatus?.ref && (
                  <MetadataRow label={t("detail.updateRef")} value={updateStatus.ref} />
                )}
                {updateStatus?.source_path && (
                  <MetadataRow label={t("detail.sourcePath")} value={updateStatus.source_path} />
                )}
                {updateStatus?.last_checked_at && (
                  <MetadataRow
                    label={t("detail.lastCheckedAt")}
                    value={new Date(updateStatus.last_checked_at).toLocaleString()}
                  />
                )}
                {updateStatus?.error && (
                  <p className="text-xs leading-relaxed text-destructive">{updateStatus.error}</p>
                )}
              </div>
            </section>
          )}

          {detail.tags && detail.tags.length > 0 && (
            <section
              aria-label={t("detail.tags", {
                defaultValue: i18n.language.startsWith("zh") ? "分类标签" : "Tags",
              })}
            >
              <SectionLabel>
                {t("detail.tags", {
                  defaultValue: i18n.language.startsWith("zh") ? "分类标签" : "Tags",
                })}
              </SectionLabel>
              <div className="flex flex-wrap gap-1.5">
                {detail.tags.map((tag) => (
                  <span
                    key={tag.id}
                    className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[11px] font-medium text-muted-foreground ring-1 ring-border/70"
                    title={tag.description ?? tag.name}
                  >
                    <Tag className="size-2.5" />
                    {tag.name}
                  </span>
                ))}
              </div>
            </section>
          )}

          {detail.is_central && !detail.is_read_only && (
            <section aria-label={t("detail.metadataManagementRegion")}>
              <SectionLabel>{t("detail.metadataManagement")}</SectionLabel>
              <div className={cn(inspectorCardClassName, "space-y-4")}>
                <div data-testid="detail-repository-management" className="space-y-2">
                  <label htmlFor="detail-repository-select" className={inspectorFieldLabelClassName}>
                    {t("central.repositorySelectLabel")}
                  </label>
                  <select
                    id="detail-repository-select"
                    className={inspectorSelectClassName}
                    value={selectedRepositoryId}
                    aria-label={t("central.repositorySelectLabel")}
                    onChange={(event) => onSelectedRepositoryChange(event.target.value)}
                  >
                    <option value="">{t("central.chooseRepository")}</option>
                    {repositories.map((repository) => (
                      <option key={repository.id} value={repository.id}>
                        {repository.name}
                      </option>
                    ))}
                  </select>
                  <Button
                    type="button"
                    size="sm"
                    variant="secondary"
                    disabled={!selectedRepositoryId || isMetadataUpdating}
                    onClick={onAssignRepository}
                    className={inspectorActionButtonClassName}
                  >
                    {t("central.assignRepository")}
                  </Button>
                </div>
                <div data-testid="detail-tag-management" className="space-y-2">
                  <label htmlFor="detail-tag-select" className={inspectorFieldLabelClassName}>
                    {t("central.tagSelectLabel")}
                  </label>
                  <select
                    id="detail-tag-select"
                    className={inspectorSelectClassName}
                    value={selectedTagId}
                    aria-label={t("central.tagSelectLabel")}
                    onChange={(event) => onSelectedTagChange(event.target.value)}
                  >
                    <option value="">{t("central.chooseTag")}</option>
                    {tags.map((tag) => (
                      <option key={tag.id} value={tag.id}>
                        {tag.name}
                      </option>
                    ))}
                  </select>
                  <Button
                    type="button"
                    size="sm"
                    variant="secondary"
                    disabled={!selectedTagId || isMetadataUpdating}
                    onClick={onAssignTag}
                    className={inspectorActionButtonClassName}
                  >
                    {t("central.assignTag")}
                  </Button>
                </div>
              </div>
            </section>
          )}

          <section aria-label={t("detail.installStatusRegion")}>
            <SectionLabel>{t("detail.installStatus")}</SectionLabel>
            <div className="space-y-1.5">
              {detail.is_read_only ? (
                <p className="text-xs leading-relaxed text-muted-foreground">
                  {t("detail.readOnlyInstallBlocked", {
                    defaultValue: i18n.language.startsWith("zh")
                      ? "插件来源的只读副本不可安装或卸载。"
                      : "Install and uninstall are unavailable for read-only plugin copies.",
                  })}
                </p>
              ) : targetAgents.length === 0 ? (
                <p className="text-xs text-muted-foreground">{t("detail.noPlatforms")}</p>
              ) : (
                <>
                  <PlatformInstallStatusGroups
                    lobsterAgents={lobsterAgents}
                    codingAgents={codingAgents}
                    installationMap={installationMap}
                    sharedRootAgentIds={sharedRootAgentIds}
                    installingAgentId={installingAgentId}
                    onToggleInstall={onToggleInstall}
                    skillName={detail.name}
                  />
                  <InstalledPlatformList
                    rows={installedPlatformRows}
                    skillName={detail.name}
                    onRequestUninstall={onRequestUninstallInstalledPlatform}
                  />
                </>
              )}
            </div>
          </section>

          {detail.is_central && !detail.is_read_only && (
            <section aria-label={t("detail.projectsUsingSkillRegion")}>
              <SectionLabel>{t("detail.projectsUsingSkill")}</SectionLabel>
              {isLoadingProjectsUsingSkill ? (
                <p className="text-xs leading-relaxed text-muted-foreground">
                  {t("detail.projectsUsingSkillLoading")}
                </p>
              ) : projectsUsingSkill.length === 0 ? (
                <p className="text-xs leading-relaxed text-muted-foreground">
                  {t("detail.projectsUsingSkillEmpty")}
                </p>
              ) : (
                <ul className="space-y-1.5">
                  {projectsUsingSkill.map((row) => (
                    <li
                      key={`${row.projectId}:${row.agentId}`}
                      className="flex items-center gap-2 rounded-md border border-border/60 bg-muted/30 px-2 py-1.5"
                    >
                      <FolderOpen className="size-3.5 shrink-0 text-muted-foreground" />
                      <div className="min-w-0 flex-1 space-y-0.5">
                        <div
                          className="truncate text-xs font-medium text-foreground"
                          title={formatPathForDisplay(row.projectPath)}
                        >
                          {row.projectName}
                        </div>
                        <div className="truncate text-[11px] text-muted-foreground">
                          {row.agentDisplayName}
                        </div>
                      </div>
                      <span
                        className={cn(
                          "shrink-0 rounded-full px-1.5 py-0.5 text-[11px] font-medium ring-1",
                          row.linkType === "symlink"
                            ? "bg-success/10 text-success-foreground ring-success/20"
                            : "bg-warning/10 text-warning-foreground ring-warning/20"
                        )}
                      >
                        {row.linkType}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          )}

          <section aria-label={t("detail.collections")}>
            <SectionLabel>{t("detail.collections")}</SectionLabel>
            {detail.is_read_only ? (
              <p className="text-xs leading-relaxed text-muted-foreground">
                {t("detail.readOnlyCollectionsBlocked", {
                  defaultValue: i18n.language.startsWith("zh")
                    ? "插件来源的只读副本不可调整技能集。"
                    : "Collection management is unavailable for read-only plugin copies.",
                })}
              </p>
            ) : (
              <div className="flex flex-wrap items-center gap-1.5">
                {skillCollections.map((collection) => (
                  <span
                    key={collection.id}
                    className="inline-flex items-center gap-1 rounded-full bg-primary/10 px-2 py-0.5 text-[11px] font-medium text-primary ring-1 ring-primary/20"
                    title={collection.description ?? collection.name}
                  >
                    <Tag className="size-2.5" />
                    {collection.name}
                  </span>
                ))}
                <Button
                  ref={addToCollectionButtonRef}
                  variant="ghost"
                  size="sm"
                  className="h-8 gap-1 px-2 text-xs text-muted-foreground hover:text-foreground md:h-6"
                  aria-label={t("detail.addToCollection")}
                  onClick={onOpenCollectionPicker}
                >
                  <Plus className="size-3" />
                  {t("detail.addToCollection")}
                </Button>
              </div>
            )}
          </section>
        </>
      ) : null}
    </aside>
  );
}
