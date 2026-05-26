import { Loader2, Pencil, Plus, Server, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { Input } from "@/components/ui/input";
import {
  isRemoteLikeTarget,
  isSshTarget,
  requiresSshPasswordRepair,
} from "@/lib/targetKind";
import type { TargetSummary } from "@/types";

import type {
  SshTargetFormState,
  WslDistributionOption,
  WslTargetFormState,
} from "./remoteTargetsSettingsTypes";
import {
  FormActionButtons,
  SshAuthMethodButtons,
  TargetTextField,
} from "./RemoteTargetFormPrimitives";
import {
  TARGET_EDIT_GRID_CLASS,
  TARGET_FORM_GRID_CLASS,
  TARGET_ROW_ACTION_CLASS,
  WSL_FORM_GRID_CLASS,
  sshCredentialStatus,
  sshCredentialStatusClass,
  targetDescription,
  targetKindLabel,
  targetTitle,
} from "./remoteTargetsSettingsLayout";
import { RemoteTargetSyncButton } from "./RemoteTargetSyncButton";

interface RemoteTargetRowProps {
  activeTarget: TargetSummary;
  deletingTargetId: string | null;
  editingTargetId: string | null;
  isLoadingTargets: boolean;
  switchingTargetId: string | null;
  sshTargetEditForm: SshTargetFormState;
  sshTargetPasswordUpdates: Record<string, string>;
  target: TargetSummary;
  testingTargetId: string | null;
  updatingPasswordTargetId: string | null;
  updatingTargetId: string | null;
  wslTargetEditForm: WslTargetFormState;
  onCancelEditTarget: () => void;
  onDeleteTarget: (targetId: string) => void;
  onOpenLocalRemoteSync: (targetId: string) => void;
  onStartEditTarget: (target: TargetSummary) => void;
  onSwitchTarget: (targetId: string) => void;
  onTestExistingTarget: (target: TargetSummary) => void;
  onUpdateExistingTargetPassword: (targetId: string, value: string) => void;
  onUpdateSshTarget: (target: TargetSummary) => void;
  onUpdateSshTargetEditForm: (
    field: keyof SshTargetFormState,
    value: string,
  ) => void;
  onUpdateTargetPassword: (target: TargetSummary) => void;
  onUpdateWslTarget: (target: TargetSummary) => void;
  onUpdateWslTargetEditForm: (
    field: keyof WslTargetFormState,
    value: string,
  ) => void;
}

export function RemoteTargetRow({
  activeTarget,
  deletingTargetId,
  editingTargetId,
  isLoadingTargets,
  switchingTargetId,
  sshTargetEditForm,
  sshTargetPasswordUpdates,
  target,
  testingTargetId,
  updatingPasswordTargetId,
  updatingTargetId,
  wslTargetEditForm,
  onCancelEditTarget,
  onDeleteTarget,
  onOpenLocalRemoteSync,
  onStartEditTarget,
  onSwitchTarget,
  onTestExistingTarget,
  onUpdateExistingTargetPassword,
  onUpdateSshTarget,
  onUpdateSshTargetEditForm,
  onUpdateTargetPassword,
  onUpdateWslTarget,
  onUpdateWslTargetEditForm,
}: RemoteTargetRowProps) {
  const { t } = useTranslation();
  const isLocal = target.kind === "local";
  const isActive = target.id === activeTarget.id || target.isActive;
  const passwordUpdateValue = sshTargetPasswordUpdates[target.id] ?? "";
  const credentialStatus = sshCredentialStatus(target);
  const isEditing = editingTargetId === target.id;

  return (
    <div className="border-b border-border/50 last:border-0">
      <div className="grid gap-3 px-4 py-3 xl:grid-cols-[minmax(0,1fr)_auto] xl:items-start">
        <div className="flex min-w-0 gap-3">
          <Server className="mt-1 size-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="min-w-0 max-w-full truncate text-sm font-medium">
                {targetTitle(target, t)}
              </span>
              <span className="rounded-full border border-border px-2 py-0.5 text-[11px] text-muted-foreground">
                {targetKindLabel(target, t)}
              </span>
              {isActive && (
                <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[11px] text-primary">
                  {t("targets.active")}
                </span>
              )}
            </div>
            <div
              className="mt-0.5 break-all text-xs leading-5 text-muted-foreground"
              title={targetDescription(target, t)}
            >
              {targetDescription(target, t)}
            </div>
            {isRemoteLikeTarget(target) && target.cacheDbPath && (
              <div
                className="mt-0.5 break-all font-mono text-[11px] leading-5 text-muted-foreground/80"
                title={target.cacheDbPath}
              >
                {target.cacheDbPath}
              </div>
            )}
            {requiresSshPasswordRepair(target) && (
              <PasswordRepairRow
                credentialError={target.credentialError}
                credentialStatus={credentialStatus}
                disabled={
                  updatingPasswordTargetId === target.id ||
                  testingTargetId === target.id ||
                  !passwordUpdateValue.trim()
                }
                isUpdating={updatingPasswordTargetId === target.id}
                label={target.label}
                value={passwordUpdateValue}
                onChange={(value) =>
                  onUpdateExistingTargetPassword(target.id, value)
                }
                onUpdate={() => onUpdateTargetPassword(target)}
              />
            )}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 xl:justify-end">
          {!isLocal && (
            <RemoteTargetSyncButton
              targetId={target.id}
              className={TARGET_ROW_ACTION_CLASS}
              onOpenLocalRemoteSync={onOpenLocalRemoteSync}
            />
          )}
          {!isLocal && (
            <Button
              type="button"
              variant={isEditing ? "secondary" : "outline"}
              size="sm"
              disabled={updatingTargetId === target.id}
              onClick={() =>
                isEditing ? onCancelEditTarget() : onStartEditTarget(target)
              }
              aria-label={t("targets.editLabel", { label: target.label })}
              className={TARGET_ROW_ACTION_CLASS}
            >
              <Pencil className="size-3.5" />
              {isEditing ? t("common.cancel") : t("common.edit")}
            </Button>
          )}
          {!isLocal && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={testingTargetId === target.id}
              onClick={() => onTestExistingTarget(target)}
              className={TARGET_ROW_ACTION_CLASS}
            >
              {testingTargetId === target.id ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : null}
              {t("targets.test")}
            </Button>
          )}
          {!isActive && (
            <Button
              type="button"
              variant="secondary"
              size="sm"
              disabled={switchingTargetId === target.id || isLoadingTargets}
              onClick={() => onSwitchTarget(target.id)}
              className={TARGET_ROW_ACTION_CLASS}
            >
              {switchingTargetId === target.id ? (
                <Loader2 className="size-3.5 animate-spin" />
              ) : null}
              {t("targets.activate")}
            </Button>
          )}
          {!isLocal && (
            <InlineConfirmAction
              onConfirm={() => onDeleteTarget(target.id)}
              isLoading={deletingTargetId === target.id}
              idleAriaLabel={t("targets.deleteLabel", { label: target.label })}
              idleTitle={t("targets.deleteLabel", { label: target.label })}
              confirmLabel={t("common.confirmDelete")}
              icon={<Trash2 className="size-3.5" />}
              className="shrink-0"
            />
          )}
        </div>
      </div>
      {!isLocal && isEditing && (
        <div className="px-4 pb-4 xl:pl-11">
          {isSshTarget(target) ? (
            <SshTargetEditForm
              form={sshTargetEditForm}
              target={target}
              updatingTargetId={updatingTargetId}
              onCancel={onCancelEditTarget}
              onSave={() => onUpdateSshTarget(target)}
              onUpdate={onUpdateSshTargetEditForm}
            />
          ) : (
            <WslTargetEditForm
              form={wslTargetEditForm}
              target={target}
              updatingTargetId={updatingTargetId}
              onCancel={onCancelEditTarget}
              onSave={() => onUpdateWslTarget(target)}
              onUpdate={onUpdateWslTargetEditForm}
            />
          )}
        </div>
      )}
    </div>
  );
}

interface PasswordRepairRowProps {
  credentialError?: string | null;
  credentialStatus: string | null;
  disabled: boolean;
  isUpdating: boolean;
  label: string;
  value: string;
  onChange: (value: string) => void;
  onUpdate: () => void;
}

function PasswordRepairRow({
  credentialError,
  credentialStatus,
  disabled,
  isUpdating,
  label,
  value,
  onChange,
  onUpdate,
}: PasswordRepairRowProps) {
  const { t } = useTranslation();
  const credentialStatusLabel = t(
    `targets.credentialStatus.${credentialStatus ?? "missing"}`,
  );

  return (
    <div className="mt-2 max-w-2xl space-y-2">
      <div className="flex flex-wrap items-center gap-2">
        <span
          title={credentialStatusLabel}
          className={`rounded-full border px-2 py-0.5 text-[11px] ${sshCredentialStatusClass(credentialStatus)}`}
        >
          {credentialStatusLabel}
        </span>
        {credentialError ? (
          <span
            className="break-all text-[11px] text-muted-foreground"
            title={credentialError}
          >
            {credentialError}
          </span>
        ) : null}
      </div>
      <div className="flex flex-col gap-2 sm:flex-row">
        <Input
          type="password"
          value={value}
          onChange={(event) => onChange(event.target.value)}
          placeholder={t("targets.updatePasswordPlaceholder")}
          aria-label={t("targets.updatePasswordFor", { label })}
          className="h-8 min-w-0 text-xs"
        />
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={disabled}
          onClick={onUpdate}
        >
          {isUpdating ? <Loader2 className="size-3.5 animate-spin" /> : null}
          {t("targets.updatePassword")}
        </Button>
      </div>
    </div>
  );
}

interface SshTargetEditFormProps {
  form: SshTargetFormState;
  target: TargetSummary;
  updatingTargetId: string | null;
  onCancel: () => void;
  onSave: () => void;
  onUpdate: (field: keyof SshTargetFormState, value: string) => void;
}

function SshTargetEditForm({
  form,
  target,
  updatingTargetId,
  onCancel,
  onSave,
  onUpdate,
}: SshTargetEditFormProps) {
  const { t } = useTranslation();

  return (
    <div className={TARGET_EDIT_GRID_CLASS}>
      <SshAuthMethodButtons
        id={`edit-ssh-auth-method-${target.id}`}
        value={form.authMethod}
        onChange={(method) => onUpdate("authMethod", method)}
      />
      <TargetTextField
        id={`edit-ssh-label-${target.id}`}
        label={t("targets.labelPlaceholder")}
        value={form.label}
        onChange={(value) => onUpdate("label", value)}
        placeholder={t("targets.labelPlaceholder")}
      />
      <TargetTextField
        id={`edit-ssh-host-${target.id}`}
        label={t("targets.hostPlaceholder")}
        value={form.host}
        onChange={(value) => onUpdate("host", value)}
        placeholder={t("targets.hostPlaceholder")}
      />
      <TargetTextField
        id={`edit-ssh-username-${target.id}`}
        label={t("targets.usernamePlaceholder")}
        value={form.username}
        onChange={(value) => onUpdate("username", value)}
        placeholder={t("targets.usernamePlaceholder")}
      />
      <TargetTextField
        id={`edit-ssh-port-${target.id}`}
        label={t("targets.portLabel")}
        value={form.port}
        onChange={(value) => onUpdate("port", value)}
        placeholder="22"
      />
      <div className="space-y-1 md:col-span-2 xl:col-span-4">
        <label
          htmlFor={`edit-ssh-credential-${target.id}`}
          className="text-xs text-muted-foreground"
        >
          {form.authMethod === "key"
            ? t("targets.keyPathPlaceholder")
            : t("targets.passwordPlaceholder")}
        </label>
        <Input
          id={`edit-ssh-credential-${target.id}`}
          type={form.authMethod === "password" ? "password" : "text"}
          value={form.authMethod === "key" ? form.keyPath : form.password}
          onChange={(event) =>
            onUpdate(
              form.authMethod === "key" ? "keyPath" : "password",
              event.target.value,
            )
          }
          placeholder={
            form.authMethod === "key"
              ? t("targets.keyPathPlaceholder")
              : target.authMethod === "password"
                ? t("targets.passwordOptionalPlaceholder")
                : t("targets.passwordPlaceholder")
          }
        />
      </div>
      <FormActionButtons
        isSaving={updatingTargetId === target.id}
        onCancel={onCancel}
        onSave={onSave}
      />
    </div>
  );
}

interface WslTargetEditFormProps {
  form: WslTargetFormState;
  target: TargetSummary;
  updatingTargetId: string | null;
  onCancel: () => void;
  onSave: () => void;
  onUpdate: (field: keyof WslTargetFormState, value: string) => void;
}

function WslTargetEditForm({
  form,
  target,
  updatingTargetId,
  onCancel,
  onSave,
  onUpdate,
}: WslTargetEditFormProps) {
  const { t } = useTranslation();

  return (
    <div className="grid gap-3 rounded-lg border border-border/70 bg-muted/20 p-3 md:grid-cols-2 xl:grid-cols-4">
      <TargetTextField
        id={`edit-wsl-label-${target.id}`}
        label={t("targets.labelPlaceholder")}
        value={form.label}
        onChange={(value) => onUpdate("label", value)}
        placeholder={t("targets.labelPlaceholder")}
      />
      <TargetTextField
        id={`edit-wsl-distribution-${target.id}`}
        label={t("targets.wslDistributionLabel")}
        value={form.distribution}
        onChange={(value) => onUpdate("distribution", value)}
        placeholder={t("targets.wslDistributionPlaceholder")}
      />
      <FormActionButtons
        isSaving={updatingTargetId === target.id}
        onCancel={onCancel}
        onSave={onSave}
      />
    </div>
  );
}

interface SshTargetFormProps {
  form: SshTargetFormState;
  isCreatingTarget: boolean;
  isTesting: boolean;
  onCreate: () => void;
  onTest: () => void;
  onUpdate: (field: keyof SshTargetFormState, value: string) => void;
}

export function SshTargetForm({
  form,
  isCreatingTarget,
  isTesting,
  onCreate,
  onTest,
  onUpdate,
}: SshTargetFormProps) {
  const { t } = useTranslation();

  return (
    <div className={TARGET_FORM_GRID_CLASS}>
      <SshAuthMethodButtons
        id="new-ssh-auth-method"
        value={form.authMethod}
        onChange={(method) => onUpdate("authMethod", method)}
      />
      <TargetTextField
        id="new-ssh-label"
        label={t("targets.labelPlaceholder")}
        value={form.label}
        onChange={(value) => onUpdate("label", value)}
        placeholder={t("targets.labelPlaceholder")}
      />
      <TargetTextField
        id="new-ssh-host"
        label={t("targets.hostPlaceholder")}
        value={form.host}
        onChange={(value) => onUpdate("host", value)}
        placeholder={t("targets.hostPlaceholder")}
      />
      <TargetTextField
        id="new-ssh-username"
        label={t("targets.usernamePlaceholder")}
        value={form.username}
        onChange={(value) => onUpdate("username", value)}
        placeholder={t("targets.usernamePlaceholder")}
      />
      <TargetTextField
        id="new-ssh-port"
        label={t("targets.portLabel")}
        value={form.port}
        onChange={(value) => onUpdate("port", value)}
        placeholder="22"
      />
      <div className="space-y-1 md:col-span-2 xl:col-span-4">
        <label
          htmlFor="new-ssh-credential"
          className="text-xs text-muted-foreground"
        >
          {form.authMethod === "key"
            ? t("targets.keyPathPlaceholder")
            : t("targets.passwordPlaceholder")}
        </label>
        <Input
          id="new-ssh-credential"
          type={form.authMethod === "password" ? "password" : "text"}
          value={form.authMethod === "key" ? form.keyPath : form.password}
          onChange={(event) =>
            onUpdate(
              form.authMethod === "key" ? "keyPath" : "password",
              event.target.value,
            )
          }
          placeholder={
            form.authMethod === "key"
              ? t("targets.keyPathPlaceholder")
              : t("targets.passwordPlaceholder")
          }
        />
      </div>
      <div className="flex flex-col gap-2 sm:flex-row md:col-span-2 md:items-end">
        <Button
          type="button"
          variant="outline"
          className="flex-1"
          disabled={isTesting || isCreatingTarget}
          onClick={onTest}
        >
          {isTesting ? <Loader2 className="size-3.5 animate-spin" /> : null}
          {t("targets.test")}
        </Button>
        <Button
          type="button"
          className="flex-1"
          disabled={isCreatingTarget || isTesting}
          onClick={onCreate}
        >
          {isCreatingTarget ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <Plus className="size-3.5" />
          )}
          {t("targets.addSsh")}
        </Button>
      </div>
    </div>
  );
}

interface WslTargetFormProps {
  form: WslTargetFormState;
  isCreatingTarget: boolean;
  isLoadingDistributions: boolean;
  isTesting: boolean;
  distributionError: string | null;
  distributions: WslDistributionOption[];
  existingDistributions: Set<string>;
  onRefreshDistributions: () => void;
  onCreate: () => void;
  onTest: () => void;
  onUpdate: (field: keyof WslTargetFormState, value: string) => void;
}

export function WslTargetForm({
  form,
  isCreatingTarget,
  isLoadingDistributions,
  isTesting,
  distributionError,
  distributions,
  existingDistributions,
  onRefreshDistributions,
  onCreate,
  onTest,
  onUpdate,
}: WslTargetFormProps) {
  const { t } = useTranslation();
  const selectedDistribution = distributions.find(
    (distribution) => distribution.name === form.distribution,
  );
  const selectedKey = form.distribution.trim().toLowerCase();
  const isAlreadyAdded = Boolean(
    selectedKey && existingDistributions.has(selectedKey),
  );
  const hasSelectedDistribution = Boolean(form.distribution.trim());

  return (
    <div className={WSL_FORM_GRID_CLASS}>
      <TargetTextField
        id="new-wsl-label"
        label={t("targets.wslAliasLabel")}
        value={form.label}
        onChange={(value) => onUpdate("label", value)}
        placeholder={t("targets.wslLabelPlaceholder")}
      />
      <div className="space-y-1">
        <label
          htmlFor="new-wsl-distribution"
          className="text-xs text-muted-foreground"
        >
          {t("targets.wslDistributionLabel")}
        </label>
        <select
          id="new-wsl-distribution"
          className="h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
          value={form.distribution}
          disabled={isLoadingDistributions || distributions.length === 0}
          onChange={(event) => onUpdate("distribution", event.target.value)}
        >
          <option value="">
            {isLoadingDistributions
              ? t("targets.wslListLoading")
              : t("targets.wslDistributionSelectPlaceholder")}
          </option>
          {distributions.map((distribution) => {
            const alreadyAdded = existingDistributions.has(
              distribution.name.toLowerCase(),
            );
            return (
              <option
                key={distribution.name}
                value={distribution.name}
                disabled={alreadyAdded}
              >
                {distribution.name}
                {distribution.isDefault
                  ? ` · ${t("targets.wslDefaultDistribution")}`
                  : ""}
                {alreadyAdded ? ` · ${t("targets.wslAlreadyAdded")}` : ""}
              </option>
            );
          })}
        </select>
      </div>
      <div className="space-y-2 md:col-span-2">
        <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={isLoadingDistributions}
            onClick={onRefreshDistributions}
          >
            {isLoadingDistributions ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : null}
            {t("targets.wslListRefresh")}
          </Button>
          {selectedDistribution ? (
            <>
              {selectedDistribution.state ? (
                <span>
                  {t("targets.wslState")}: {selectedDistribution.state}
                </span>
              ) : null}
              {selectedDistribution.version ? (
                <span>
                  {t("targets.wslVersion")}: {selectedDistribution.version}
                </span>
              ) : null}
              {selectedDistribution.isDefault ? (
                <span>{t("targets.wslDefaultDistribution")}</span>
              ) : null}
              {isAlreadyAdded ? (
                <span className="text-primary">
                  {t("targets.wslAlreadyAdded")}
                </span>
              ) : null}
            </>
          ) : null}
        </div>
        {distributionError ? (
          <p className="text-xs text-destructive">{distributionError}</p>
        ) : distributions.length === 0 && !isLoadingDistributions ? (
          <p className="text-xs text-muted-foreground">
            {t("targets.wslListEmpty")}
          </p>
        ) : null}
      </div>
      <div className="flex flex-col gap-2 sm:flex-row md:col-span-2 md:items-end">
        <Button
          type="button"
          variant="outline"
          className="flex-1"
          disabled={
            isTesting ||
            isCreatingTarget ||
            isLoadingDistributions ||
            !hasSelectedDistribution ||
            isAlreadyAdded
          }
          onClick={onTest}
        >
          {isTesting ? <Loader2 className="size-3.5 animate-spin" /> : null}
          {t("targets.test")}
        </Button>
        <Button
          type="button"
          className="flex-1"
          disabled={
            isCreatingTarget ||
            isTesting ||
            isLoadingDistributions ||
            !hasSelectedDistribution ||
            isAlreadyAdded
          }
          onClick={onCreate}
        >
          {isCreatingTarget ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <Plus className="size-3.5" />
          )}
          {t("targets.addWsl")}
        </Button>
      </div>
    </div>
  );
}
