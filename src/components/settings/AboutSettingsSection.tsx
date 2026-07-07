import {
  Activity,
  CheckCircle2,
  CircleDot,
  Code2,
  Cpu,
  Database,
  Download,
  ExternalLink,
  FileText,
  Gauge,
  Globe,
  Info,
  Loader2,
  PackageCheck,
  RotateCcw,
  Server,
  ShieldCheck,
  Sparkles,
  TerminalSquare,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  formatAppUpdateBytes,
  getAppUpdateProgressPercent,
  useAppUpdateStore,
  type AppUpdateStatus,
} from "@/stores/appUpdateStore";
import { cn } from "@/lib/utils";
import { statusChipClass } from "@/lib/statusTone";

interface AboutSettingsSectionProps {
  appVersion: string;
  dbPathDisplay: string;
  repoUrl: string;
}

const HELP_RESOURCES = [
  {
    key: "readme",
    href: "https://github.com/bahayonghang/skills-manage-windows#readme",
  },
  {
    key: "readmeZh",
    href: "https://github.com/bahayonghang/skills-manage-windows/blob/main/README_CN.md",
  },
  {
    key: "releases",
    href: "https://github.com/bahayonghang/skills-manage-windows/releases",
  },
  {
    key: "issues",
    href: "https://github.com/bahayonghang/skills-manage-windows/issues",
  },
] as const;

const STACK_ITEMS = [
  "Rust",
  "Tauri 2",
  "React 19",
  "TypeScript 6",
  "Tailwind CSS 4",
  "SQLite/sqlx",
  "Zustand",
  "i18next",
];

export function AboutSettingsSection({
  appVersion,
  dbPathDisplay,
  repoUrl,
}: AboutSettingsSectionProps) {
  const { t } = useTranslation();
  const status = useAppUpdateStore((state) => state.status);
  const currentVersion = useAppUpdateStore((state) => state.currentVersion);
  const latestVersion = useAppUpdateStore((state) => state.latestVersion);
  const releaseNotes = useAppUpdateStore((state) => state.releaseNotes);
  const progress = useAppUpdateStore((state) => state.progress);
  const error = useAppUpdateStore((state) => state.error);
  const checkForUpdate = useAppUpdateStore((state) => state.checkForUpdate);
  const installUpdate = useAppUpdateStore((state) => state.installUpdate);
  const progressPercent = getAppUpdateProgressPercent(progress);
  const statusLabel = t(`settings.aboutCockpit.updateStatus.${status}`);
  const isChecking = status === "checking";
  const isDownloading = status === "downloading";
  const isInstalling = status === "installing";
  const canInstall = status === "available";
  const hasUpdate = Boolean(latestVersion);

  return (
    <div className="relative overflow-hidden rounded-2xl bg-card p-4 text-foreground shadow-sm ring-1 ring-border sm:p-6">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 opacity-50 [background-image:linear-gradient(color-mix(in_srgb,var(--foreground)_6%,transparent)_1px,transparent_1px),linear-gradient(90deg,color-mix(in_srgb,var(--foreground)_6%,transparent)_1px,transparent_1px)] [background-size:24px_24px]"
      />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute -right-24 -top-24 size-64 rounded-full bg-primary/15 blur-3xl"
      />
      <div className="relative space-y-5">
        <section className="rounded-xl bg-muted/40 p-5 ring-1 ring-border">
          <div className="flex flex-col gap-5 xl:flex-row xl:items-start xl:justify-between">
            <div className="max-w-3xl space-y-4">
              <div className="flex flex-wrap items-center gap-2">
                <span className="inline-flex items-center gap-2 rounded-full border border-primary/40 bg-primary/15 px-3 py-1 text-xs font-medium uppercase tracking-[0.18em] text-primary">
                  <CircleDot className="size-3" aria-hidden="true" />
                  {t("settings.aboutCockpit.eyebrow")}
                </span>
                <StatusChip status={status} label={statusLabel} />
              </div>
              <div>
                <h3 className="font-heading text-3xl font-semibold tracking-tight text-foreground sm:text-4xl">
                  SkillPort
                </h3>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  {t("settings.aboutCockpit.heroDesc")}
                </p>
              </div>
              <div className="flex flex-wrap gap-2 text-xs">
                <MetaPill
                  icon={<PackageCheck />}
                  label={t("settings.appVersion")}
                  value={`v${appVersion}`}
                />
                <MetaPill
                  icon={<ShieldCheck />}
                  label={t("settings.aboutCockpit.channel")}
                  value={t("settings.aboutCockpit.stable")}
                />
                <MetaPill
                  icon={<Server />}
                  label={t("settings.aboutCockpit.platform")}
                  value={t("settings.aboutCockpit.windowsX64")}
                />
              </div>
            </div>

            <div className="grid min-w-0 gap-3 sm:grid-cols-2 xl:w-[28rem]">
              <HeroMetric
                label={t("settings.aboutCockpit.currentVersion")}
                value={`v${currentVersion || appVersion}`}
              />
              <HeroMetric
                label={t("settings.aboutCockpit.latestVersion")}
                value={
                  latestVersion
                    ? `v${latestVersion}`
                    : t("settings.aboutCockpit.pendingCheck")
                }
              />
              <a
                href={repoUrl}
                target="_blank"
                rel="noreferrer"
                className="group col-span-full flex min-w-0 items-center justify-between gap-3 rounded-lg bg-background px-4 py-3 text-sm text-muted-foreground ring-1 ring-border transition hover:bg-primary/10 hover:text-foreground hover:ring-primary/50"
              >
                <span className="flex min-w-0 items-center gap-2">
                  <Globe
                    className="size-4 shrink-0 text-primary"
                    aria-hidden="true"
                  />
                  <span className="truncate">{repoUrl}</span>
                </span>
                <ExternalLink className="size-4 shrink-0 opacity-70 transition group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
              </a>
            </div>
          </div>
        </section>

        <div className="grid gap-5 xl:grid-cols-[minmax(0,1.1fr)_minmax(320px,0.9fr)]">
          <ReleaseUpdateCard
            canInstall={canInstall}
            error={error}
            hasUpdate={hasUpdate}
            isChecking={isChecking}
            isDownloading={isDownloading}
            isInstalling={isInstalling}
            progressPercent={progressPercent}
            releaseNotes={releaseNotes}
            status={status}
            statusLabel={statusLabel}
            downloadedLabel={formatAppUpdateBytes(progress.downloaded)}
            totalLabel={formatAppUpdateBytes(progress.total)}
            latestVersion={latestVersion}
            onCheck={() => {
              void checkForUpdate();
            }}
            onInstall={() => {
              void installUpdate();
            }}
          />

          <DiagnosticCard dbPathDisplay={dbPathDisplay} repoUrl={repoUrl} />
        </div>

        <div className="grid gap-5 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
          <TechStackCard />
          <HelpResourcesCard />
        </div>
      </div>
    </div>
  );
}

function ReleaseUpdateCard({
  canInstall,
  error,
  hasUpdate,
  isChecking,
  isDownloading,
  isInstalling,
  progressPercent,
  releaseNotes,
  status,
  statusLabel,
  downloadedLabel,
  totalLabel,
  latestVersion,
  onCheck,
  onInstall,
}: {
  canInstall: boolean;
  error: string | null;
  hasUpdate: boolean;
  isChecking: boolean;
  isDownloading: boolean;
  isInstalling: boolean;
  progressPercent: number;
  releaseNotes: string | null;
  status: AppUpdateStatus;
  statusLabel: string;
  downloadedLabel: string;
  totalLabel: string;
  latestVersion: string | null;
  onCheck: () => void;
  onInstall: () => void;
}) {
  const { t } = useTranslation();
  const progressLabel = t("settings.aboutCockpit.downloadProgress", {
    downloaded: downloadedLabel,
    total: totalLabel,
    percent: progressPercent,
  });

  return (
    <CockpitCard
      icon={<Download />}
      title={t("settings.aboutCockpit.updateTitle")}
      description={t("settings.aboutCockpit.updateDesc")}
      data-testid="about-update-card"
    >
      <div className="space-y-4">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <div className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
              {t("settings.aboutCockpit.updateState")}
            </div>
            <div className="mt-1 flex flex-wrap items-center gap-2">
              <StatusChip status={status} label={statusLabel} />
              {latestVersion ? (
                <span
                  className={cn(
                    "rounded-full px-2.5 py-1 text-xs",
                    statusChipClass.success,
                  )}
                >
                  {t("settings.aboutCockpit.newVersion", {
                    version: latestVersion,
                  })}
                </span>
              ) : null}
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="outline"
              disabled={isChecking || isDownloading || isInstalling}
              onClick={onCheck}
            >
              {isChecking ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <RotateCcw className="size-4" />
              )}
              {isChecking
                ? t("settings.aboutCockpit.checking")
                : t("settings.aboutCockpit.checkUpdate")}
            </Button>
            <Button
              type="button"
              disabled={!canInstall || isDownloading || isInstalling}
              onClick={onInstall}
            >
              {isDownloading || isInstalling ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Download className="size-4" />
              )}
              {isInstalling
                ? t("settings.aboutCockpit.installing")
                : t("settings.aboutCockpit.downloadInstall")}
            </Button>
          </div>
        </div>

        <div className="rounded-lg bg-background p-4 ring-1 ring-border">
          <div className="mb-2 flex items-center justify-between gap-3 text-xs text-muted-foreground">
            <span>{t("settings.aboutCockpit.artifact")}</span>
            <span>{progressLabel}</span>
          </div>
          <div
            className="h-2 overflow-hidden rounded-full bg-muted"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={progressPercent}
            aria-label={t("settings.aboutCockpit.progressAria")}
          >
            <div
              className="h-full rounded-full bg-primary transition-[width] duration-300"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
        </div>

        {status === "upToDate" ? (
          <InlineState tone="success" icon={<CheckCircle2 />}>
            {t("settings.aboutCockpit.upToDateMessage")}
          </InlineState>
        ) : null}
        {status === "unsupported" ? (
          <InlineState tone="warning" icon={<Info />}>
            {t("settings.aboutCockpit.unsupportedMessage")}
          </InlineState>
        ) : null}
        {error ? (
          <InlineState tone="error" icon={<Info />}>
            {error}
          </InlineState>
        ) : null}
        {hasUpdate && releaseNotes ? (
          <div className="rounded-lg bg-muted/30 p-4 ring-1 ring-border">
            <div className="mb-2 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">
              {t("settings.aboutCockpit.releaseNotes")}
            </div>
            <p className="max-h-28 overflow-auto whitespace-pre-wrap text-sm leading-6 text-muted-foreground">
              {releaseNotes}
            </p>
          </div>
        ) : null}
      </div>
    </CockpitCard>
  );
}

function DiagnosticCard({
  dbPathDisplay,
  repoUrl,
}: {
  dbPathDisplay: string;
  repoUrl: string;
}) {
  const { t } = useTranslation();
  return (
    <CockpitCard
      icon={<Gauge />}
      title={t("settings.aboutCockpit.diagnosticsTitle")}
      description={t("settings.aboutCockpit.diagnosticsDesc")}
    >
      <div className="space-y-3">
        <DiagnosticRow
          icon={<Database />}
          label={t("settings.dbPath")}
          value={dbPathDisplay}
        />
        <DiagnosticRow
          icon={<Globe />}
          label={t("settings.repoUrl")}
          value={repoUrl}
          href={repoUrl}
        />
        <DiagnosticRow
          icon={<Cpu />}
          label={t("settings.aboutCockpit.buildTarget")}
          value={t("settings.aboutCockpit.windowsSupport")}
        />
        <DiagnosticRow
          icon={<ShieldCheck />}
          label={t("settings.aboutCockpit.channel")}
          value={t("settings.aboutCockpit.signingNote")}
        />
      </div>
    </CockpitCard>
  );
}

function TechStackCard() {
  const { t } = useTranslation();
  return (
    <CockpitCard
      icon={<Code2 />}
      title={t("settings.aboutCockpit.stackTitle")}
      description={t("settings.aboutCockpit.stackDesc")}
    >
      <div className="flex flex-wrap gap-2">
        {STACK_ITEMS.map((item) => (
          <span
            key={item}
            className="inline-flex items-center gap-2 rounded-full border border-border bg-muted/40 px-3 py-1.5 text-xs font-medium text-foreground"
          >
            <Sparkles className="size-3 text-primary" aria-hidden="true" />
            {item}
          </span>
        ))}
      </div>
    </CockpitCard>
  );
}

function HelpResourcesCard() {
  const { t } = useTranslation();
  const icons = [FileText, FileText, PackageCheck, Globe];

  return (
    <CockpitCard
      icon={<TerminalSquare />}
      title={t("settings.aboutCockpit.resourcesTitle")}
      description={t("settings.aboutCockpit.resourcesDesc")}
    >
      <div className="grid gap-3 sm:grid-cols-2">
        {HELP_RESOURCES.map((resource, index) => {
          const Icon = icons[index] ?? FileText;
          return (
            <a
              key={resource.key}
              href={resource.href}
              target="_blank"
              rel="noreferrer"
              className="group rounded-lg bg-background p-4 text-sm ring-1 ring-border transition hover:bg-primary/10 hover:ring-primary/50"
            >
              <div className="flex items-center justify-between gap-3">
                <span className="flex items-center gap-2 font-medium text-foreground">
                  <Icon className="size-4 text-primary" aria-hidden="true" />
                  {t(`settings.aboutCockpit.resources.${resource.key}.title`)}
                </span>
                <ExternalLink className="size-4 text-muted-foreground transition group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
              </div>
              <p className="mt-2 text-xs leading-5 text-muted-foreground">
                {t(`settings.aboutCockpit.resources.${resource.key}.desc`)}
              </p>
            </a>
          );
        })}
      </div>
    </CockpitCard>
  );
}

function CockpitCard({
  icon,
  title,
  description,
  children,
  "data-testid": testId,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
  children: React.ReactNode;
  "data-testid"?: string;
}) {
  return (
    <section
      className="rounded-xl bg-muted/40 p-5 ring-1 ring-border"
      data-testid={testId}
    >
      <div className="mb-4 flex items-start gap-3">
        <span className="grid size-10 shrink-0 place-items-center rounded-xl border border-primary/30 bg-primary/15 text-primary">
          {icon}
        </span>
        <div>
          <h4 className="font-heading text-base font-semibold text-foreground">
            {title}
          </h4>
          <p className="mt-1 text-sm leading-5 text-muted-foreground">
            {description}
          </p>
        </div>
      </div>
      {children}
    </section>
  );
}

function HeroMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-background px-4 py-3 ring-1 ring-border">
      <div className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
        {label}
      </div>
      <div className="mt-1 truncate text-sm font-semibold text-foreground">
        {value}
      </div>
    </div>
  );
}

function MetaPill({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <span className="inline-flex items-center gap-2 rounded-full border border-border bg-muted/40 px-3 py-1.5 text-foreground">
      <span
        className="[&_svg]:size-3.5 [&_svg]:text-primary"
        aria-hidden="true"
      >
        {icon}
      </span>
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium text-foreground">{value}</span>
    </span>
  );
}

function DiagnosticRow({
  icon,
  label,
  value,
  href,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  href?: string;
}) {
  const content = href ? (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className="break-all text-primary hover:underline"
    >
      {value}
    </a>
  ) : (
    <span className="break-all">{value}</span>
  );

  return (
    <div className="flex items-start gap-3 rounded-lg bg-background p-3 ring-1 ring-border">
      <span className="mt-0.5 text-primary [&_svg]:size-4" aria-hidden="true">
        {icon}
      </span>
      <div className="min-w-0">
        <div className="text-xs uppercase tracking-[0.16em] text-muted-foreground">
          {label}
        </div>
        <div className="mt-1 font-mono text-sm text-foreground">{content}</div>
      </div>
    </div>
  );
}

function InlineState({
  tone,
  icon,
  children,
}: {
  tone: "success" | "warning" | "error";
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div
      className={cn(
        "flex items-start gap-2 rounded-lg border px-3 py-2 text-sm",
        statusChipClass[tone],
      )}
    >
      <span className="mt-0.5 [&_svg]:size-4" aria-hidden="true">
        {icon}
      </span>
      <span>{children}</span>
    </div>
  );
}

function StatusChip({
  status,
  label,
}: {
  status: AppUpdateStatus;
  label: string;
}) {
  const busy =
    status === "checking" ||
    status === "downloading" ||
    status === "installing";
  return (
    <span
      className={cn(
        "inline-flex items-center gap-2 rounded-full border px-3 py-1 text-xs font-medium",
        status === "available" && statusChipClass.success,
        status === "upToDate" && statusChipClass.info,
        status === "unsupported" && statusChipClass.warning,
        status === "error" && statusChipClass.error,
        (status === "idle" || busy) &&
          "border-border bg-muted text-muted-foreground",
      )}
    >
      {busy ? (
        <Loader2 className="size-3 animate-spin" aria-hidden="true" />
      ) : (
        <Activity className="size-3" aria-hidden="true" />
      )}
      {label}
    </span>
  );
}
