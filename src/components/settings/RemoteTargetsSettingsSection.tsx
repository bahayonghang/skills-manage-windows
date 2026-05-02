import { Loader2, Pencil, Plus, Server, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { Input } from "@/components/ui/input";
import type { SshAuthMethod, TargetSummary } from "@/types";

export type SshTargetFormState = {
  label: string;
  host: string;
  username: string;
  port: string;
  authMethod: SshAuthMethod;
  keyPath: string;
  password: string;
};

type TargetMessage = { type: "success" | "error"; text: string } | null;

interface RemoteTargetsSettingsSectionProps {
  activeTarget: TargetSummary;
  deletingTargetId: string | null;
  editingTargetId: string | null;
  isCreatingTarget: boolean;
  isLoadingTargets: boolean;
  switchingTargetId: string | null;
  sshTargetEditForm: SshTargetFormState;
  sshTargetForm: SshTargetFormState;
  sshTargetPasswordUpdates: Record<string, string>;
  targetMessage: TargetMessage;
  targets: TargetSummary[];
  testingTargetId: string | null;
  updatingPasswordTargetId: string | null;
  updatingTargetId: string | null;
  onCancelEditTarget: () => void;
  onCreateSshTarget: () => void;
  onDeleteTarget: (targetId: string) => void;
  onStartEditTarget: (target: TargetSummary) => void;
  onSwitchTarget: (targetId: string) => void;
  onTestExistingTarget: (targetId: string) => void;
  onTestNewSshTarget: () => void;
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
}

function sshCredentialStatus(target: TargetSummary) {
  if (target.authMethod !== "password") return null;
  return target.credentialStatus ?? (target.hasStoredPassword ? "stored" : "missing");
}

function sshCredentialStatusClass(status: string | null) {
  switch (status) {
    case "stored":
      return "border-primary/30 bg-primary/10 text-primary";
    case "session":
      return "border-amber-500/40 bg-amber-500/10 text-amber-600";
    case "unreadable":
      return "border-destructive/40 bg-destructive/10 text-destructive";
    default:
      return "border-muted-foreground/30 bg-muted text-muted-foreground";
  }
}

export function RemoteTargetsSettingsSection({
  activeTarget,
  deletingTargetId,
  editingTargetId,
  isCreatingTarget,
  isLoadingTargets,
  switchingTargetId,
  sshTargetEditForm,
  sshTargetForm,
  sshTargetPasswordUpdates,
  targetMessage,
  targets,
  testingTargetId,
  updatingPasswordTargetId,
  updatingTargetId,
  onCancelEditTarget,
  onCreateSshTarget,
  onDeleteTarget,
  onStartEditTarget,
  onSwitchTarget,
  onTestExistingTarget,
  onTestNewSshTarget,
  onUpdateExistingTargetPassword,
  onUpdateSshTarget,
  onUpdateSshTargetEditForm,
  onUpdateSshTargetForm,
  onUpdateTargetPassword,
}: RemoteTargetsSettingsSectionProps) {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <Server className="size-5 text-muted-foreground" />
          <div>
            <CardTitle>{t("targets.title")}</CardTitle>
            <CardDescription className="mt-1">
              {t("targets.description")}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {targetMessage && (
            <p
              className={`text-xs ${targetMessage.type === "success" ? "text-primary" : "text-destructive"}`}
              role="status"
            >
              {targetMessage.text}
            </p>
          )}

          <div className="rounded-lg border border-border overflow-hidden">
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
                onCancelEditTarget={onCancelEditTarget}
                onDeleteTarget={onDeleteTarget}
                onStartEditTarget={onStartEditTarget}
                onSwitchTarget={onSwitchTarget}
                onTestExistingTarget={onTestExistingTarget}
                onUpdateExistingTargetPassword={onUpdateExistingTargetPassword}
                onUpdateSshTarget={onUpdateSshTarget}
                onUpdateSshTargetEditForm={onUpdateSshTargetEditForm}
                onUpdateTargetPassword={onUpdateTargetPassword}
              />
            ))}
          </div>

          <SshTargetForm
            form={sshTargetForm}
            isCreatingTarget={isCreatingTarget}
            isTesting={testingTargetId === "new"}
            onCreate={onCreateSshTarget}
            onTest={onTestNewSshTarget}
            onUpdate={onUpdateSshTargetForm}
          />

          <div className="space-y-1 text-xs text-muted-foreground">
            <p>{t("targets.sshHint")}</p>
            <p>{t("targets.passwordRepairHint")}</p>
            <p>{t("targets.sshKeyPathHelp")}</p>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

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
  onCancelEditTarget: () => void;
  onDeleteTarget: (targetId: string) => void;
  onStartEditTarget: (target: TargetSummary) => void;
  onSwitchTarget: (targetId: string) => void;
  onTestExistingTarget: (targetId: string) => void;
  onUpdateExistingTargetPassword: (targetId: string, value: string) => void;
  onUpdateSshTarget: (target: TargetSummary) => void;
  onUpdateSshTargetEditForm: (
    field: keyof SshTargetFormState,
    value: string
  ) => void;
  onUpdateTargetPassword: (target: TargetSummary) => void;
}

function RemoteTargetRow({
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
  onCancelEditTarget,
  onDeleteTarget,
  onStartEditTarget,
  onSwitchTarget,
  onTestExistingTarget,
  onUpdateExistingTargetPassword,
  onUpdateSshTarget,
  onUpdateSshTargetEditForm,
  onUpdateTargetPassword,
}: RemoteTargetRowProps) {
  const { t } = useTranslation();
  const isLocal = target.kind === "local";
  const isActive = target.id === activeTarget.id || target.isActive;
  const passwordUpdateValue = sshTargetPasswordUpdates[target.id] ?? "";
  const credentialStatus = sshCredentialStatus(target);
  const isEditing = editingTargetId === target.id;

  return (
    <div className="border-b border-border/50 last:border-0">
      <div className="flex items-center gap-3 px-4 py-3">
        <Server className="size-4 shrink-0 text-muted-foreground" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-medium">
              {isLocal ? t("targets.local") : target.label}
            </span>
            {isActive && (
              <span className="rounded-full bg-primary/10 px-2 py-0.5 text-[11px] text-primary">
                {t("targets.active")}
              </span>
            )}
          </div>
          <div className="mt-0.5 truncate text-xs text-muted-foreground">
            {isLocal
              ? t("targets.localDescription")
              : `${target.authMethod === "password" ? t("targets.authPassword") : t("targets.authKey")} - ${target.username ?? ""}@${target.host ?? ""}:${target.port ?? 22} - ${target.remoteOs ?? "unknown"} - ${target.remoteHome ?? ""}`}
          </div>
          {!isLocal && target.cacheDbPath && (
            <div className="mt-0.5 truncate font-mono text-[11px] text-muted-foreground/80">
              {target.cacheDbPath}
            </div>
          )}
          {!isLocal && target.authMethod === "password" && (
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
              onChange={(value) => onUpdateExistingTargetPassword(target.id, value)}
              onUpdate={() => onUpdateTargetPassword(target)}
            />
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2">
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
              onClick={() => onTestExistingTarget(target.id)}
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
            />
          )}
        </div>
      </div>
      {!isLocal && isEditing && (
        <div className="px-4 pb-4 pl-11">
          <SshTargetEditForm
            form={sshTargetEditForm}
            target={target}
            updatingTargetId={updatingTargetId}
            onCancel={onCancelEditTarget}
            onSave={() => onUpdateSshTarget(target)}
            onUpdate={onUpdateSshTargetEditForm}
          />
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

  return (
    <div className="mt-2 max-w-xl space-y-2">
      <div className="flex flex-wrap items-center gap-2">
        <span
          className={`rounded-full border px-2 py-0.5 text-[11px] ${sshCredentialStatusClass(credentialStatus)}`}
        >
          {t(`targets.credentialStatus.${credentialStatus ?? "missing"}`)}
        </span>
        {credentialError ? (
          <span className="text-[11px] text-muted-foreground">
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
          className="h-8 text-xs"
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
    <div className="grid gap-3 rounded-lg border border-border/70 bg-muted/20 p-3 md:grid-cols-5">
      <SshAuthMethodButtons
        id={`edit-ssh-auth-method-${target.id}`}
        value={form.authMethod}
        onChange={(method) => onUpdate("authMethod", method)}
      />
      <SshTextField
        id={`edit-ssh-label-${target.id}`}
        label={t("targets.labelPlaceholder")}
        value={form.label}
        onChange={(value) => onUpdate("label", value)}
        placeholder={t("targets.labelPlaceholder")}
      />
      <SshTextField
        id={`edit-ssh-host-${target.id}`}
        label={t("targets.hostPlaceholder")}
        value={form.host}
        onChange={(value) => onUpdate("host", value)}
        placeholder={t("targets.hostPlaceholder")}
      />
      <SshTextField
        id={`edit-ssh-username-${target.id}`}
        label={t("targets.usernamePlaceholder")}
        value={form.username}
        onChange={(value) => onUpdate("username", value)}
        placeholder={t("targets.usernamePlaceholder")}
      />
      <SshTextField
        id={`edit-ssh-port-${target.id}`}
        label={t("targets.portLabel")}
        value={form.port}
        onChange={(value) => onUpdate("port", value)}
        placeholder="22"
      />
      <div className="space-y-1 md:col-span-3">
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
            onUpdate(form.authMethod === "key" ? "keyPath" : "password", event.target.value)
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
      <div className="flex gap-2 md:col-span-2 md:items-end">
        <Button
          type="button"
          variant="outline"
          className="flex-1"
          disabled={updatingTargetId === target.id}
          onClick={onCancel}
        >
          {t("common.cancel")}
        </Button>
        <Button
          type="button"
          className="flex-1"
          disabled={updatingTargetId === target.id}
          onClick={onSave}
        >
          {updatingTargetId === target.id ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : null}
          {t("targets.saveChanges")}
        </Button>
      </div>
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

function SshTargetForm({
  form,
  isCreatingTarget,
  isTesting,
  onCreate,
  onTest,
  onUpdate,
}: SshTargetFormProps) {
  const { t } = useTranslation();

  return (
    <div className="grid gap-3 rounded-lg border border-border/70 bg-muted/20 p-4 md:grid-cols-5">
      <SshAuthMethodButtons
        id="new-ssh-auth-method"
        value={form.authMethod}
        onChange={(method) => onUpdate("authMethod", method)}
      />
      <SshTextField
        id="new-ssh-label"
        label={t("targets.labelPlaceholder")}
        value={form.label}
        onChange={(value) => onUpdate("label", value)}
        placeholder={t("targets.labelPlaceholder")}
      />
      <SshTextField
        id="new-ssh-host"
        label={t("targets.hostPlaceholder")}
        value={form.host}
        onChange={(value) => onUpdate("host", value)}
        placeholder={t("targets.hostPlaceholder")}
      />
      <SshTextField
        id="new-ssh-username"
        label={t("targets.usernamePlaceholder")}
        value={form.username}
        onChange={(value) => onUpdate("username", value)}
        placeholder={t("targets.usernamePlaceholder")}
      />
      <SshTextField
        id="new-ssh-port"
        label={t("targets.portLabel")}
        value={form.port}
        onChange={(value) => onUpdate("port", value)}
        placeholder="22"
      />
      <div className="space-y-1 md:col-span-3">
        <label htmlFor="new-ssh-credential" className="text-xs text-muted-foreground">
          {form.authMethod === "key"
            ? t("targets.keyPathPlaceholder")
            : t("targets.passwordPlaceholder")}
        </label>
        <Input
          id="new-ssh-credential"
          type={form.authMethod === "password" ? "password" : "text"}
          value={form.authMethod === "key" ? form.keyPath : form.password}
          onChange={(event) =>
            onUpdate(form.authMethod === "key" ? "keyPath" : "password", event.target.value)
          }
          placeholder={
            form.authMethod === "key"
              ? t("targets.keyPathPlaceholder")
              : t("targets.passwordPlaceholder")
          }
        />
      </div>
      <div className="flex gap-2 md:col-span-2 md:items-end">
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
          disabled={isCreatingTarget}
          onClick={onCreate}
        >
          {isCreatingTarget ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <Plus className="size-3.5" />
          )}
          {t("targets.add")}
        </Button>
      </div>
    </div>
  );
}

interface SshAuthMethodButtonsProps {
  id: string;
  value: SshAuthMethod;
  onChange: (method: SshAuthMethod) => void;
}

function SshAuthMethodButtons({ id, value, onChange }: SshAuthMethodButtonsProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-1 md:col-span-5">
      <div id={id} className="text-xs text-muted-foreground">
        {t("targets.authMethodLabel")}
      </div>
      <div className="flex gap-2" role="group" aria-labelledby={id}>
        {(["key", "password"] as const).map((method) => (
          <Button
            key={method}
            type="button"
            variant={value === method ? "default" : "outline"}
            className="flex-1"
            onClick={() => onChange(method)}
            aria-pressed={value === method}
          >
            {method === "key" ? t("targets.authKey") : t("targets.authPassword")}
          </Button>
        ))}
      </div>
    </div>
  );
}

interface SshTextFieldProps {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
}

function SshTextField({
  id,
  label,
  value,
  onChange,
  placeholder,
}: SshTextFieldProps) {
  return (
    <div className="space-y-1">
      <label htmlFor={id} className="text-xs text-muted-foreground">
        {label}
      </label>
      <Input
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
      />
    </div>
  );
}
