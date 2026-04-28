import { useEffect, useMemo, useState } from "react";
import { Plus, Trash2, Pencil, Loader2, FolderOpen, Cpu, Info, Database, Globe, Palette, Droplets, Bot, ChevronDown, ChevronRight, KeyRound, Search, Server } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import i18n from "@/i18n";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
} from "@/components/ui/card";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { Switch } from "@/components/ui/switch";
import { useSettingsStore } from "@/stores/settingsStore";
import { useThemeStore, ThemeFlavor, CatppuccinAccent, ACCENT_NAMES, THEME_FLAVORS } from "@/stores/themeStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useDiscoverStore } from "@/stores/discoverStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useTargetStore } from "@/stores/targetStore";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  getPlatformCategoryKey,
  sortPlatformVisibilityAgents,
  type PlatformCategoryKey,
} from "@/lib/platformVisibility";
import { AddDirectoryDialog } from "@/components/settings/AddDirectoryDialog";
import { PlatformDialog } from "@/components/settings/PlatformDialog";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { Input } from "@/components/ui/input";
import { AgentWithStatus, ScanDirectory, SshAuthMethod, TargetSummary } from "@/types";
import { AI_PROVIDERS, REGION_LABELS, RegionId } from "@/data/aiProviders";
import { deriveHomeDir, formatPathForDisplay, joinPathForDisplay } from "@/lib/path";
import {
  createPlatformTargetGroups,
  getPlatformTargetMemberNames,
  isUniversalPlatformTarget,
  type PlatformTarget,
  type PlatformTargetGroup,
} from "@/lib/platformTargetGroups";

// ─── App constants ────────────────────────────────────────────────────────────

const APP_VERSION = __APP_VERSION__;
const DB_PATH_FALLBACK = "~/.skillsmanage/db.sqlite";
const REPO_URL = "https://github.com/bahayonghang/skills-manage-windows";

type SshTargetFormState = {
  label: string;
  host: string;
  username: string;
  port: string;
  authMethod: SshAuthMethod;
  keyPath: string;
  password: string;
};

const EMPTY_SSH_TARGET_FORM: SshTargetFormState = {
  label: "",
  host: "",
  username: "",
  port: "22",
  authMethod: "key",
  keyPath: "",
  password: "",
};

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

/** Default accent hex per theme, used for visual preview dots on theme buttons. */
const FLAVOR_COLORS: Record<ThemeFlavor, string> = {
  mocha: "#b4befe",
  macchiato: "#b7bdf8",
  frappe: "#babbf1",
  latte: "#7287fd",
  "claude-light": "#cc785c",
  "claude-dark": "#cc785c",
};

/**
 * Mapping of accent name → CSS custom property name.
 * These are resolved at runtime via getComputedStyle to show the
 * actual color for the current flavor.
 */
const CTP_VAR_MAP: Record<CatppuccinAccent, string> = {
  rosewater: "--ctp-rosewater",
  flamingo: "--ctp-flamingo",
  pink: "--ctp-pink",
  mauve: "--ctp-mauve",
  red: "--ctp-red",
  maroon: "--ctp-maroon",
  peach: "--ctp-peach",
  yellow: "--ctp-yellow",
  green: "--ctp-green",
  teal: "--ctp-teal",
  sky: "--ctp-sky",
  sapphire: "--ctp-sapphire",
  blue: "--ctp-blue",
  lavender: "--ctp-lavender",
};

const FLAVOR_ORDER: ThemeFlavor[] = THEME_FLAVORS;

// ─── ScanDirectoryRow ─────────────────────────────────────────────────────────

interface ScanDirectoryRowProps {
  dir: ScanDirectory;
  onRemove: () => void;
  onToggle: (active: boolean) => void;
  isRemoving: boolean;
}

function ScanDirectoryRow({ dir, onRemove, onToggle, isRemoving }: ScanDirectoryRowProps) {
  const { t } = useTranslation();
  const action = dir.is_active ? t("settings.enabled") : t("settings.disabled");
  return (
    <div className="flex items-center gap-3 py-2.5 px-4 border-b border-border/50 last:border-0">
      <FolderOpen className="size-4 text-muted-foreground shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium truncate">{formatPathForDisplay(dir.path)}</div>
        {dir.label && (
          <div className="text-xs text-muted-foreground mt-0.5">{dir.label}</div>
        )}
        {dir.is_builtin && (
          <div className="text-xs text-muted-foreground mt-0.5">{t("settings.builtinDir")}</div>
        )}
      </div>
      <div className="flex items-center gap-2 shrink-0">
        {/* Toggle for non-builtin dirs (built-in dirs are always active) */}
        {!dir.is_builtin && (
          <div className="flex items-center gap-1.5">
            <span className="text-xs text-muted-foreground">
              {action}
            </span>
            <Switch
              checked={dir.is_active}
              onCheckedChange={onToggle}
              aria-label={t("settings.enableDirLabel", { action, path: dir.path })}
            />
          </div>
        )}
        {/* Remove button for non-builtin dirs */}
        {!dir.is_builtin && (
          <InlineConfirmAction
            onConfirm={onRemove}
            isLoading={isRemoving}
            idleAriaLabel={t("settings.removeDirLabel", { path: dir.path })}
            idleTitle={t("settings.removeDirLabel", { path: dir.path })}
            confirmLabel={t("common.confirmDelete")}
            icon={<Trash2 className="size-3.5" />}
          />
        )}
      </div>
    </div>
  );
}

// ─── CustomPlatformRow ────────────────────────────────────────────────────────

interface CustomPlatformRowProps {
  agent: AgentWithStatus;
  onEdit: () => void;
  onRemove: () => void;
  isRemoving: boolean;
}

function CustomPlatformRow({ agent, onEdit, onRemove, isRemoving }: CustomPlatformRowProps) {
  const { t } = useTranslation();
  return (
    <div className="flex items-center gap-3 py-2.5 px-4 border-b border-border/50 last:border-0">
      <Cpu className="size-4 text-muted-foreground shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium truncate">{agent.display_name}</div>
        <div className="text-xs text-muted-foreground truncate mt-0.5">
          {formatPathForDisplay(agent.global_skills_dir)}
        </div>
      </div>
      <div className="flex items-center gap-2 shrink-0">
        <Button
          variant="outline"
          size="sm"
          onClick={onEdit}
          aria-label={t("settings.editPlatformLabel", { name: agent.display_name })}
        >
          <Pencil className="size-3.5" />
          <span>{t("common.edit")}</span>
        </Button>
        <InlineConfirmAction
          onConfirm={onRemove}
          isLoading={isRemoving}
          idleAriaLabel={t("settings.removePlatformLabel", { name: agent.display_name })}
          idleTitle={t("settings.removePlatformLabel", { name: agent.display_name })}
          confirmLabel={t("common.confirmDelete")}
          icon={<Trash2 className="size-3.5" />}
        />
      </div>
    </div>
  );
}

interface PlatformVisibilityRowProps {
  agent: AgentWithStatus;
  onToggle: (enabled: boolean) => void;
}

function formatPlatformPathHint(path: string): string {
  const normalizedPath = path.replace(/\\/g, "/");

  if (normalizedPath.startsWith("~/")) {
    const homeSegments = normalizedPath.slice(2).split("/").filter(Boolean);
    if (homeSegments.length <= 3) {
      return normalizedPath;
    }

    return `~/${homeSegments.slice(-3).join("/")}`;
  }

  const segments = normalizedPath.split("/").filter(Boolean);
  if (segments.length <= 3) {
    return normalizedPath;
  }

  return `…/${segments.slice(-3).join("/")}`;
}

function matchesPlatformVisibilityQuery(
  agent: PlatformTarget,
  normalizedQuery: string
): boolean {
  if (!normalizedQuery) {
    return true;
  }

  const searchText = [
    isUniversalPlatformTarget(agent) ? "Universal universal-agents" : "",
    agent.display_name,
    agent.id,
    agent.global_skills_dir,
    ...getPlatformTargetMemberNames(agent),
    ...(isUniversalPlatformTarget(agent)
      ? agent.member_agents.flatMap((member) => [
          member.id,
          member.global_skills_dir,
        ])
      : []),
  ]
    .join(" ")
    .toLowerCase();

  return searchText.includes(normalizedQuery);
}

function PlatformVisibilityRow({ agent, onToggle }: PlatformVisibilityRowProps) {
  const { t } = useTranslation();
  const pathHint = formatPlatformPathHint(agent.global_skills_dir);

  return (
    <div className="flex items-center gap-3 border-t border-border/40 px-4 py-2 first:border-t-0">
      <PlatformIcon
        agentId={agent.id}
        className="size-4 text-muted-foreground shrink-0"
        size={16}
      />
      <div className="flex-1 min-w-0">
        <div className="truncate text-sm font-medium">{agent.display_name}</div>
        <div
          className="mt-0.5 flex items-center gap-1.5 text-[11px] text-muted-foreground/80"
          title={agent.global_skills_dir}
        >
          <FolderOpen className="size-3 shrink-0" />
          <span className="truncate">{pathHint}</span>
        </div>
      </div>
      <div className="flex w-16 justify-end shrink-0">
        <Switch
          checked={agent.is_enabled}
          onCheckedChange={onToggle}
          aria-label={t("settings.togglePlatformVisibilityLabel", {
            name: agent.display_name,
          })}
        />
      </div>
    </div>
  );
}

interface PlatformVisibilityUniversalRowProps {
  agent: PlatformTargetGroup;
  isSearchActive: boolean;
  normalizedQuery: string;
  onToggleAgent: (agentId: string, enabled: boolean) => void;
}

function PlatformVisibilityUniversalRow({
  agent,
  isSearchActive,
  normalizedQuery,
  onToggleAgent,
}: PlatformVisibilityUniversalRowProps) {
  const { t } = useTranslation();
  const [isExpanded, setIsExpanded] = useState(false);
  const members = agent.member_agents ?? [];
  const matchingMembers = isSearchActive
    ? members.filter((member) =>
        matchesPlatformVisibilityQuery(member, normalizedQuery)
      )
    : members;
  const visibleMembers =
    isSearchActive && matchingMembers.length > 0 ? matchingMembers : members;
  const pathHint = formatPlatformPathHint(agent.global_skills_dir);
  const enabledCount = members.filter((member) => member.is_enabled).length;
  const expanded = isExpanded || isSearchActive;

  return (
    <div className="border-t border-border/40 first:border-t-0">
      <button
        type="button"
        className="flex w-full items-center gap-3 px-4 py-2 text-left transition-colors hover:bg-muted/30"
        onClick={() => setIsExpanded((value) => !value)}
        aria-expanded={expanded}
      >
        <PlatformIcon
          agentId={agent.id}
          className="size-4 text-muted-foreground shrink-0"
          size={16}
        />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <div className="truncate text-sm font-medium">
              {t("platformTargets.universalLabel")}
            </div>
            <span className="rounded-full bg-muted px-2 py-0.5 text-[11px] text-muted-foreground">
              {t("settings.universalMembersSummary", {
                enabled: enabledCount,
                total: members.length,
              })}
            </span>
          </div>
          <div
            className="mt-0.5 flex items-center gap-1.5 text-[11px] text-muted-foreground/80"
            title={agent.global_skills_dir}
          >
            <FolderOpen className="size-3 shrink-0" />
            <span className="truncate">{pathHint}</span>
          </div>
        </div>
        {expanded ? (
          <ChevronDown className="size-4 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="size-4 text-muted-foreground shrink-0" />
        )}
      </button>

      {expanded && (
        <div className="bg-muted/10 pl-5">
          {visibleMembers.map((member) => (
            <PlatformVisibilityRow
              key={`universal-${member.id}`}
              agent={member}
              onToggle={(enabled) => onToggleAgent(member.id, enabled)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface PlatformVisibilityGroupProps {
  category: PlatformCategoryKey;
  title: string;
  description: string;
  agents: PlatformTarget[];
  enabledCount: number;
  totalCount: number;
  groupVisible: boolean;
  isSearchActive: boolean;
  normalizedQuery: string;
  onToggleGroup: (visible: boolean) => void;
  onToggleAgent: (agentId: string, enabled: boolean) => void;
}

function PlatformVisibilityGroup({
  category,
  title,
  description,
  agents,
  enabledCount,
  totalCount,
  groupVisible,
  isSearchActive,
  normalizedQuery,
  onToggleGroup,
  onToggleAgent,
}: PlatformVisibilityGroupProps) {
  const { t } = useTranslation();
  const isExpanded = groupVisible || isSearchActive;

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      <div className="flex items-start justify-between gap-4 border-b border-border/40 bg-muted/20 px-4 py-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <div className="text-sm font-medium">{title}</div>
            <span className="rounded-full bg-muted px-2 py-0.5 text-[11px] text-muted-foreground">
              {t("settings.platformEnabledSummary", {
                enabled: enabledCount,
                total: totalCount,
              })}
            </span>
          </div>
          <div className="mt-1 text-xs text-muted-foreground">{description}</div>
        </div>
        <div className="flex w-24 items-center justify-end gap-2 shrink-0">
          <span className="text-xs text-muted-foreground">
            {t("settings.platformGroupVisible")}
          </span>
          <Switch
            checked={groupVisible}
            onCheckedChange={onToggleGroup}
            aria-label={t("settings.toggleCategoryVisibilityLabel", { name: title })}
          />
        </div>
      </div>

      {!isExpanded ? (
        <div className="px-4 py-3 text-xs text-muted-foreground">
          {t("settings.platformGroupHiddenSummary", {
            enabled: enabledCount,
            total: totalCount,
          })}
        </div>
      ) : agents.length === 0 ? (
        <div className="px-4 py-3 text-xs text-muted-foreground">
          {t("settings.noPlatformItems")}
        </div>
      ) : (
        agents.map((agent) =>
          isUniversalPlatformTarget(agent) ? (
            <PlatformVisibilityUniversalRow
              key={`${category}-${agent.id}`}
              agent={agent}
              isSearchActive={isSearchActive}
              normalizedQuery={normalizedQuery}
              onToggleAgent={onToggleAgent}
            />
          ) : (
            <PlatformVisibilityRow
              key={`${category}-${agent.id}`}
              agent={agent}
              onToggle={(enabled) => onToggleAgent(agent.id, enabled)}
            />
          )
        )
      )}
    </div>
  );
}

// ─── SettingsView ─────────────────────────────────────────────────────────────

export function SettingsView() {
  const { t } = useTranslation();

  // ── Store State ────────────────────────────────────────────────────────────

  const scanDirectories = useSettingsStore((s) => s.scanDirectories);
  const isLoadingScanDirs = useSettingsStore((s) => s.isLoadingScanDirs);
  const loadScanDirectories = useSettingsStore((s) => s.loadScanDirectories);
  const addScanDirectory = useSettingsStore((s) => s.addScanDirectory);
  const removeScanDirectory = useSettingsStore((s) => s.removeScanDirectory);
  const toggleScanDirectory = useSettingsStore((s) => s.toggleScanDirectory);
  const addCustomAgent = useSettingsStore((s) => s.addCustomAgent);
  const updateCustomAgent = useSettingsStore((s) => s.updateCustomAgent);
  const removeCustomAgent = useSettingsStore((s) => s.removeCustomAgent);
  const githubPat = useSettingsStore((s) => s.githubPat);
  const isLoadingGitHubPat = useSettingsStore((s) => s.isLoadingGitHubPat);
  const isSavingGitHubPat = useSettingsStore((s) => s.isSavingGitHubPat);
  const isTestingGitHubPat = useSettingsStore((s) => s.isTestingGitHubPat);
  const loadGitHubPat = useSettingsStore((s) => s.loadGitHubPat);
  const saveGitHubPat = useSettingsStore((s) => s.saveGitHubPat);
  const clearGitHubPat = useSettingsStore((s) => s.clearGitHubPat);
  const testGitHubPat = useSettingsStore((s) => s.testGitHubPat);

  const agents = usePlatformStore((s) => s.agents);
  const categoryVisibility = usePlatformStore((s) => s.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const setCategoryVisibility = usePlatformStore((s) => s.setCategoryVisibility) ?? (async () => undefined);
  const setAgentEnabled = usePlatformStore((s) => s.setAgentEnabled) ?? (async () => undefined);
  const loadCentralSkills = useCentralSkillsStore((s) => s.loadCentralSkills);
  const refreshDiscoverCounts = useDiscoverStore((s) => s.refreshCounts);
  const loadMarketplaceRegistries = useMarketplaceStore((s) => s.loadRegistries);
  const selectedMarketplaceRegistryId = useMarketplaceStore((s) => s.selectedRegistryId);
  const loadMarketplaceSkills = useMarketplaceStore((s) => s.loadSkills);
  const targets = useTargetStore((s) => s.targets);
  const activeTarget = useTargetStore((s) => s.activeTarget);
  const isLoadingTargets = useTargetStore((s) => s.isLoading);
  const isCreatingTarget = useTargetStore((s) => s.isCreating);
  const updatingTargetId = useTargetStore((s) => s.updatingTargetId);
  const testingTargetId = useTargetStore((s) => s.testingTargetId);
  const updatingPasswordTargetId = useTargetStore((s) => s.updatingPasswordTargetId);
  const switchingTargetId = useTargetStore((s) => s.switchingTargetId);
  const deletingTargetId = useTargetStore((s) => s.deletingTargetId);
  const loadTargets = useTargetStore((s) => s.loadTargets);
  const createSshTarget = useTargetStore((s) => s.createSshTarget);
  const updateSshTarget = useTargetStore((s) => s.updateSshTarget);
  const testSshTarget = useTargetStore((s) => s.testSshTarget);
  const updateSshTargetPassword = useTargetStore((s) => s.updateSshTargetPassword);
  const deleteTarget = useTargetStore((s) => s.deleteTarget);
  const switchTarget = useTargetStore((s) => s.switchTarget);

  const flavor = useThemeStore((s) => s.flavor);
  const setFlavor = useThemeStore((s) => s.setFlavor);
  const accent = useThemeStore((s) => s.accent);
  const setAccent = useThemeStore((s) => s.setAccent);
  const rescan = usePlatformStore((s) => s.rescan);
  const refreshCounts = usePlatformStore((s) => s.refreshCounts);

  // Custom agents are those that are not built-in.
  const customAgents = agents.filter((a) => !a.is_builtin);
  const homeDir = useMemo(() => {
    const candidates = [
      agents.find((agent) => agent.id === "central")?.global_skills_dir,
      ...scanDirectories.map((dir) => dir.path),
      ...agents.map((agent) => agent.global_skills_dir),
    ].filter((candidate): candidate is string => Boolean(candidate));

    return candidates
      .map((candidate) => deriveHomeDir(candidate))
      .find((candidate): candidate is string => Boolean(candidate));
  }, [agents, scanDirectories]);
  const dbPathDisplay = useMemo(
    () => (homeDir ? joinPathForDisplay(homeDir, ".skillsmanage/db.sqlite") : DB_PATH_FALLBACK),
    [homeDir]
  );

  // ── Local State ────────────────────────────────────────────────────────────

  const [platformVisibilityQuery, setPlatformVisibilityQuery] = useState("");

  // AI Provider state
  const [aiProvider, setAiProvider] = useState("claude");
  const [aiRegion, setAiRegion] = useState<RegionId>("intl");
  const [aiApiKey, setAiApiKey] = useState("");
  const [aiModel, setAiModel] = useState("");
  const [aiCustomUrl, setAiCustomUrl] = useState("");
  const [aiTagConcurrency, setAiTagConcurrency] = useState("1");
  const [aiTagIntervalMs, setAiTagIntervalMs] = useState("4000");
  const [aiTagStopOnRateLimit, setAiTagStopOnRateLimit] = useState(true);
  const [aiLoaded, setAiLoaded] = useState(false);

  // Load AI settings on mount
  useEffect(() => {
    (async () => {
      try {
        const provider = await invoke<string | null>("get_setting", { key: "ai_provider" });
        const region = await invoke<string | null>("get_setting", { key: "ai_region" });
        const key = await invoke<string | null>("get_setting", { key: "ai_api_key" });
        const model = await invoke<string | null>("get_setting", { key: "ai_model" });
        const url = await invoke<string | null>("get_setting", { key: "ai_api_url" });
        const tagConcurrency = await invoke<string | null>("get_setting", { key: "ai_tag_concurrency" });
        const tagIntervalMs = await invoke<string | null>("get_setting", { key: "ai_tag_interval_ms" });
        const tagStopOnRateLimit = await invoke<string | null>("get_setting", { key: "ai_tag_stop_on_rate_limit" });
        if (provider) setAiProvider(provider);
        if (region) setAiRegion(region as RegionId);
        if (key) setAiApiKey(key);
        if (model) setAiModel(model);
        if (url) setAiCustomUrl(url);
        if (tagConcurrency) setAiTagConcurrency(tagConcurrency);
        if (tagIntervalMs) setAiTagIntervalMs(tagIntervalMs);
        if (tagStopOnRateLimit) {
          setAiTagStopOnRateLimit(tagStopOnRateLimit !== "false" && tagStopOnRateLimit !== "0");
        }
      } catch { /* first run, no settings yet */ }
      setAiLoaded(true);
    })();
  }, []);

  // Save AI settings when changed
  useEffect(() => {
    if (!aiLoaded) return;
    const save = async () => {
      try {
        await invoke("set_setting", { key: "ai_provider", value: aiProvider });
        await invoke("set_setting", { key: "ai_region", value: aiRegion });
        await invoke("set_setting", { key: "ai_api_key", value: aiApiKey });
        await invoke("set_setting", { key: "ai_model", value: aiModel });
        // Compute and save the actual API URL
        const p = AI_PROVIDERS.find((x) => x.id === aiProvider);
        const url = aiProvider === "custom" ? aiCustomUrl : (p?.endpoints[aiRegion] ?? "");
        await invoke("set_setting", { key: "ai_api_url", value: url });
        await invoke("set_setting", { key: "ai_tag_concurrency", value: aiTagConcurrency });
        await invoke("set_setting", { key: "ai_tag_interval_ms", value: aiTagIntervalMs });
        await invoke("set_setting", { key: "ai_tag_stop_on_rate_limit", value: aiTagStopOnRateLimit ? "true" : "false" });
      } catch { /* ignore */ }
    };
    save();
  }, [
    aiProvider,
    aiRegion,
    aiApiKey,
    aiModel,
    aiCustomUrl,
    aiTagConcurrency,
    aiTagIntervalMs,
    aiTagStopOnRateLimit,
    aiLoaded,
  ]);

  // When provider or region changes, update model to default
  function handleProviderChange(id: string) {
    setAiProvider(id);
    const p = AI_PROVIDERS.find((x) => x.id === id);
    if (p) {
      setAiModel(p.defaultModel);
      // Auto-select first available region
      if (!p.regions.includes(aiRegion)) {
        setAiRegion(p.regions[0]);
      }
    }
  }

  const currentProvider = AI_PROVIDERS.find((p) => p.id === aiProvider);
  const resolvedUrl = aiProvider === "custom"
    ? aiCustomUrl
    : (currentProvider?.endpoints[aiRegion] ?? "");
  const lang = i18n.language;
  const [aiTesting, setAiTesting] = useState(false);
  const [aiTestResult, setAiTestResult] = useState<{ ok: boolean; msg: string; details?: string } | null>(null);
  const [showAiTestDetails, setShowAiTestDetails] = useState(false);

  const [isAddDirOpen, setIsAddDirOpen] = useState(false);
  const [showBuiltinDirs, setShowBuiltinDirs] = useState(false);
  const [isPlatformDialogOpen, setIsPlatformDialogOpen] = useState(false);
  const [editingPlatform, setEditingPlatform] = useState<AgentWithStatus | null>(null);
  const [removingDir, setRemovingDir] = useState<string | null>(null);
  const [removingAgent, setRemovingAgent] = useState<string | null>(null);
  const [scanDirError, setScanDirError] = useState<string | null>(null);
  const [platformError, setPlatformError] = useState<string | null>(null);
  const [githubPatInput, setGitHubPatInput] = useState("");
  const [githubPatMessage, setGitHubPatMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [sshTargetForm, setSshTargetForm] = useState<SshTargetFormState>(EMPTY_SSH_TARGET_FORM);
  const [editingTargetId, setEditingTargetId] = useState<string | null>(null);
  const [sshTargetEditForm, setSshTargetEditForm] =
    useState<SshTargetFormState>(EMPTY_SSH_TARGET_FORM);
  const [sshTargetPasswordUpdates, setSshTargetPasswordUpdates] = useState<Record<string, string>>({});
  const [targetMessage, setTargetMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const normalizedPlatformVisibilityQuery = useMemo(
    () => platformVisibilityQuery.trim().toLowerCase(),
    [platformVisibilityQuery]
  );
  const isPlatformVisibilitySearchActive =
    normalizedPlatformVisibilityQuery.length > 0;
  const allPlatformAgents = useMemo(
    () => agents.filter((agent) => agent.id !== "central"),
    [agents]
  );
  const platformVisibilityGroups = useMemo(() => {
    const groupConfigs = [
      {
        category: "coding" as const,
        title: t("sidebar.categoryCoding"),
        description: t("settings.platformGroupCodingDesc"),
        groupVisible: categoryVisibility.coding,
      },
      {
        category: "lobster" as const,
        title: t("sidebar.categoryLobster"),
        description: t("settings.platformGroupLobsterDesc"),
        groupVisible: categoryVisibility.lobster,
      },
    ];

    return groupConfigs
      .map((group) => {
        const groupAgents = sortPlatformVisibilityAgents(
          allPlatformAgents.filter(
            (agent) => getPlatformCategoryKey(agent.category) === group.category
          )
        );
        const groupedAgents = createPlatformTargetGroups(groupAgents, agents);
        const matchingAgents = groupedAgents.filter((agent) =>
          matchesPlatformVisibilityQuery(agent, normalizedPlatformVisibilityQuery)
        );

        return {
          ...group,
          enabledCount: groupAgents.filter((agent) => agent.is_enabled).length,
          totalCount: groupAgents.length,
          agents: matchingAgents,
        };
      })
      .filter(
        (group) =>
          !isPlatformVisibilitySearchActive || group.agents.length > 0
      );
  }, [
    allPlatformAgents,
    agents,
    categoryVisibility.coding,
    categoryVisibility.lobster,
    isPlatformVisibilitySearchActive,
    normalizedPlatformVisibilityQuery,
    t,
  ]);

  // ── Load on mount ──────────────────────────────────────────────────────────

  useEffect(() => {
    loadScanDirectories();
    loadGitHubPat();
    loadTargets();
  }, [loadScanDirectories, loadGitHubPat, loadTargets]);

  useEffect(() => {
    setGitHubPatInput(githubPat);
  }, [githubPat]);

  const isGitHubPatDirty = useMemo(() => githubPatInput.trim() !== githubPat, [githubPatInput, githubPat]);

  async function refreshAfterTargetChange() {
    await rescan();
    await Promise.allSettled([
      loadCentralSkills(),
      refreshDiscoverCounts(),
      loadMarketplaceRegistries().then(() => {
        if (selectedMarketplaceRegistryId) {
          return loadMarketplaceSkills(selectedMarketplaceRegistryId);
        }
      }),
    ]);
  }

  function updateSshTargetForm(field: keyof SshTargetFormState, value: string) {
    setSshTargetForm((current) => ({ ...current, [field]: value }));
  }

  function updateSshTargetEditForm(field: keyof SshTargetFormState, value: string) {
    setSshTargetEditForm((current) => ({ ...current, [field]: value }));
  }

  function updateExistingTargetPassword(targetId: string, value: string) {
    setSshTargetPasswordUpdates((current) => ({ ...current, [targetId]: value }));
  }

  function targetToSshTargetForm(target: TargetSummary): SshTargetFormState {
    return {
      label: target.label,
      host: target.host ?? "",
      username: target.username ?? "",
      port: String(target.port ?? 22),
      authMethod: target.authMethod ?? "key",
      keyPath: target.keyPath ?? "",
      password: "",
    };
  }

  function sshTargetPayload(form: SshTargetFormState, includeEmptyPassword: boolean) {
    const port = Number(form.port.trim() || "22");
    const authMethod = form.authMethod;
    const password = form.password.trim();
    return {
      label: form.label.trim(),
      host: form.host.trim(),
      username: form.username.trim(),
      port: Number.isFinite(port) ? port : 22,
      authMethod,
      keyPath: authMethod === "key" ? form.keyPath.trim() : null,
      password: authMethod === "password" && (includeEmptyPassword || password)
        ? form.password
        : null,
    };
  }

  function handleStartEditTarget(target: TargetSummary) {
    setTargetMessage(null);
    setEditingTargetId(target.id);
    setSshTargetEditForm(targetToSshTargetForm(target));
  }

  function handleCancelEditTarget() {
    setEditingTargetId(null);
    setSshTargetEditForm(EMPTY_SSH_TARGET_FORM);
  }

  async function handleCreateSshTarget() {
    setTargetMessage(null);
    try {
      const target = await createSshTarget(sshTargetPayload(sshTargetForm, true));
      setSshTargetForm(EMPTY_SSH_TARGET_FORM);
      setTargetMessage({ type: "success", text: t("targets.created", { label: target.label }) });
      toast.success(t("targets.created", { label: target.label }));
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleTestNewSshTarget() {
    setTargetMessage(null);
    try {
      const result = await testSshTarget(sshTargetPayload(sshTargetForm, true));
      const text = result.ok
        ? t("targets.testSucceeded", {
            home: result.remoteHome ?? "",
            os: result.remoteOs ?? "",
          })
        : result.message;
      setTargetMessage({ type: result.ok ? "success" : "error", text });
      if (result.ok) {
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleTestExistingTarget(targetId: string) {
    setTargetMessage(null);
    try {
      const password = sshTargetPasswordUpdates[targetId];
      const result = await testSshTarget({
        id: targetId,
        password: password?.trim() ? password : null,
      });
      const text = result.ok
        ? t("targets.testSucceeded", {
            home: result.remoteHome ?? "",
            os: result.remoteOs ?? "",
          })
        : result.message;
      setTargetMessage({ type: result.ok ? "success" : "error", text });
      if (result.ok) {
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleUpdateSshTarget(target: TargetSummary) {
    setTargetMessage(null);
    try {
      const updatedTarget = await updateSshTarget({
        id: target.id,
        ...sshTargetPayload(sshTargetEditForm, false),
      });
      setEditingTargetId(null);
      setSshTargetEditForm(EMPTY_SSH_TARGET_FORM);
      updateExistingTargetPassword(target.id, "");
      await refreshAfterTargetChange();
      const text = t("targets.updated", { label: updatedTarget.label });
      setTargetMessage({ type: "success", text });
      toast.success(text);
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleUpdateTargetPassword(target: TargetSummary) {
    const password = sshTargetPasswordUpdates[target.id] ?? "";
    if (!password.trim()) {
      const text = t("targets.passwordRequired");
      setTargetMessage({ type: "error", text });
      toast.error(text);
      return;
    }

    setTargetMessage(null);
    try {
      const result = await updateSshTargetPassword(target.id, password);
      const text = result.ok
        ? result.credentialStatus === "session"
          ? t("targets.passwordUpdatedSession", { label: target.label })
          : t("targets.passwordUpdated", { label: target.label })
        : result.message;
      setTargetMessage({ type: result.ok ? "success" : "error", text });
      if (result.ok) {
        updateExistingTargetPassword(target.id, "");
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleSwitchTarget(targetId: string) {
    setTargetMessage(null);
    try {
      const target = await switchTarget(targetId);
      await refreshAfterTargetChange();
      toast.success(t("targets.switched", { label: target.label }));
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleDeleteTarget(targetId: string) {
    setTargetMessage(null);
    try {
      await deleteTarget(targetId);
      await refreshAfterTargetChange();
      toast.success(t("targets.deleted"));
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  // ── Scan Directories Handlers ──────────────────────────────────────────────

  async function handleAddDirectory(path: string) {
    setScanDirError(null);
    try {
      await addScanDirectory(path);
      // Trigger rescan after adding a directory.
      await refreshCounts();
      toast.success(t("addDir.add") + " ✓");
    } catch (err) {
      setScanDirError(String(err));
      toast.error(String(err));
      throw err; // Re-throw so the dialog knows it failed
    }
  }

  async function handleRemoveDirectory(path: string) {
    setRemovingDir(path);
    setScanDirError(null);
    try {
      await removeScanDirectory(path);
      // Trigger rescan after removing a directory.
      await refreshCounts();
      toast.success(t("common.delete") + " ✓");
    } catch (err) {
      setScanDirError(String(err));
      toast.error(String(err));
    } finally {
      setRemovingDir(null);
    }
  }

  /**
   * Toggle the active state of a custom scan directory.
   * Persists the change to the backend via set_scan_directory_active command.
   */
  async function handleToggleDirectory(path: string, active: boolean) {
    setScanDirError(null);
    try {
      await toggleScanDirectory(path, active);
    } catch (err) {
      setScanDirError(String(err));
      toast.error(String(err));
    }
  }

  // ── Custom Platform Handlers ───────────────────────────────────────────────

  function handleOpenAddPlatform() {
    setEditingPlatform(null);
    setPlatformError(null);
    setIsPlatformDialogOpen(true);
  }

  function handleOpenEditPlatform(agent: AgentWithStatus) {
    setEditingPlatform(agent);
    setPlatformError(null);
    setIsPlatformDialogOpen(true);
  }

  async function handleAddPlatform(displayName: string, globalSkillsDir: string, category?: string) {
    setPlatformError(null);
    try {
      await addCustomAgent({
        display_name: displayName,
        global_skills_dir: globalSkillsDir,
        category: category || "coding",
      });
      // Refresh agents + rescan to show new platform in sidebar.
      await rescan();
      toast.success(t("platformDialog.add") + " ✓");
    } catch (err) {
      setPlatformError(String(err));
      toast.error(String(err));
      throw err;
    }
  }

  async function handleEditPlatform(displayName: string, globalSkillsDir: string, category?: string) {
    if (!editingPlatform) return;
    setPlatformError(null);
    try {
      await updateCustomAgent(editingPlatform.id, {
        display_name: displayName,
        global_skills_dir: globalSkillsDir,
        category: category || "coding",
      });
      // Refresh agents + rescan.
      await rescan();
      toast.success(t("platformDialog.save") + " ✓");
    } catch (err) {
      setPlatformError(String(err));
      toast.error(String(err));
      throw err;
    }
  }

  async function handleRemovePlatform(agentId: string) {
    setRemovingAgent(agentId);
    setPlatformError(null);
    try {
      await removeCustomAgent(agentId);
      // Refresh agents.
      await rescan();
      toast.success(t("common.delete") + " ✓");
    } catch (err) {
      setPlatformError(String(err));
      toast.error(String(err));
    } finally {
      setRemovingAgent(null);
    }
  }

  async function handleToggleCategory(category: PlatformCategoryKey, visible: boolean) {
    try {
      await setCategoryVisibility(category, visible);
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function handleTogglePlatformVisibility(agentId: string, enabled: boolean) {
    try {
      await setAgentEnabled(agentId, enabled);
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function handleSaveGitHubPat() {
    setGitHubPatMessage(null);
    try {
      await saveGitHubPat(githubPatInput);
      setGitHubPatMessage({
        type: "success",
        text: t("settings.githubPatSaved"),
      });
      toast.success(t("settings.githubPatSaved"));
    } catch (err) {
      const text = String(err);
      setGitHubPatMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleClearGitHubPat() {
    setGitHubPatMessage(null);
    try {
      await clearGitHubPat();
      setGitHubPatInput("");
      setGitHubPatMessage({
        type: "success",
        text: t("settings.githubPatCleared"),
      });
      toast.success(t("settings.githubPatCleared"));
    } catch (err) {
      const text = String(err);
      setGitHubPatMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleTestGitHubPat() {
    setGitHubPatMessage(null);
    try {
      const result = await testGitHubPat();
      setGitHubPatMessage({
        type: result.ok ? "success" : "error",
        text: result.message,
      });
      if (result.ok) {
        toast.success(result.message);
      } else {
        toast.error(result.message);
      }
    } catch (err) {
      const text = String(err);
      setGitHubPatMessage({ type: "error", text });
      toast.error(text);
    }
  }

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b border-border px-6 py-4">
        <h1 className="text-xl font-semibold">{t("settings.title")}</h1>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6 space-y-6">

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
                {targets.map((target) => {
                  const isLocal = target.kind === "local";
                  const isActive = target.id === activeTarget.id || target.isActive;
                  const passwordUpdateValue = sshTargetPasswordUpdates[target.id] ?? "";
                  const credentialStatus = sshCredentialStatus(target);
                  const isEditing = editingTargetId === target.id;
                  return (
                    <div
                      key={target.id}
                      className="border-b border-border/50 last:border-0"
                    >
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
                            <div className="mt-2 max-w-xl space-y-2">
                              <div className="flex flex-wrap items-center gap-2">
                                <span
                                  className={`rounded-full border px-2 py-0.5 text-[11px] ${sshCredentialStatusClass(credentialStatus)}`}
                                >
                                  {t(`targets.credentialStatus.${credentialStatus ?? "missing"}`)}
                                </span>
                                {target.credentialError ? (
                                  <span className="text-[11px] text-muted-foreground">
                                    {target.credentialError}
                                  </span>
                                ) : null}
                              </div>
                              <div className="flex flex-col gap-2 sm:flex-row">
                                <Input
                                  type="password"
                                  value={passwordUpdateValue}
                                  onChange={(event) => updateExistingTargetPassword(target.id, event.target.value)}
                                  placeholder={t("targets.updatePasswordPlaceholder")}
                                  aria-label={t("targets.updatePasswordFor", { label: target.label })}
                                  className="h-8 text-xs"
                                />
                                <Button
                                  type="button"
                                  variant="outline"
                                  size="sm"
                                  disabled={
                                    updatingPasswordTargetId === target.id ||
                                    testingTargetId === target.id ||
                                    !passwordUpdateValue.trim()
                                  }
                                  onClick={() => void handleUpdateTargetPassword(target)}
                                >
                                  {updatingPasswordTargetId === target.id ? (
                                    <Loader2 className="size-3.5 animate-spin" />
                                  ) : null}
                                  {t("targets.updatePassword")}
                                </Button>
                              </div>
                            </div>
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
                                isEditing ? handleCancelEditTarget() : handleStartEditTarget(target)
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
                              onClick={() => void handleTestExistingTarget(target.id)}
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
                              onClick={() => void handleSwitchTarget(target.id)}
                            >
                              {switchingTargetId === target.id ? (
                                <Loader2 className="size-3.5 animate-spin" />
                              ) : null}
                              {t("targets.activate")}
                            </Button>
                          )}
                          {!isLocal && (
                            <InlineConfirmAction
                              onConfirm={() => handleDeleteTarget(target.id)}
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
                          <div className="grid gap-3 rounded-lg border border-border/70 bg-muted/20 p-3 md:grid-cols-5">
                            <div className="space-y-1 md:col-span-5">
                              <div
                                id={`edit-ssh-auth-method-${target.id}`}
                                className="text-xs text-muted-foreground"
                              >
                                {t("targets.authMethodLabel")}
                              </div>
                              <div
                                className="flex gap-2"
                                role="group"
                                aria-labelledby={`edit-ssh-auth-method-${target.id}`}
                              >
                                {(["key", "password"] as const).map((method) => (
                                  <Button
                                    key={method}
                                    type="button"
                                    variant={sshTargetEditForm.authMethod === method ? "default" : "outline"}
                                    className="flex-1"
                                    onClick={() => updateSshTargetEditForm("authMethod", method)}
                                    aria-pressed={sshTargetEditForm.authMethod === method}
                                  >
                                    {method === "key" ? t("targets.authKey") : t("targets.authPassword")}
                                  </Button>
                                ))}
                              </div>
                            </div>
                            <div className="space-y-1">
                              <label
                                htmlFor={`edit-ssh-label-${target.id}`}
                                className="text-xs text-muted-foreground"
                              >
                                {t("targets.labelPlaceholder")}
                              </label>
                              <Input
                                id={`edit-ssh-label-${target.id}`}
                                value={sshTargetEditForm.label}
                                onChange={(event) => updateSshTargetEditForm("label", event.target.value)}
                                placeholder={t("targets.labelPlaceholder")}
                              />
                            </div>
                            <div className="space-y-1">
                              <label
                                htmlFor={`edit-ssh-host-${target.id}`}
                                className="text-xs text-muted-foreground"
                              >
                                {t("targets.hostPlaceholder")}
                              </label>
                              <Input
                                id={`edit-ssh-host-${target.id}`}
                                value={sshTargetEditForm.host}
                                onChange={(event) => updateSshTargetEditForm("host", event.target.value)}
                                placeholder={t("targets.hostPlaceholder")}
                              />
                            </div>
                            <div className="space-y-1">
                              <label
                                htmlFor={`edit-ssh-username-${target.id}`}
                                className="text-xs text-muted-foreground"
                              >
                                {t("targets.usernamePlaceholder")}
                              </label>
                              <Input
                                id={`edit-ssh-username-${target.id}`}
                                value={sshTargetEditForm.username}
                                onChange={(event) => updateSshTargetEditForm("username", event.target.value)}
                                placeholder={t("targets.usernamePlaceholder")}
                              />
                            </div>
                            <div className="space-y-1">
                              <label
                                htmlFor={`edit-ssh-port-${target.id}`}
                                className="text-xs text-muted-foreground"
                              >
                                {t("targets.portLabel")}
                              </label>
                              <Input
                                id={`edit-ssh-port-${target.id}`}
                                value={sshTargetEditForm.port}
                                onChange={(event) => updateSshTargetEditForm("port", event.target.value)}
                                placeholder="22"
                              />
                            </div>
                            <div className="space-y-1 md:col-span-3">
                              <label
                                htmlFor={`edit-ssh-credential-${target.id}`}
                                className="text-xs text-muted-foreground"
                              >
                                {sshTargetEditForm.authMethod === "key"
                                  ? t("targets.keyPathPlaceholder")
                                  : t("targets.passwordPlaceholder")}
                              </label>
                              <Input
                                id={`edit-ssh-credential-${target.id}`}
                                type={sshTargetEditForm.authMethod === "password" ? "password" : "text"}
                                value={
                                  sshTargetEditForm.authMethod === "key"
                                    ? sshTargetEditForm.keyPath
                                    : sshTargetEditForm.password
                                }
                                onChange={(event) =>
                                  updateSshTargetEditForm(
                                    sshTargetEditForm.authMethod === "key" ? "keyPath" : "password",
                                    event.target.value
                                  )
                                }
                                placeholder={
                                  sshTargetEditForm.authMethod === "key"
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
                                onClick={handleCancelEditTarget}
                              >
                                {t("common.cancel")}
                              </Button>
                              <Button
                                type="button"
                                className="flex-1"
                                disabled={updatingTargetId === target.id}
                                onClick={() => void handleUpdateSshTarget(target)}
                              >
                                {updatingTargetId === target.id ? (
                                  <Loader2 className="size-3.5 animate-spin" />
                                ) : null}
                                {t("targets.saveChanges")}
                              </Button>
                            </div>
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>

              <div className="grid gap-3 rounded-lg border border-border/70 bg-muted/20 p-4 md:grid-cols-5">
                <div className="space-y-1 md:col-span-5">
                  <div id="new-ssh-auth-method" className="text-xs text-muted-foreground">
                    {t("targets.authMethodLabel")}
                  </div>
                  <div className="flex gap-2" role="group" aria-labelledby="new-ssh-auth-method">
                  {(["key", "password"] as const).map((method) => (
                    <Button
                      key={method}
                      type="button"
                      variant={sshTargetForm.authMethod === method ? "default" : "outline"}
                      className="flex-1"
                      onClick={() => updateSshTargetForm("authMethod", method)}
                      aria-pressed={sshTargetForm.authMethod === method}
                    >
                      {method === "key" ? t("targets.authKey") : t("targets.authPassword")}
                    </Button>
                  ))}
                  </div>
                </div>
                <div className="space-y-1">
                  <label htmlFor="new-ssh-label" className="text-xs text-muted-foreground">
                    {t("targets.labelPlaceholder")}
                  </label>
                  <Input
                    id="new-ssh-label"
                    value={sshTargetForm.label}
                    onChange={(event) => updateSshTargetForm("label", event.target.value)}
                    placeholder={t("targets.labelPlaceholder")}
                  />
                </div>
                <div className="space-y-1">
                  <label htmlFor="new-ssh-host" className="text-xs text-muted-foreground">
                    {t("targets.hostPlaceholder")}
                  </label>
                  <Input
                    id="new-ssh-host"
                    value={sshTargetForm.host}
                    onChange={(event) => updateSshTargetForm("host", event.target.value)}
                    placeholder={t("targets.hostPlaceholder")}
                  />
                </div>
                <div className="space-y-1">
                  <label htmlFor="new-ssh-username" className="text-xs text-muted-foreground">
                    {t("targets.usernamePlaceholder")}
                  </label>
                  <Input
                    id="new-ssh-username"
                    value={sshTargetForm.username}
                    onChange={(event) => updateSshTargetForm("username", event.target.value)}
                    placeholder={t("targets.usernamePlaceholder")}
                  />
                </div>
                <div className="space-y-1">
                  <label htmlFor="new-ssh-port" className="text-xs text-muted-foreground">
                    {t("targets.portLabel")}
                  </label>
                  <Input
                    id="new-ssh-port"
                    value={sshTargetForm.port}
                    onChange={(event) => updateSshTargetForm("port", event.target.value)}
                    placeholder="22"
                  />
                </div>
                <div className="space-y-1 md:col-span-3">
                  <label htmlFor="new-ssh-credential" className="text-xs text-muted-foreground">
                    {sshTargetForm.authMethod === "key"
                      ? t("targets.keyPathPlaceholder")
                      : t("targets.passwordPlaceholder")}
                  </label>
                  <Input
                    id="new-ssh-credential"
                    type={sshTargetForm.authMethod === "password" ? "password" : "text"}
                    value={sshTargetForm.authMethod === "key" ? sshTargetForm.keyPath : sshTargetForm.password}
                    onChange={(event) =>
                      updateSshTargetForm(
                        sshTargetForm.authMethod === "key" ? "keyPath" : "password",
                        event.target.value
                      )
                    }
                    placeholder={
                      sshTargetForm.authMethod === "key"
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
                    disabled={testingTargetId === "new" || isCreatingTarget}
                    onClick={() => void handleTestNewSshTarget()}
                  >
                    {testingTargetId === "new" ? (
                      <Loader2 className="size-3.5 animate-spin" />
                    ) : null}
                    {t("targets.test")}
                  </Button>
                  <Button
                    type="button"
                    className="flex-1"
                    disabled={isCreatingTarget}
                    onClick={() => void handleCreateSshTarget()}
                  >
                    {isCreatingTarget ? <Loader2 className="size-3.5 animate-spin" /> : <Plus className="size-3.5" />}
                    {t("targets.add")}
                  </Button>
                </div>
              </div>

              <div className="space-y-1 text-xs text-muted-foreground">
                <p>{t("targets.sshHint")}</p>
                <p>{t("targets.passwordRepairHint")}</p>
                <p>{t("targets.sshKeyPathHelp")}</p>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* ── Section 1: Custom Platforms ───────────────────────────────────── */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>{t("settings.customPlatforms")}</CardTitle>
                <CardDescription className="mt-1">
                  {t("settings.customPlatformsDesc")}
                </CardDescription>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={handleOpenAddPlatform}
                aria-label={t("settings.addPlatformAriaLabel")}
              >
                <Plus className="size-3.5" />
                <span>{t("settings.addPlatform")}</span>
              </Button>
            </div>
          </CardHeader>

          <CardContent>
            {platformError && (
              <p className="text-xs text-destructive mb-3" role="alert">
                {platformError}
              </p>
            )}

            {customAgents.length === 0 ? (
              <p className="text-sm text-muted-foreground py-4 text-center">
                {t("settings.noPlatforms")}
              </p>
            ) : (
              <div className="rounded-lg border border-border overflow-hidden">
                {customAgents.map((agent) => (
                  <CustomPlatformRow
                    key={agent.id}
                    agent={agent}
                    onEdit={() => handleOpenEditPlatform(agent)}
                    onRemove={() => handleRemovePlatform(agent.id)}
                    isRemoving={removingAgent === agent.id}
                  />
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* ── Section 2: Platform Visibility ────────────────────────────── */}
        <Card>
          <CardHeader>
            <div>
              <CardTitle>{t("settings.platformVisibility")}</CardTitle>
              <CardDescription className="mt-1">
                {t("settings.platformVisibilityDesc")}
              </CardDescription>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="relative">
                <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  value={platformVisibilityQuery}
                  onChange={(event) =>
                    setPlatformVisibilityQuery(event.target.value)
                  }
                  placeholder={t("settings.platformSearchPlaceholder")}
                  aria-label={t("settings.platformSearchLabel")}
                  className="pl-9"
                />
              </div>

              <div className="rounded-lg border border-border/70 bg-muted/20 p-3 text-sm text-muted-foreground">
                <p>{t("settings.platformVisibilityScope")}</p>
              </div>

              {platformVisibilityGroups.length === 0 ? (
                <div className="rounded-lg border border-dashed border-border px-4 py-6 text-sm text-muted-foreground">
                  {t("settings.platformSearchEmpty")}
                </div>
              ) : (
                platformVisibilityGroups.map((group) => (
                  <PlatformVisibilityGroup
                    key={group.category}
                    category={group.category}
                    title={group.title}
                    description={group.description}
                    agents={group.agents}
                    enabledCount={group.enabledCount}
                    totalCount={group.totalCount}
                    groupVisible={group.groupVisible}
                    isSearchActive={isPlatformVisibilitySearchActive}
                    normalizedQuery={normalizedPlatformVisibilityQuery}
                    onToggleGroup={(visible) =>
                      void handleToggleCategory(group.category, visible)
                    }
                    onToggleAgent={(agentId, enabled) =>
                      void handleTogglePlatformVisibility(agentId, enabled)
                    }
                  />
                ))
              )}
            </div>
          </CardContent>
        </Card>

        {/* ── Section 3: GitHub Import Auth ─────────────────────────────── */}
        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <KeyRound className="size-5 text-muted-foreground" />
              <div>
                <CardTitle>{t("settings.githubPatTitle")}</CardTitle>
                <CardDescription className="mt-1">
                  {t("settings.githubPatDesc")}
                </CardDescription>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div>
                <label htmlFor="github-pat" className="mb-1 block text-xs text-muted-foreground">
                  {t("settings.githubPatLabel")}
                </label>
                <Input
                  id="github-pat"
                  type="password"
                  placeholder="github_pat_..."
                  value={githubPatInput}
                  onChange={(event) => setGitHubPatInput(event.target.value)}
                  disabled={isLoadingGitHubPat || isSavingGitHubPat}
                />
              </div>

              <div className="rounded-lg border border-border/70 bg-muted/20 p-3 text-sm text-muted-foreground">
                <p>{t("settings.githubPatDirectOnly")}</p>
                <p className="mt-2">{t("settings.githubPatRateLimitHint")}</p>
                <p className="mt-2">{t("settings.githubPatAppWideHint")}</p>
              </div>

              {githubPatMessage ? (
                <p
                  className={githubPatMessage.type === "error" ? "text-sm text-destructive" : "text-sm text-emerald-600 dark:text-emerald-400"}
                  role="status"
                >
                  {githubPatMessage.text}
                </p>
              ) : null}

              <div className="flex flex-wrap items-center gap-2">
                <Button
                  onClick={handleSaveGitHubPat}
                  disabled={isLoadingGitHubPat || isSavingGitHubPat || !isGitHubPatDirty}
                >
                  {isSavingGitHubPat ? <Loader2 className="size-4 animate-spin" /> : null}
                  <span>{t("common.save")}</span>
                </Button>
                <Button
                  variant="outline"
                  onClick={handleClearGitHubPat}
                  disabled={isLoadingGitHubPat || isSavingGitHubPat || !githubPat}
                >
                  <span>{t("settings.githubPatClear")}</span>
                </Button>
                <Button
                  variant="outline"
                  onClick={handleTestGitHubPat}
                  disabled={
                    isLoadingGitHubPat ||
                    isSavingGitHubPat ||
                    isTestingGitHubPat ||
                    !githubPat ||
                    isGitHubPatDirty
                  }
                >
                  {isTestingGitHubPat ? <Loader2 className="size-4 animate-spin" /> : null}
                  <span>{t("settings.githubPatTest")}</span>
                </Button>
                {isLoadingGitHubPat ? (
                  <span className="text-xs text-muted-foreground">{t("settings.loading")}</span>
                ) : null}
              </div>
            </div>
          </CardContent>
        </Card>

        {/* ── Section 4: AI Provider ─────────────────────────────────────── */}
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
                  {AI_PROVIDERS.map((p) => (
                    <button
                      key={p.id}
                      type="button"
                      onClick={() => handleProviderChange(p.id)}
                      aria-pressed={aiProvider === p.id}
                      className={`min-h-8 px-3 py-1.5 rounded-md text-xs transition-colors cursor-pointer border md:min-h-7 ${aiProvider === p.id ? "bg-primary/15 border-primary text-foreground font-medium" : "border-border bg-background text-muted-foreground hover:border-primary/40 hover:bg-hover-bg/10"}`}
                    >
                      {lang === "zh" ? p.name.zh : p.name.en}
                    </button>
                  ))}
                </div>
              </div>
              {currentProvider && currentProvider.regions.length > 1 && (
                <div>
                  <div id="settings-ai-region-label" className="text-xs text-muted-foreground mb-2">
                    {t("settings.aiRegionLabel")}
                  </div>
                  <div className="flex gap-1.5" role="group" aria-labelledby="settings-ai-region-label">
                    {currentProvider.regions.map((r) => (
                      <button
                        key={r}
                        type="button"
                        onClick={() => setAiRegion(r)}
                        aria-pressed={aiRegion === r}
                        className={`min-h-8 px-3 py-1.5 rounded-md text-xs transition-colors cursor-pointer border md:min-h-7 ${aiRegion === r ? "bg-primary/15 border-primary text-foreground font-medium" : "border-border bg-background text-muted-foreground hover:border-primary/40"}`}
                      >
                        {lang === "zh" ? REGION_LABELS[r].zh : REGION_LABELS[r].en}
                      </button>
                    ))}
                  </div>
                </div>
              )}
              <div>
                <label htmlFor="settings-ai-api-key" className="text-xs text-muted-foreground mb-1 block">
                  {t("settings.aiApiKeyLabel")}
                </label>
                <Input
                  id="settings-ai-api-key"
                  type="password"
                  placeholder="sk-..."
                  value={aiApiKey}
                  onChange={(e) => setAiApiKey(e.target.value)}
                />
              </div>
              <div>
                <label htmlFor="settings-ai-model" className="text-xs text-muted-foreground mb-1 block">
                  {t("settings.aiModelLabel")}
                </label>
                <Input
                  id="settings-ai-model"
                  placeholder={t("settings.aiModelPlaceholder")}
                  value={aiModel}
                  onChange={(e) => setAiModel(e.target.value)}
                />
              </div>
              {aiProvider === "custom" && (
                <div>
                  <label htmlFor="settings-ai-api-url" className="text-xs text-muted-foreground mb-1 block">
                    {t("settings.aiApiUrlLabel")}
                  </label>
                  <Input
                    id="settings-ai-api-url"
                    placeholder={t("settings.aiApiUrlPlaceholder")}
                    value={aiCustomUrl}
                    onChange={(e) => setAiCustomUrl(e.target.value)}
                  />
                </div>
              )}
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
                      checked={aiTagStopOnRateLimit}
                      onCheckedChange={setAiTagStopOnRateLimit}
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
                      value={aiTagConcurrency}
                      onChange={(event) => setAiTagConcurrency(event.target.value)}
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
                      value={aiTagIntervalMs}
                      onChange={(event) => setAiTagIntervalMs(event.target.value)}
                    />
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                {resolvedUrl && (
                  <div className="text-xs text-muted-foreground bg-muted/30 rounded-md px-3 py-2 font-mono truncate flex-1 min-w-0">{resolvedUrl}</div>
                )}
                <Button variant="outline" size="sm" disabled={aiTesting || !aiApiKey || !resolvedUrl} onClick={async () => {
                  setAiTesting(true); setAiTestResult(null); setShowAiTestDetails(false);
                  try {
                    const result = await invoke<string>("explain_skill", { content: "Test connection. Reply with: OK" });
                    setAiTestResult({ ok: true, msg: result.slice(0, 60) });
                  } catch (err) {
                    const raw = String(err);
                    // Try to extract structured error from JSON-like error strings
                    let msg = raw;
                    let details: string | undefined;
                    const prefix = "API 请求失败: ";
                    if (raw.startsWith(prefix)) {
                      const after = raw.slice(prefix.length);
                      const nlIdx = after.indexOf("\n");
                      if (nlIdx > 0) {
                        msg = after.slice(nlIdx + 1);
                        details = after.slice(0, nlIdx);
                      } else {
                        msg = after;
                      }
                    }
                    setAiTestResult({ ok: false, msg, details });
                  }
                  finally { setAiTesting(false); }
                }} className="shrink-0">
                  {aiTesting ? <Loader2 className="size-3.5 animate-spin" /> : <Bot className="size-3.5" />}
                  <span>{t("settings.aiTestConnection")}</span>
                </Button>
              </div>
              {aiTestResult && (
                <div className={`text-xs rounded-md px-3 py-2 space-y-1.5 ${aiTestResult.ok ? "bg-green-500/10 text-green-700 dark:text-green-400" : "bg-destructive/10 text-destructive"}`}>
                  <p>{aiTestResult.ok ? "✓ " : "✕ "}{aiTestResult.msg}</p>
                  {!aiTestResult.ok && aiTestResult.details && (
                    <div>
                      <button
                        type="button"
                        className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                        onClick={() => setShowAiTestDetails((v) => !v)}
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
                  {!aiTestResult.ok && currentProvider && currentProvider.regions.length > 1 && (
                    <p className="text-muted-foreground">
                      {t("settings.aiTestRegionHint", {
                        region: lang === "zh"
                          ? REGION_LABELS[aiRegion]?.zh ?? aiRegion
                          : REGION_LABELS[aiRegion]?.en ?? aiRegion,
                      })}
                    </p>
                  )}
                </div>
              )}
            </div>
          </CardContent>
        </Card>

        {/* ── Section 5: Scan Directories (compact) ─────────────────────── */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle>{t("settings.scanDirs")}</CardTitle>
                <CardDescription className="mt-1">{t("settings.scanDirsDesc")}</CardDescription>
              </div>
              <Button variant="outline" size="sm" onClick={() => setIsAddDirOpen(true)} aria-label={t("settings.addDirAriaLabel")}>
                <Plus className="size-3.5" />
                <span>{t("settings.addDirectory")}</span>
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            {scanDirError && <p className="text-xs text-destructive mb-3" role="alert">{scanDirError}</p>}
            {isLoadingScanDirs ? (
              <div className="flex items-center gap-2 py-4 text-muted-foreground text-sm justify-center">
                <Loader2 className="size-4 animate-spin" />
                <span>{t("settings.loading")}</span>
              </div>
            ) : (() => {
              const customDirs = scanDirectories.filter((d) => !d.is_builtin);
              const builtinDirs = scanDirectories.filter((d) => d.is_builtin);
              return (
                <div className="space-y-3">
                  {/* Custom dirs first */}
                  {customDirs.length > 0 && (
                    <div className="rounded-lg border border-border overflow-hidden">
                      {customDirs.map((dir) => (
                        <ScanDirectoryRow key={dir.id} dir={dir} onRemove={() => handleRemoveDirectory(dir.path)} onToggle={(active) => handleToggleDirectory(dir.path, active)} isRemoving={removingDir === dir.path} />
                      ))}
                    </div>
                  )}
                  {customDirs.length === 0 && (
                    <p className="text-xs text-muted-foreground text-center py-2">{t("settings.noDirs")}</p>
                  )}
                  {/* Built-in dirs — collapsible, two-column */}
                  {builtinDirs.length > 0 && (
                    <div>
                      <button
                        onClick={() => setShowBuiltinDirs((v) => !v)}
                        className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                      >
                        <span>{showBuiltinDirs ? "▾" : "▸"}</span>
                        <span>{t("settings.builtinDir")} ({builtinDirs.length})</span>
                      </button>
                      {showBuiltinDirs && (
                        <div className="grid grid-cols-2 gap-1.5 mt-2">
                          {builtinDirs.map((dir) => (
                            <div key={dir.id} className="flex items-center gap-2 px-2.5 py-1.5 rounded-md bg-muted/30 text-xs text-muted-foreground truncate">
                              <FolderOpen className="size-3 shrink-0" />
                              <span className="truncate">{formatPathForDisplay(dir.path)}</span>
                              {dir.label && <span className="shrink-0 opacity-60">· {dir.label}</span>}
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })()}
          </CardContent>
        </Card>

        {/* ── Section 6: About ────────────────────────────────────────────── */}
        <Card>
          <CardHeader>
            <CardTitle>{t("settings.about")}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              <div className="flex items-center gap-3">
                <Info className="size-4 text-muted-foreground shrink-0" />
                <div>
                  <div className="text-xs text-muted-foreground">{t("settings.appVersion")}</div>
                  <div className="text-sm font-medium">SkillPort v{APP_VERSION}</div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <Database className="size-4 text-muted-foreground shrink-0" />
                <div>
                  <div className="text-xs text-muted-foreground">{t("settings.dbPath")}</div>
                  <div className="text-sm font-medium font-mono">{dbPathDisplay}</div>
                </div>
              </div>
              <div className="flex items-center gap-3">
                <Globe className="size-4 text-muted-foreground shrink-0" />
                <div>
                  <div className="text-xs text-muted-foreground">{t("settings.repoUrl")}</div>
                  <a
                    href={REPO_URL}
                    target="_blank"
                    rel="noreferrer"
                    className="text-sm font-medium text-primary hover:underline break-all"
                  >
                    {REPO_URL}
                  </a>
                </div>
              </div>
              {/* ── Flavor Switcher ──────────────────────────────────────── */}
              <div className="flex items-center gap-3">
                <Palette className="size-4 text-muted-foreground shrink-0" />
                <div className="flex-1">
                  <div className="text-xs text-muted-foreground mb-1.5">{t("settings.flavor")}</div>
                  <div className="flex gap-2">
                    {FLAVOR_ORDER.map((f) => (
                      <Button
                        key={f}
                        variant={flavor === f ? "default" : "outline"}
                        size="sm"
                        onClick={() => setFlavor(f)}
                        aria-pressed={flavor === f}
                      >
                        <span
                          className="inline-block size-2 rounded-full mr-1.5 shrink-0"
                          style={{ backgroundColor: FLAVOR_COLORS[f] }}
                        />
                        {t(`settings.${f}`)}
                      </Button>
                    ))}
                  </div>
                </div>
              </div>
              {/* ── Accent Color Picker ─────────────────────────────────── */}
              <div className="flex items-center gap-3">
                <Droplets className="size-4 text-muted-foreground shrink-0" />
                <div className="flex-1">
                  <div className="text-xs text-muted-foreground mb-1.5">{t("settings.accentColor")}</div>
                  <div className="flex flex-wrap gap-1.5" role="radiogroup" aria-label={t("settings.accentColor")}>
                    {ACCENT_NAMES.map((name) => {
                      const ctpVar = CTP_VAR_MAP[name];
                      const isActive = accent === name;
                      return (
                        <button
                          key={name}
                          type="button"
                          role="radio"
                          aria-checked={isActive}
                          aria-label={t(`settings.accent.${name}`)}
                          title={t(`settings.accent.${name}`)}
                          onClick={() => setAccent(name)}
                          className={`relative size-8 rounded-full transition-colors cursor-pointer md:size-6
                            ${isActive
                              ? "ring-2 ring-ring ring-offset-2 ring-offset-background scale-110"
                              : "ring-1 ring-border hover:scale-105 hover:ring-2 hover:ring-ring/50"
                            }`}
                          style={{ backgroundColor: `var(${ctpVar})` }}
                        />
                      );
                    })}
                  </div>
                </div>
              </div>
              {/* ── Language Switcher ──────────────────────────────────────── */}
              <div className="flex items-center gap-3">
                <Globe className="size-4 text-muted-foreground shrink-0" />
                <div className="flex-1">
                  <div className="text-xs text-muted-foreground mb-1.5">{t("settings.language")}</div>
                  <div className="flex gap-2">
                    <Button
                      variant={i18n.language === "zh" ? "default" : "outline"}
                      size="sm"
                      onClick={() => i18n.changeLanguage("zh")}
                      aria-pressed={i18n.language === "zh"}
                    >
                      {t("settings.chinese")}
                    </Button>
                    <Button
                      variant={i18n.language === "en" ? "default" : "outline"}
                      size="sm"
                      onClick={() => i18n.changeLanguage("en")}
                      aria-pressed={i18n.language === "en"}
                    >
                      {t("settings.english")}
                    </Button>
                  </div>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* ── Dialogs ────────────────────────────────────────────────────────── */}
      <AddDirectoryDialog
        open={isAddDirOpen}
        onOpenChange={setIsAddDirOpen}
        onAdd={handleAddDirectory}
      />

      <PlatformDialog
        open={isPlatformDialogOpen}
        onOpenChange={setIsPlatformDialogOpen}
        platform={editingPlatform}
        onAdd={handleAddPlatform}
        onEdit={handleEditPlatform}
      />
    </div>
  );
}
