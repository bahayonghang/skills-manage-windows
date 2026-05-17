import { Bot, ChevronDown, ChevronRight, Eye, EyeOff, Loader2, LockKeyhole } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { SettingsCollapsibleCard } from "@/components/settings/SettingsCollapsibleCard";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  AI_PROVIDERS,
  API_PROTOCOLS,
  REGION_LABEL_KEYS,
  REGION_LABELS,
  type ApiProtocol,
  type RegionId,
} from "@/data/aiProviders";
import { formatBackendError } from "@/lib/backendError";
import type { TFunction } from "i18next";
import type {
  AiConnectionTestResult,
  AiSaveStatus,
  AiSettings,
} from "@/stores/settingsStore";
import type { AiApiKeyState, SecretStorageState } from "@/types";

interface AiSettingsSectionProps {
  aiSaveError: string | null;
  aiSaveStatus: AiSaveStatus;
  aiApiKeyState: AiApiKeyState;
  aiSettings: AiSettings;
  aiTestResult: AiConnectionTestResult | null;
  aiTesting: boolean;
  isLoadingAiSettings: boolean;
  lang: string;
  resolvedUrl: string;
  showAiTestDetails: boolean;
  onProviderChange: (id: string) => void;
  onClearApiKey: () => void;
  onSetShowAiTestDetails: (value: boolean | ((current: boolean) => boolean)) => void;
  onTestConnection: () => void;
  onUpdateAiSettings: (patch: Partial<AiSettings>) => void;
}

export function AiSettingsSection({
  aiSaveError,
  aiSaveStatus,
  aiApiKeyState,
  aiSettings,
  aiTestResult,
  aiTesting,
  isLoadingAiSettings,
  lang,
  resolvedUrl,
  showAiTestDetails,
  onProviderChange,
  onClearApiKey,
  onSetShowAiTestDetails,
  onTestConnection,
  onUpdateAiSettings,
}: AiSettingsSectionProps) {
  const { t } = useTranslation();
  const [showApiKey, setShowApiKey] = useState(false);
  const currentProvider = AI_PROVIDERS.find((provider) => provider.id === aiSettings.provider);
  const currentProviderLabel = currentProvider ? t(currentProvider.labelKey) : aiSettings.provider;
  const aiControlsDisabled = isLoadingAiSettings || aiSaveStatus === "saving";
  const hasNewApiKeyInput = aiSettings.apiKey.trim().length > 0;
  const canRevealNewApiKeyInput = !aiControlsDisabled && hasNewApiKeyInput;

  return (
    <SettingsCollapsibleCard
      sectionId="ai-provider"
      title={t("settings.aiProviderTitle")}
      description={t("settings.aiProviderDesc")}
      icon={<Bot className="size-5 shrink-0 text-muted-foreground" />}
    >
        <div className="space-y-4">
          <div>
            <div id="settings-ai-provider-label" className="text-xs text-muted-foreground mb-2">
              {t("settings.aiProviderLabel")}
            </div>
            <div className="flex flex-wrap gap-1.5" role="group" aria-labelledby="settings-ai-provider-label">
              {AI_PROVIDERS.map((provider) => (
                <button
                  key={provider.id}
                  type="button"
                  onClick={() => onProviderChange(provider.id)}
                  disabled={aiControlsDisabled}
                  aria-pressed={aiSettings.provider === provider.id}
                  className={`min-h-8 px-3 py-1.5 rounded-md text-xs transition-colors cursor-pointer border md:min-h-7 ${aiSettings.provider === provider.id ? "bg-primary/15 border-primary text-foreground font-medium" : "border-border bg-background text-muted-foreground hover:border-primary/40 hover:bg-hover-bg/10"}`}
                >
                  {t(provider.labelKey)}
                </button>
              ))}
            </div>
          </div>
          {currentProvider && currentProvider.regions.length > 1 && (
            <AiRegionPicker
              lang={lang}
              region={aiSettings.region}
              regions={currentProvider.regions}
              disabled={aiControlsDisabled}
              onChange={(region) => onUpdateAiSettings({ region })}
            />
          )}
          <div>
            <label htmlFor="settings-ai-api-key" className="text-xs text-muted-foreground mb-1 block">
              {t("settings.aiApiKeyLabel")}
            </label>
            <div className="mb-2 rounded-lg border border-border/70 bg-muted/20 px-3 py-2">
              <div className="text-xs font-medium text-foreground">
                {t("settings.aiApiKeyActiveState", {
                  provider: currentProviderLabel,
                  state: getAiApiKeyActiveStateLabel(t, aiApiKeyState),
                })}
              </div>
              <p className="mt-1 text-xs text-muted-foreground">
                {t("settings.aiApiKeyUsageHint")}
              </p>
              {aiApiKeyState.configured && aiApiKeyState.fingerprint ? (
                <p className="mt-1 text-xs text-muted-foreground">
                  {t("settings.aiApiKeyFingerprint", {
                    fingerprint: aiApiKeyState.fingerprint,
                  })}
                </p>
              ) : null}
            </div>
            <div className="relative">
              <Input
                id="settings-ai-api-key"
                type={showApiKey ? "text" : "password"}
                placeholder={
                  aiApiKeyState.configured
                    ? t("settings.aiApiKeyReplacePlaceholder")
                    : t("settings.aiApiKeyPlaceholder")
                }
                value={aiSettings.apiKey}
                className="pr-10"
                disabled={aiControlsDisabled}
                onChange={(event) => onUpdateAiSettings({ apiKey: event.target.value })}
              />
              <button
                type="button"
                className="absolute inset-y-0 right-0 flex w-9 items-center justify-center text-muted-foreground hover:text-foreground disabled:opacity-50"
                disabled={!canRevealNewApiKeyInput}
                aria-label={
                  hasNewApiKeyInput
                    ? showApiKey
                      ? t("settings.aiApiKeyHide")
                      : t("settings.aiApiKeyShow")
                    : t("settings.aiApiKeySavedHidden")
                }
                onClick={() => setShowApiKey((value) => !value)}
              >
                {!hasNewApiKeyInput && aiApiKeyState.configured ? (
                  <LockKeyhole className="size-4" />
                ) : showApiKey ? (
                  <EyeOff className="size-4" />
                ) : (
                  <Eye className="size-4" />
                )}
              </button>
            </div>
            {hasNewApiKeyInput ? (
              <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
                {t("settings.aiApiKeyWillReplace", { provider: currentProviderLabel })}
              </p>
            ) : null}
            {aiApiKeyState.configured && !aiSettings.apiKey ? (
              <p className="mt-1 text-xs text-muted-foreground">
                {t("settings.aiApiKeyConfiguredNoReveal")}
              </p>
            ) : null}
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <span
                className={`inline-flex items-center rounded-full px-2 py-0.5 text-[11px] ${secretStorageTone(aiApiKeyState.storageState)}`}
              >
                {aiApiKeyState.configured || aiApiKeyState.storageState === "unreadable"
                  ? t(`settings.aiApiKeyStorageState.${aiApiKeyState.storageState}`)
                  : t("settings.aiApiKeyNotConfigured")}
              </span>
              <Button
                variant="outline"
                size="sm"
                disabled={aiControlsDisabled || !aiApiKeyState.configured}
                onClick={onClearApiKey}
              >
                {t("settings.aiApiKeyClear")}
              </Button>
            </div>
            {aiApiKeyState.error ? (
              <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
                {t("settings.aiApiKeyMigrationWarning")}
              </p>
            ) : null}
          </div>
          <AiSaveStatusRow
            aiSaveError={aiSaveError}
            aiSaveStatus={aiSaveStatus}
            isLoadingAiSettings={isLoadingAiSettings}
          />
          <div>
            <label htmlFor="settings-ai-model" className="text-xs text-muted-foreground mb-1 block">
              {t("settings.aiModelLabel")}
            </label>
            <Input
              id="settings-ai-model"
              placeholder={t("settings.aiModelPlaceholder")}
              value={aiSettings.model}
              disabled={aiControlsDisabled}
              onChange={(event) => onUpdateAiSettings({ model: event.target.value })}
            />
          </div>
          {aiSettings.provider === "custom" && (
            <div className="space-y-3">
              <div>
                <label htmlFor="settings-ai-api-url" className="text-xs text-muted-foreground mb-1 block">
                  {t("settings.aiApiUrlLabel")}
                </label>
                <Input
                  id="settings-ai-api-url"
                  placeholder={t("settings.aiApiUrlPlaceholder")}
                  value={aiSettings.customUrl}
                  disabled={aiControlsDisabled}
                  onChange={(event) => onUpdateAiSettings({ customUrl: event.target.value })}
                />
              </div>
              <AiProtocolPicker
                protocol={aiSettings.protocol}
                disabled={aiControlsDisabled}
                onChange={(protocol) => onUpdateAiSettings({ protocol })}
              />
            </div>
          )}
          <AiTagRateSettings
            aiSettings={aiSettings}
            onUpdateAiSettings={onUpdateAiSettings}
          />
          <div className="flex items-center gap-3">
            {resolvedUrl && (
              <div className="text-xs text-muted-foreground bg-muted/30 rounded-md px-3 py-2 font-mono truncate flex-1 min-w-0">
                {resolvedUrl}
              </div>
            )}
            <Button
              variant="outline"
              size="sm"
              disabled={aiControlsDisabled || aiTesting || (!aiSettings.apiKey && !aiApiKeyState.configured) || !resolvedUrl}
              onClick={onTestConnection}
              className="shrink-0"
            >
              {aiTesting ? <Loader2 className="size-3.5 animate-spin" /> : <Bot className="size-3.5" />}
              <span>{t("settings.aiTestConnection")}</span>
            </Button>
          </div>
          {aiTestResult && currentProvider && (
            <AiTestResultPanel
              aiRegion={aiSettings.region}
              aiTestResult={aiTestResult}
              currentProviderRegionCount={currentProvider.regions.length}
              lang={lang}
              providerLabel={t(currentProvider.labelKey)}
              showAiTestDetails={showAiTestDetails}
              onSetShowAiTestDetails={onSetShowAiTestDetails}
            />
          )}
        </div>
    </SettingsCollapsibleCard>
  );
}

function secretStorageTone(state: SecretStorageState) {
  switch (state) {
    case "stored":
      return "bg-emerald-500/10 text-emerald-700 dark:text-emerald-300";
    case "session":
      return "bg-amber-500/10 text-amber-700 dark:text-amber-300";
    case "unreadable":
      return "bg-destructive/10 text-destructive";
    case "missing":
    default:
      return "bg-muted text-muted-foreground";
  }
}

function getAiApiKeyActiveStateLabel(
  t: TFunction,
  aiApiKeyState: AiApiKeyState
) {
  if (aiApiKeyState.storageState === "unreadable") {
    return t("settings.aiApiKeyActiveUnavailable");
  }
  return aiApiKeyState.configured
    ? t("settings.aiApiKeyActiveConfigured")
    : t("settings.aiApiKeyActiveMissing");
}

interface AiRegionPickerProps {
  lang: string;
  region: RegionId;
  regions: RegionId[];
  disabled?: boolean;
  onChange: (region: RegionId) => void;
}

function AiRegionPicker({ lang: _lang, region, regions, disabled = false, onChange }: AiRegionPickerProps) {
  const { t } = useTranslation();

  return (
    <div>
      <div id="settings-ai-region-label" className="text-xs text-muted-foreground mb-2">
        {t("settings.aiRegionLabel")}
      </div>
      <div className="flex gap-1.5" role="group" aria-labelledby="settings-ai-region-label">
        {regions.map((item) => (
          <button
            key={item}
            type="button"
            onClick={() => onChange(item)}
            disabled={disabled}
            aria-pressed={region === item}
            className={`min-h-8 px-3 py-1.5 rounded-md text-xs transition-colors cursor-pointer border md:min-h-7 ${region === item ? "bg-primary/15 border-primary text-foreground font-medium" : "border-border bg-background text-muted-foreground hover:border-primary/40"}`}
          >
            {t(REGION_LABEL_KEYS[item])}
          </button>
        ))}
      </div>
    </div>
  );
}

interface AiProtocolPickerProps {
  protocol: ApiProtocol;
  disabled?: boolean;
  onChange: (protocol: ApiProtocol) => void;
}

function AiProtocolPicker({
  protocol,
  disabled = false,
  onChange,
}: AiProtocolPickerProps) {
  const { t } = useTranslation();

  return (
    <div>
      <div id="settings-ai-protocol-label" className="text-xs text-muted-foreground mb-2">
        {t("settings.aiProtocolLabel")}
      </div>
      <div className="grid gap-2 sm:grid-cols-3" role="radiogroup" aria-labelledby="settings-ai-protocol-label">
        {API_PROTOCOLS.map((item) => (
          <button
            key={item.id || "auto"}
            type="button"
            role="radio"
            disabled={disabled}
            aria-checked={protocol === item.id}
            onClick={() => onChange(item.id)}
            className={`rounded-md border px-3 py-2 text-left text-xs transition-colors ${protocol === item.id ? "bg-primary/15 border-primary text-foreground" : "border-border bg-background text-muted-foreground hover:border-primary/40"}`}
          >
            <span className="block font-medium">{t(item.labelKey)}</span>
            <span className="mt-1 block text-[11px] text-muted-foreground">
              {t(item.descriptionKey)}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}

interface AiSaveStatusRowProps {
  aiSaveError: string | null;
  aiSaveStatus: AiSaveStatus;
  isLoadingAiSettings: boolean;
}

function AiSaveStatusRow({
  aiSaveError,
  aiSaveStatus,
  isLoadingAiSettings,
}: AiSaveStatusRowProps) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center gap-2 text-xs" role="status">
      {aiSaveStatus === "saving" || isLoadingAiSettings ? (
        <>
          <Loader2 className="size-3 animate-spin text-muted-foreground" />
          <span className="text-muted-foreground">{t("settings.aiSaveSaving")}</span>
        </>
      ) : aiSaveStatus === "saved" ? (
        <span className="text-emerald-600 dark:text-emerald-400">{t("settings.aiSaveSaved")}</span>
      ) : aiSaveStatus === "error" ? (
        <span className="text-destructive">
          {t("settings.aiSaveError", { error: aiSaveError ?? "" })}
        </span>
      ) : null}
    </div>
  );
}

interface AiTagRateSettingsProps {
  aiSettings: AiSettings;
  onUpdateAiSettings: (patch: Partial<AiSettings>) => void;
}

function AiTagRateSettings({
  aiSettings,
  onUpdateAiSettings,
}: AiTagRateSettingsProps) {
  const { t } = useTranslation();

  return (
    <div className="rounded-lg border border-border/70 bg-muted/20 p-3">
      <div className="mb-3 flex items-start justify-between gap-3">
        <div>
          <div id="settings-ai-tag-rate-title" className="text-sm font-medium">
            {t("settings.aiTagRateTitle")}
          </div>
          <p className="mt-1 text-xs text-muted-foreground">
            {t("settings.aiTagRateDesc")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span id="settings-ai-tag-stop-on-rate-limit-label" className="text-xs text-muted-foreground">
            {t("settings.aiTagStopOn429Label")}
          </span>
          <Switch
            checked={aiSettings.tagStopOnRateLimit}
            onCheckedChange={(checked) => onUpdateAiSettings({ tagStopOnRateLimit: checked })}
            aria-labelledby="settings-ai-tag-stop-on-rate-limit-label"
          />
        </div>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <div>
          <label htmlFor="settings-ai-tag-concurrency" className="text-xs text-muted-foreground mb-1 block">
            {t("settings.aiTagConcurrencyLabel")}
          </label>
          <Input
            id="settings-ai-tag-concurrency"
            type="number"
            min={1}
            max={8}
            value={aiSettings.tagConcurrency}
            onChange={(event) => onUpdateAiSettings({ tagConcurrency: event.target.value })}
          />
        </div>
        <div>
          <label htmlFor="settings-ai-tag-interval" className="text-xs text-muted-foreground mb-1 block">
            {t("settings.aiTagIntervalLabel")}
          </label>
          <Input
            id="settings-ai-tag-interval"
            type="number"
            min={0}
            max={60000}
            step={500}
            value={aiSettings.tagIntervalMs}
            onChange={(event) => onUpdateAiSettings({ tagIntervalMs: event.target.value })}
          />
        </div>
      </div>
    </div>
  );
}

interface AiTestResultPanelProps {
  aiRegion: RegionId;
  aiTestResult: AiConnectionTestResult;
  currentProviderRegionCount: number;
  lang: string;
  providerLabel: string;
  showAiTestDetails: boolean;
  onSetShowAiTestDetails: (value: boolean | ((current: boolean) => boolean)) => void;
}

function AiTestResultPanel({
  aiRegion,
  aiTestResult,
  currentProviderRegionCount,
  lang,
  providerLabel,
  showAiTestDetails,
  onSetShowAiTestDetails,
}: AiTestResultPanelProps) {
  const { t } = useTranslation();
  const message = aiTestResult.ok
    ? t("settings.aiTestSuccess", { provider: providerLabel })
    : aiTestResult.code
      ? formatBackendError(`${aiTestResult.code}:${aiTestResult.msg}`, t)
      : aiTestResult.msg;

  return (
    <div className={`text-xs rounded-md px-3 py-2 space-y-1.5 ${aiTestResult.ok ? "bg-green-500/10 text-green-700 dark:text-green-400" : "bg-destructive/10 text-destructive"}`}>
      <p>{aiTestResult.ok ? "✓ " : "✕ "}{message}</p>
      {!aiTestResult.ok && aiTestResult.details && (
        <div>
          <button
            type="button"
            className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
            onClick={() => onSetShowAiTestDetails((current) => !current)}
            aria-expanded={showAiTestDetails}
          >
            {showAiTestDetails ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
            {t("settings.aiTestDetails")}
          </button>
          {showAiTestDetails && (
            <pre className="mt-1 text-[11px] leading-4 font-mono text-muted-foreground whitespace-pre-wrap break-all bg-muted/30 rounded-md p-2 max-h-32 overflow-auto">
              {aiTestResult.details}
            </pre>
          )}
        </div>
      )}
      {!aiTestResult.ok && currentProviderRegionCount > 1 && (
        <p className="text-muted-foreground">
          {t("settings.aiTestRegionHint", {
            region:
              lang === "zh"
                ? REGION_LABELS[aiRegion]?.zh ?? aiRegion
                : REGION_LABELS[aiRegion]?.en ?? aiRegion,
          })}
        </p>
      )}
    </div>
  );
}
