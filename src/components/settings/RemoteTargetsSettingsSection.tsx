import { Server } from "lucide-react";
import { useTranslation } from "react-i18next";

import { SettingsCollapsibleCard } from "@/components/settings/SettingsCollapsibleCard";
import type { TargetSummary } from "@/types";

import { RemoteTargetRow, SshTargetForm, WslTargetForm } from "./RemoteTargetsSettingsControls";
import type {
  SshTargetFormState,
  TargetMessage,
  WslDistributionOption,
  WslTargetFormState,
} from "./remoteTargetsSettingsTypes";

export type {
  SshTargetFormState,
  TargetMessage,
  WslDistributionOption,
  WslTargetFormState,
} from "./remoteTargetsSettingsTypes";

interface RemoteTargetsSettingsSectionProps {
  activeTarget: TargetSummary;
  deletingTargetId: string | null;
  editingTargetId: string | null;
  isCreatingTarget: boolean;
  isLoadingTargets: boolean;
  isLoadingWslDistributions: boolean;
  switchingTargetId: string | null;
  sshTargetEditForm: SshTargetFormState;
  sshTargetForm: SshTargetFormState;
  sshTargetPasswordUpdates: Record<string, string>;
  targetMessage: TargetMessage;
  targets: TargetSummary[];
  testingTargetId: string | null;
  updatingPasswordTargetId: string | null;
  updatingTargetId: string | null;
  wslDistributionError: string | null;
  wslDistributions: WslDistributionOption[];
  wslTargetEditForm: WslTargetFormState;
  wslTargetForm: WslTargetFormState;
  onCancelEditTarget: () => void;
  onCreateSshTarget: () => void;
  onCreateWslTarget: () => void;
  onDeleteTarget: (targetId: string) => void;
  onOpenLocalRemoteSync: (targetId: string) => void;
  onRefreshWslDistributions: () => void;
  onStartEditTarget: (target: TargetSummary) => void;
  onSwitchTarget: (targetId: string) => void;
  onTestExistingTarget: (target: TargetSummary) => void;
  onTestNewSshTarget: () => void;
  onTestNewWslTarget: () => void;
  onUpdateExistingTargetPassword: (targetId: string, value: string) => void;
  onUpdateSshTarget: (target: TargetSummary) => void;
  onUpdateSshTargetEditForm: (
    field: keyof SshTargetFormState,
    value: string
  ) => void;
  onUpdateSshTargetForm: (
    field: keyof SshTargetFormState,
    value: string
  ) => void;
  onUpdateTargetPassword: (target: TargetSummary) => void;
  onUpdateWslTarget: (target: TargetSummary) => void;
  onUpdateWslTargetEditForm: (
    field: keyof WslTargetFormState,
    value: string
  ) => void;
  onUpdateWslTargetForm: (
    field: keyof WslTargetFormState,
    value: string
  ) => void;
}

export function RemoteTargetsSettingsSection({
  activeTarget,
  deletingTargetId,
  editingTargetId,
  isCreatingTarget,
  isLoadingTargets,
  isLoadingWslDistributions,
  switchingTargetId,
  sshTargetEditForm,
  sshTargetForm,
  sshTargetPasswordUpdates,
  targetMessage,
  targets,
  testingTargetId,
  updatingPasswordTargetId,
  updatingTargetId,
  wslDistributionError,
  wslDistributions,
  wslTargetEditForm,
  wslTargetForm,
  onCancelEditTarget,
  onCreateSshTarget,
  onCreateWslTarget,
  onDeleteTarget,
  onOpenLocalRemoteSync,
  onRefreshWslDistributions,
  onStartEditTarget,
  onSwitchTarget,
  onTestExistingTarget,
  onTestNewSshTarget,
  onTestNewWslTarget,
  onUpdateExistingTargetPassword,
  onUpdateSshTarget,
  onUpdateSshTargetEditForm,
  onUpdateSshTargetForm,
  onUpdateTargetPassword,
  onUpdateWslTarget,
  onUpdateWslTargetEditForm,
  onUpdateWslTargetForm,
}: RemoteTargetsSettingsSectionProps) {
  const { t } = useTranslation();
  const existingDistributions = new Set(
    targets
      .filter((target) => target.kind === "wsl" && target.distribution)
      .map((target) => target.distribution!.toLowerCase())
  );

  return (
    <SettingsCollapsibleCard
      sectionId="remote-targets"
      title={t("targets.title")}
      description={t("targets.description")}
      icon={<Server className="size-5 shrink-0 text-muted-foreground" />}
    >
      <div className="space-y-4">
        {targetMessage && (
          <p
            className={`text-xs ${targetMessage.type === "success" ? "text-primary" : "text-destructive"}`}
            role="status"
          >
            {targetMessage.text}
          </p>
        )}

        <div className="overflow-hidden rounded-xl border border-border bg-card/35">
          {targets.map((target) => (
            <RemoteTargetRow
              key={target.id}
              activeTarget={activeTarget}
              deletingTargetId={deletingTargetId}
              editingTargetId={editingTargetId}
              isLoadingTargets={isLoadingTargets}
              switchingTargetId={switchingTargetId}
              sshTargetEditForm={sshTargetEditForm}
              sshTargetPasswordUpdates={sshTargetPasswordUpdates}
              target={target}
              testingTargetId={testingTargetId}
              updatingPasswordTargetId={updatingPasswordTargetId}
              updatingTargetId={updatingTargetId}
              wslTargetEditForm={wslTargetEditForm}
              onCancelEditTarget={onCancelEditTarget}
              onDeleteTarget={onDeleteTarget}
              onOpenLocalRemoteSync={onOpenLocalRemoteSync}
              onStartEditTarget={onStartEditTarget}
              onSwitchTarget={onSwitchTarget}
              onTestExistingTarget={onTestExistingTarget}
              onUpdateExistingTargetPassword={onUpdateExistingTargetPassword}
              onUpdateSshTarget={onUpdateSshTarget}
              onUpdateSshTargetEditForm={onUpdateSshTargetEditForm}
              onUpdateTargetPassword={onUpdateTargetPassword}
              onUpdateWslTarget={onUpdateWslTarget}
              onUpdateWslTargetEditForm={onUpdateWslTargetEditForm}
            />
          ))}
        </div>

        <div className="grid gap-4 2xl:grid-cols-2">
          <div className="space-y-4 rounded-xl border border-border/70 bg-muted/20 p-4 xl:p-5">
            <div>
              <div className="text-sm font-medium">{t("targets.addSshTitle")}</div>
              <div className="text-xs text-muted-foreground">
                {t("targets.addSshDescription")}
              </div>
            </div>
            <SshTargetForm
              form={sshTargetForm}
              isCreatingTarget={isCreatingTarget}
              isTesting={testingTargetId === "new"}
              onCreate={onCreateSshTarget}
              onTest={onTestNewSshTarget}
              onUpdate={onUpdateSshTargetForm}
            />
          </div>

          <div className="space-y-4 rounded-xl border border-border/70 bg-muted/20 p-4 xl:p-5">
            <div>
              <div className="text-sm font-medium">{t("targets.addWslTitle")}</div>
              <div className="text-xs text-muted-foreground">
                {t("targets.addWslDescription")}
              </div>
            </div>
            <WslTargetForm
              form={wslTargetForm}
              isCreatingTarget={isCreatingTarget}
              isLoadingDistributions={isLoadingWslDistributions}
              isTesting={testingTargetId === "new-wsl"}
              distributionError={wslDistributionError}
              distributions={wslDistributions}
              existingDistributions={existingDistributions}
              onRefreshDistributions={onRefreshWslDistributions}
              onCreate={onCreateWslTarget}
              onTest={onTestNewWslTarget}
              onUpdate={onUpdateWslTargetForm}
            />
          </div>
        </div>

        <div className="space-y-1 text-xs text-muted-foreground">
          <p>{t("targets.sshHint")}</p>
          <p>{t("targets.passwordRepairHint")}</p>
          <p>{t("targets.sshKeyPathHelp")}</p>
          <p>{t("targets.wslHint")}</p>
        </div>
      </div>
    </SettingsCollapsibleCard>
  );
}
