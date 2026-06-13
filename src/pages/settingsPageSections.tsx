import { ACCENT_NAMES, type CatppuccinAccent, type ThemeFlavor } from "@/stores/themeStore";
import { AddDirectoryDialog } from "@/components/settings/AddDirectoryDialog";
import { AboutSettingsSection } from "@/components/settings/AboutSettingsSection";
import { AiSettingsSection } from "@/components/settings/AiSettingsSection";
import { AppearanceSettingsSection } from "@/components/settings/AppearanceSettingsSection";
import { CustomPlatformsSettingsSection } from "@/components/settings/CustomPlatformsSettingsSection";
import { CentralUpdateCheckModeSettingsSection } from "@/components/settings/CentralUpdateCheckModeSettingsSection";
import { GitHubPatSettingsSection } from "@/components/settings/GitHubPatSettingsSection";
import { LocalRemoteSyncDialog } from "@/components/settings/LocalRemoteSyncDialog";
import { PlatformDialog } from "@/components/settings/PlatformDialog";
import { PlatformVisibilitySettingsSection } from "@/components/settings/PlatformVisibilitySettingsSection";
import {
  RemoteTargetsSettingsSection,
  type SshTargetFormState,
  type WslTargetFormState,
} from "@/components/settings/RemoteTargetsSettingsSection";
import { ScanDirectoriesSettingsSection } from "@/components/settings/ScanDirectoriesSettingsSection";
import type {
  SettingsPageDefinition,
  SettingsPageId,
} from "@/components/settings/settingsPages";
import type { PlatformVisibilityGroupViewModel } from "@/pages/settingsViewModel";
import type { PlatformCategoryKey } from "@/lib/platformVisibility";
import type {
  AgentWithStatus,
  AiApiKeyState,
  GitHubPatState,
  LocalRemoteSyncApplyResult,
  LocalRemoteSyncPreview,
  ScanDirectory,
  TargetSummary,
  WslDistributionSummary,
} from "@/types";
import type {
  AiConnectionTestResult,
  AiSaveStatus,
  AiSettings,
} from "@/stores/settingsStore";
import type { UpdateCheckMode } from "@/pages/centralUpdateCheckMode";

type Message = { type: "success" | "error"; text: string; detail?: string | null };
type CatppuccinVarMap = Record<CatppuccinAccent, string>;

export interface SettingsPageSectionsProps {
  accent: CatppuccinAccent;
  activeTarget: TargetSummary;
  aiApiKeyState: AiApiKeyState;
  aiSaveError: string | null;
  aiSaveStatus: AiSaveStatus;
  aiSettings: AiSettings;
  aiTestResult: AiConnectionTestResult | null;
  aiTesting: boolean;
  appVersion: string;
  ctpVarMap: CatppuccinVarMap;
  customAgents: AgentWithStatus[];
  dbPathDisplay: string;
  deletingTargetId: string | null;
  editingPlatform: AgentWithStatus | null;
  editingTargetId: string | null;
  flavor: ThemeFlavor;
  flavorColors: Record<ThemeFlavor, string>;
  flavorOrder: ThemeFlavor[];
  githubPatInput: string;
  githubPatMessage: Message | null;
  githubPatState: GitHubPatState;
  isAddDirOpen: boolean;
  isApplyingLocalRemoteSync: boolean;
  isCreatingTarget: boolean;
  isLoadingAiSettings: boolean;
  isLoadingGitHubPat: boolean;
  isLoadingScanDirs: boolean;
  isLoadingTargets: boolean;
  isLoadingWslDistributions: boolean;
  isPlatformDialogOpen: boolean;
  isPreviewingLocalRemoteSync: boolean;
  isSavingGitHubPat: boolean;
  isTestingGitHubPat: boolean;
  lang: string;
  localRemoteSyncError: string | null;
  localRemoteSyncPreview: LocalRemoteSyncPreview | null;
  localRemoteSyncResult: LocalRemoteSyncApplyResult | null;
  localRemoteSyncTarget: TargetSummary | undefined;
  normalizedPlatformVisibilityQuery: string;
  page: SettingsPageDefinition;
  platformError: string | null;
  platformVisibilityGroups: PlatformVisibilityGroupViewModel[];
  platformVisibilityQuery: string;
  platformVisibilitySearchActive: boolean;
  removingAgent: string | null;
  removingDir: string | null;
  repoUrl: string;
  resolvedUrl: string;
  scanDirError: string | null;
  scanDirectories: ScanDirectory[];
  centralUpdateCheckMode: UpdateCheckMode;
  isLoadingCentralUpdateCheckMode: boolean;
  showAiTestDetails: boolean;
  showBuiltinDirs: boolean;
  sshTargetEditForm: SshTargetFormState;
  sshTargetForm: SshTargetFormState;
  sshTargetPasswordUpdates: Record<string, string>;
  switchingTargetId: string | null;
  targetMessage: { type: "success" | "error"; text: string } | null;
  targets: TargetSummary[];
  testingTargetId: string | null;
  updatingPasswordTargetId: string | null;
  updatingTargetId: string | null;
  wslDistributionError: string | null;
  wslDistributions: WslDistributionSummary[];
  wslTargetEditForm: WslTargetFormState;
  wslTargetForm: WslTargetFormState;
  onAddDirectory: (path: string) => Promise<void>;
  onAddPlatform: (
    displayName: string,
    globalSkillsDir: string,
    category?: string
  ) => Promise<void>;
  onApplyLocalRemoteSync: () => void;
  onCancelEditTarget: () => void;
  onClearAiApiKey: () => void;
  onClearGitHubPat: () => void;
  onCreateSshTarget: () => void;
  onCreateWslTarget: () => void;
  onDeleteTarget: (targetId: string) => void;
  onEditPlatform: (
    displayName: string,
    globalSkillsDir: string,
    category?: string
  ) => Promise<void>;
  onGitHubPatInputChange: (value: string) => void;
  onLocalRemoteSyncOpenChange: (open: boolean) => void;
  onOpenAddDirectory: () => void;
  onOpenAddDirectoryChange: (open: boolean) => void;
  onOpenAddPlatform: () => void;
  onOpenEditPlatform: (agent: AgentWithStatus) => void;
  onOpenLocalRemoteSync: (targetId: string) => void;
  onPlatformDialogOpenChange: (open: boolean) => void;
  onPreviewLocalRemoteSync: () => void;
  onProviderChange: (id: string) => void;
  onRefreshWslDistributions: () => void;
  onRevealAiApiKey: (providerId: string) => Promise<string | null>;
  onRevealGitHubPat: () => Promise<string | null>;
  onRemoveDirectory: (path: string) => void;
  onRemovePlatform: (agentId: string) => void;
  onSaveGitHubPat: () => void;
  onSetAccent: (accent: CatppuccinAccent) => void;
  onSetFlavor: (flavor: ThemeFlavor) => void;
  onSetPlatformVisibilityQuery: (value: string) => void;
  onSetShowAiTestDetails: (value: boolean | ((current: boolean) => boolean)) => void;
  onToggleBuiltinDirs: () => void;
  onStartEditTarget: (target: TargetSummary) => void;
  onSwitchTarget: (targetId: string) => void;
  onTestAiConnection: () => Promise<void>;
  onTestExistingTarget: (target: TargetSummary) => void;
  onTestGitHubPat: () => void;
  onTestNewSshTarget: () => void;
  onTestNewWslTarget: () => void;
  onToggleCategory: (category: PlatformCategoryKey, visible: boolean) => void;
  onToggleDirectory: (path: string, active: boolean) => void;
  onCentralUpdateCheckModeChange: (mode: UpdateCheckMode) => void;
  onTogglePlatformVisibility: (agentId: string, enabled: boolean) => void;
  onUpdateAiSettings: (updates: Partial<AiSettings>) => void;
  onUpdateExistingTargetPassword: (targetId: string, password: string) => void;
  onUpdateSshTarget: (target: TargetSummary) => void;
  onUpdateSshTargetEditForm: (field: keyof SshTargetFormState, value: string) => void;
  onUpdateSshTargetForm: (field: keyof SshTargetFormState, value: string) => void;
  onUpdateTargetPassword: (target: TargetSummary) => void;
  onUpdateWslTarget: (target: TargetSummary) => void;
  onUpdateWslTargetEditForm: (field: keyof WslTargetFormState, value: string) => void;
  onUpdateWslTargetForm: (field: keyof WslTargetFormState, value: string) => void;
}

export function SettingsPageSections(props: SettingsPageSectionsProps) {
  return (
    <>
      {renderSettingsPage(props.page.id, props)}
      <AddDirectoryDialog
        open={props.isAddDirOpen}
        onOpenChange={props.onOpenAddDirectoryChange}
        onAdd={props.onAddDirectory}
      />
      <PlatformDialog
        open={props.isPlatformDialogOpen}
        onOpenChange={props.onPlatformDialogOpenChange}
        platform={props.editingPlatform}
        onAdd={props.onAddPlatform}
        onEdit={props.onEditPlatform}
      />
      <LocalRemoteSyncDialog
        open={Boolean(props.localRemoteSyncTarget)}
        targetLabel={props.localRemoteSyncTarget?.label ?? ""}
        preview={props.localRemoteSyncPreview}
        result={props.localRemoteSyncResult}
        isPreviewing={props.isPreviewingLocalRemoteSync}
        isApplying={props.isApplyingLocalRemoteSync}
        error={props.localRemoteSyncError}
        onOpenChange={props.onLocalRemoteSyncOpenChange}
        onPreview={props.onPreviewLocalRemoteSync}
        onApply={props.onApplyLocalRemoteSync}
      />
    </>
  );
}

function renderSettingsPage(pageId: SettingsPageId, props: SettingsPageSectionsProps) {
  switch (pageId) {
    case "connections":
      return <SettingsConnectionsPage {...props} />;
    case "platforms":
      return <SettingsPlatformsPage {...props} />;
    case "integrations":
      return <SettingsIntegrationsPage {...props} />;
    case "skill-sources":
      return <SettingsSkillSourcesPage {...props} />;
    case "about":
      return <SettingsAboutPage {...props} />;
    case "appearance":
    default:
      return <SettingsAppearancePage {...props} />;
  }
}

function SettingsAppearancePage(props: SettingsPageSectionsProps) {
  return (
    <section id="appearance-section" className="scroll-mt-24">
      <AppearanceSettingsSection
        accent={props.accent}
        accentNames={ACCENT_NAMES}
        ctpVarMap={props.ctpVarMap}
        flavor={props.flavor}
        flavorColors={props.flavorColors}
        flavorOrder={props.flavorOrder}
        onSetAccent={props.onSetAccent}
        onSetFlavor={props.onSetFlavor}
      />
    </section>
  );
}

function SettingsConnectionsPage(props: SettingsPageSectionsProps) {
  return (
    <section id="remote-targets-section" className="scroll-mt-24">
      <RemoteTargetsSettingsSection
        activeTarget={props.activeTarget}
        deletingTargetId={props.deletingTargetId}
        editingTargetId={props.editingTargetId}
        isCreatingTarget={props.isCreatingTarget}
        isLoadingTargets={props.isLoadingTargets}
        isLoadingWslDistributions={props.isLoadingWslDistributions}
        switchingTargetId={props.switchingTargetId}
        sshTargetEditForm={props.sshTargetEditForm}
        sshTargetForm={props.sshTargetForm}
        sshTargetPasswordUpdates={props.sshTargetPasswordUpdates}
        targetMessage={props.targetMessage}
        targets={props.targets}
        testingTargetId={props.testingTargetId}
        updatingPasswordTargetId={props.updatingPasswordTargetId}
        updatingTargetId={props.updatingTargetId}
        wslDistributionError={props.wslDistributionError}
        wslDistributions={props.wslDistributions}
        wslTargetEditForm={props.wslTargetEditForm}
        wslTargetForm={props.wslTargetForm}
        onCancelEditTarget={props.onCancelEditTarget}
        onCreateSshTarget={props.onCreateSshTarget}
        onCreateWslTarget={props.onCreateWslTarget}
        onDeleteTarget={props.onDeleteTarget}
        onOpenLocalRemoteSync={props.onOpenLocalRemoteSync}
        onRefreshWslDistributions={props.onRefreshWslDistributions}
        onStartEditTarget={props.onStartEditTarget}
        onSwitchTarget={props.onSwitchTarget}
        onTestExistingTarget={props.onTestExistingTarget}
        onTestNewSshTarget={props.onTestNewSshTarget}
        onTestNewWslTarget={props.onTestNewWslTarget}
        onUpdateExistingTargetPassword={props.onUpdateExistingTargetPassword}
        onUpdateSshTarget={props.onUpdateSshTarget}
        onUpdateSshTargetEditForm={props.onUpdateSshTargetEditForm}
        onUpdateSshTargetForm={props.onUpdateSshTargetForm}
        onUpdateTargetPassword={props.onUpdateTargetPassword}
        onUpdateWslTarget={props.onUpdateWslTarget}
        onUpdateWslTargetEditForm={props.onUpdateWslTargetEditForm}
        onUpdateWslTargetForm={props.onUpdateWslTargetForm}
      />
    </section>
  );
}

function SettingsPlatformsPage(props: SettingsPageSectionsProps) {
  return (
    <>
      <section id="custom-platforms-section" className="scroll-mt-24">
        <CustomPlatformsSettingsSection
          customAgents={props.customAgents}
          platformError={props.platformError}
          removingAgent={props.removingAgent}
          onAddPlatform={props.onOpenAddPlatform}
          onEditPlatform={props.onOpenEditPlatform}
          onRemovePlatform={props.onRemovePlatform}
        />
      </section>
      <section id="platform-visibility-section" className="scroll-mt-24">
        <PlatformVisibilitySettingsSection
          groups={props.platformVisibilityGroups}
          isSearchActive={props.platformVisibilitySearchActive}
          normalizedQuery={props.normalizedPlatformVisibilityQuery}
          query={props.platformVisibilityQuery}
          onQueryChange={props.onSetPlatformVisibilityQuery}
          onToggleCategory={props.onToggleCategory}
          onTogglePlatform={props.onTogglePlatformVisibility}
        />
      </section>
    </>
  );
}

function SettingsIntegrationsPage(props: SettingsPageSectionsProps) {
  return (
    <div className="space-y-4">
      <section id="github-pat-section" className="scroll-mt-24">
        <GitHubPatSettingsSection
          githubPatState={props.githubPatState}
          githubPatInput={props.githubPatInput}
          githubPatMessage={props.githubPatMessage}
          isLoadingGitHubPat={props.isLoadingGitHubPat}
          isSavingGitHubPat={props.isSavingGitHubPat}
          isTestingGitHubPat={props.isTestingGitHubPat}
          onClear={props.onClearGitHubPat}
          onInputChange={props.onGitHubPatInputChange}
          onReveal={props.onRevealGitHubPat}
          onSave={props.onSaveGitHubPat}
          onTest={props.onTestGitHubPat}
        />
      </section>
      <section id="ai-section" className="scroll-mt-24">
        <AiSettingsSection
          aiSaveError={props.aiSaveError}
          aiSaveStatus={props.aiSaveStatus}
          aiApiKeyState={props.aiApiKeyState}
          aiSettings={props.aiSettings}
          aiTestResult={props.aiTestResult}
          aiTesting={props.aiTesting}
          isLoadingAiSettings={props.isLoadingAiSettings}
          lang={props.lang}
          resolvedUrl={props.resolvedUrl}
          showAiTestDetails={props.showAiTestDetails}
          onClearApiKey={props.onClearAiApiKey}
          onProviderChange={props.onProviderChange}
          onRevealApiKey={props.onRevealAiApiKey}
          onSetShowAiTestDetails={props.onSetShowAiTestDetails}
          onTestConnection={props.onTestAiConnection}
          onUpdateAiSettings={props.onUpdateAiSettings}
        />
      </section>
    </div>
  );
}

function SettingsSkillSourcesPage(props: SettingsPageSectionsProps) {
  return (
    <div className="space-y-4">
      <section id="central-update-check-mode-section" className="scroll-mt-24">
        <CentralUpdateCheckModeSettingsSection
          mode={props.centralUpdateCheckMode}
          isLoading={props.isLoadingCentralUpdateCheckMode}
          onChange={props.onCentralUpdateCheckModeChange}
        />
      </section>
      <section id="scan-directories-section" className="scroll-mt-24">
        <ScanDirectoriesSettingsSection
          isLoadingScanDirs={props.isLoadingScanDirs}
          removingDir={props.removingDir}
          scanDirError={props.scanDirError}
          scanDirectories={props.scanDirectories}
          showBuiltinDirs={props.showBuiltinDirs}
          onAddDirectory={props.onOpenAddDirectory}
          onRemoveDirectory={props.onRemoveDirectory}
          onToggleBuiltinDirs={props.onToggleBuiltinDirs}
          onToggleDirectory={props.onToggleDirectory}
        />
      </section>
    </div>
  );
}

function SettingsAboutPage(props: SettingsPageSectionsProps) {
  return (
    <section id="about-section" className="scroll-mt-24">
      <AboutSettingsSection
        appVersion={props.appVersion}
        dbPathDisplay={props.dbPathDisplay}
        repoUrl={props.repoUrl}
      />
    </section>
  );
}
