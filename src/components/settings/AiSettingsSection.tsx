import { Bot, ChevronDown, ChevronRight, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { AI_PROVIDERS, REGION_LABELS, type RegionId } from "@/data/aiProviders";
import type {
  AiConnectionTestResult,
  AiSaveStatus,
  AiSettings,
} from "@/stores/settingsStore";

interface AiSettingsSectionProps {
  aiSaveError: string | null;
  aiSaveStatus: AiSaveStatus;
  aiSettings: AiSettings;
  aiTestResult: AiConnectionTestResult | null;
  aiTesting: boolean;
  isLoadingAiSettings: boolean;
  lang: string;
  resolvedUrl: string;
  showAiTestDetails: boolean;
  onProviderChange: (id: string) => void;
  onSetShowAiTestDetails: (value: boolean | ((current: boolean) => boolean)) => void;
  onTestConnection: () => void;
  onUpdateAiSettings: (patch: Partial<AiSettings>) => void;
}

export function AiSettingsSection({
  aiSaveError,
  aiSaveStatus,
  aiSettings,
  aiTestResult,
  aiTesting,
  isLoadingAiSettings,
  lang,
  resolvedUrl,
  showAiTestDetails,
  onProviderChange,
  onSetShowAiTestDetails,
  onTestConnection,
  onUpdateAiSettings,
}: AiSettingsSectionProps) {
  const { t } = useTranslation();
  const currentProvider = AI_PROVIDERS.find((provider) => provider.id === aiSettings.provider);

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <Bot className="size-5 text-muted-foreground" />
          <div>
            <CardTitle>{t("settings.aiProviderTitle")}</CardTitle>
            <CardDescription className="mt-1">
              {t("settings.aiProviderDesc")}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent>
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
                  aria-pressed={aiSettings.provider === provider.id}
                  className={`min-h-8 px-3 py-1.5 rounded-md text-xs transition-colors cursor-pointer border md:min-h-7 ${aiSettings.provider === provider.id ? "bg-primary/15 border-primary text-foreground font-medium" : "border-border bg-background text-muted-foreground hover:border-primary/40 hover:bg-hover-bg/10"}`}
                >
                  {lang === "zh" ? provider.name.zh : provider.name.en}
                </button>
              ))}
            </div>
          </div>
          {currentProvider && currentProvider.regions.length > 1 && (
            <AiRegionPicker
              lang={lang}
              region={aiSettings.region}
              regions={currentProvider.regions}
              onChange={(region) => onUpdateAiSettings({ region })}
            />
          )}
          <div>
            <label htmlFor="settings-ai-api-key" className="text-xs text-muted-foreground mb-1 block">
              {t("settings.aiApiKeyLabel")}
            </label>
            <Input
              id="settings-ai-api-key"
              type="password"
              placeholder="sk-..."
              value={aiSettings.apiKey}
              onChange={(event) => onUpdateAiSettings({ apiKey: event.target.value })}
            />
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
              onChange={(event) => onUpdateAiSettings({ model: event.target.value })}
            />
          </div>
          {aiSettings.provider === "custom" && (
            <div>
              <label htmlFor="settings-ai-api-url" className="text-xs text-muted-foreground mb-1 block">
                {t("settings.aiApiUrlLabel")}
              </label>
              <Input
                id="settings-ai-api-url"
                placeholder={t("settings.aiApiUrlPlaceholder")}
                value={aiSettings.customUrl}
                onChange={(event) => onUpdateAiSettings({ customUrl: event.target.value })}
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
              disabled={aiTesting || !aiSettings.apiKey || !resolvedUrl}
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
              showAiTestDetails={showAiTestDetails}
              onSetShowAiTestDetails={onSetShowAiTestDetails}
            />
          )}
        </div>
      </CardContent>
    </Card>
  );
}

interface AiRegionPickerProps {
  lang: string;
  region: RegionId;
  regions: RegionId[];
  onChange: (region: RegionId) => void;
}

function AiRegionPicker({ lang, region, regions, onChange }: AiRegionPickerProps) {
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
            aria-pressed={region === item}
            className={`min-h-8 px-3 py-1.5 rounded-md text-xs transition-colors cursor-pointer border md:min-h-7 ${region === item ? "bg-primary/15 border-primary text-foreground font-medium" : "border-border bg-background text-muted-foreground hover:border-primary/40"}`}
          >
            {lang === "zh" ? REGION_LABELS[item].zh : REGION_LABELS[item].en}
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
  showAiTestDetails: boolean;
  onSetShowAiTestDetails: (value: boolean | ((current: boolean) => boolean)) => void;
}

function AiTestResultPanel({
  aiRegion,
  aiTestResult,
  currentProviderRegionCount,
  lang,
  showAiTestDetails,
  onSetShowAiTestDetails,
}: AiTestResultPanelProps) {
  const { t } = useTranslation();

  return (
    <div className={`text-xs rounded-md px-3 py-2 space-y-1.5 ${aiTestResult.ok ? "bg-green-500/10 text-green-700 dark:text-green-400" : "bg-destructive/10 text-destructive"}`}>
      <p>{aiTestResult.ok ? "✓ " : "✕ "}{aiTestResult.msg}</p>
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
