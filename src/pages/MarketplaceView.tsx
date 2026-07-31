import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import i18n from "@/i18n";

import { InstallDialog } from "@/components/central/InstallDialog";
import { MarketplaceShell } from "@/components/marketplace/MarketplaceShell";
import { formatGitHubImportToast } from "@/components/marketplace/githubImportWizardUtils";
import type { MarketplaceSkillDetail } from "@/components/marketplace/marketplaceSkillDetailTypes";
import type { OfficialPublisher, SkillTag } from "@/data/officialSources";
import { isTauriRuntime } from "@/lib/ipc";
import { useMarketplaceBindings } from "@/pages/marketplaceBindings";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import {
  findPreviewRegistryId,
  mapGitHubPreviewSkillToPreviewSkill,
  mapRegistrySkillToPreviewSkill,
  type MarketplacePreviewSkill,
  type MarketplacePreviewStatus,
  type MarketplaceTabId,
  useMarketplaceViewModel,
} from "@/pages/marketplaceViewModel";
import type { GitHubSkillImportSelection, SkillWithLinks } from "@/types";
import { useImportIntentBindings } from "@/stores/importIntentStore";

export function MarketplaceView() {
  const { t } = useTranslation();
  const lang = i18n.language;
  const {
    centralAgents,
    centralSkills,
    getNormalizedRegistryIdentity,
    getSkillsByAgent,
    githubImport,
    importGitHubRepoSkills,
    installGitHubPreviewSkill,
    installCentralSkill,
    installFromSkillsSh,
    installingIds,
    isSkillsShLoading,
    installSkill,
    loadCentralSkills,
    loadPreviewSkills,
    loadRegistries,
    platformAgents,
    previewGitHubRepoImport,
    previewGitHubRepoSkills,
    registries,
    resetGitHubImport,
    rescan,
    searchSkillsSh,
    skillsByAgent,
    skillsShError,
    skillsShQuery,
    skillsShResults,
  } = useMarketplaceBindings();

  const [activeTab, setActiveTab] = useState<MarketplaceTabId>("recommended");
  const [selectedTag, setSelectedTag] = useState<SkillTag | null>(null);
  const [recommendedSearch, setRecommendedSearch] = useState("");
  const [selectedPublisher, setSelectedPublisher] = useState<OfficialPublisher | null>(null);
  const [publisherSearch, setPublisherSearch] = useState("");

  const [previewRepo, setPreviewRepo] = useState<string | null>(null);
  const [previewSkills, setPreviewSkills] = useState<MarketplacePreviewSkill[]>([]);
  const [previewCache, setPreviewCache] = useState<Record<string, MarketplacePreviewSkill[]>>({});
  const [isPreviewLoading, setIsPreviewLoading] = useState(false);
  const [previewInstallingIds, setPreviewInstallingIds] = useState<Set<string>>(
    new Set()
  );
  const [detailSkill, setDetailSkill] = useState<MarketplaceSkillDetail | null>(null);
  const [centralInstallTarget, setCentralInstallTarget] = useState<SkillWithLinks | null>(null);
  const [isCentralInstallOpen, setIsCentralInstallOpen] = useState(false);
  const [previewStatus, setPreviewStatus] = useState<MarketplacePreviewStatus>({
    kind: "idle",
  });
  const {
    githubBranch,
    githubRepoUrl,
    isGitHubImportOpen,
    setGithubBranch,
    setGithubRepoUrl,
    setIsGitHubImportOpen,
  } = useImportIntentBindings();
  const detailTriggerRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    loadRegistries();
  }, [loadRegistries]);

  const viewModel = useMarketplaceViewModel({
    centralAgents,
    centralSkills,
    githubImportResult: githubImport.importResult,
    lang,
    platformAgents,
    publisherSearch,
    recommendedSearch,
    selectedTag,
  });

  async function handleInstallFromSource(skillId: string) {
    try {
      await installSkill(skillId);
      await rescan();
      setDetailSkill((current) =>
        current && current.id === skillId ? { ...current, installed: true } : current
      );
      toast.success(t("marketplace.installSuccess"));
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function handleInstallSkillsSh(source: string, skillId: string) {
    try {
      const importedSkillId = await installFromSkillsSh(source, skillId);
      await Promise.all([rescan(), loadCentralSkills()]);
      const refreshedCentralSkills = useCentralSkillsStore.getState().skills;
      const importedCentralSkill =
        refreshedCentralSkills.find((skill) => skill.id === importedSkillId) ??
        refreshedCentralSkills.find((skill) => skill.id === skillId || skill.name === skillId) ??
        null;
      setDetailSkill((current) =>
        current?.remoteKind === "skills_sh" &&
        current.source === source &&
        current.skillId === skillId
          ? { ...current, id: importedSkillId || current.id, installed: true }
          : current
      );
      if (importedCentralSkill) {
        setCentralInstallTarget(importedCentralSkill);
        setIsCentralInstallOpen(true);
      }
      toast.success(t("marketplace.skillsShCentralImportSuccess"));
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function handlePreviewRepo(
    repoFullName: string,
    repoUrl: string,
    options?: { forceRefresh?: boolean }
  ) {
    const forceRefresh = options?.forceRefresh ?? false;

    if (previewRepo === repoFullName && !forceRefresh) {
      setPreviewRepo(null);
      setPreviewStatus({ kind: "idle" });
      return;
    }

    if (!forceRefresh && Object.prototype.hasOwnProperty.call(previewCache, repoUrl)) {
      setPreviewRepo(repoFullName);
      setPreviewSkills(previewCache[repoUrl] ?? []);
      setPreviewStatus({ kind: "idle" });
      setIsPreviewLoading(false);
      return;
    }

    setPreviewRepo(repoFullName);
    setPreviewSkills([]);
    setPreviewStatus({ kind: "idle" });
    setIsPreviewLoading(true);
    try {
      if (!isTauriRuntime()) {
        setPreviewStatus({
          kind: "browser-fallback",
          title:
            lang === "zh"
              ? "浏览器模式下暂不支持预览"
              : "Preview unavailable in browser mode",
          detail:
            lang === "zh"
              ? "请在桌面应用中打开此流程，以浏览并安装仓库里的技能。"
              : "Open this flow in the desktop app to browse and install repository skills.",
        });
        return;
      }

      const registryId = findPreviewRegistryId({
        getNormalizedRegistryIdentity,
        registries,
        repoUrl,
      });

      if (registryId) {
        const skills = await loadPreviewSkills(registryId);
        if (skills.length > 0) {
          const nextPreviewSkills = skills.map(mapRegistrySkillToPreviewSkill);
          setPreviewSkills(nextPreviewSkills);
          setPreviewCache((current) => ({ ...current, [repoUrl]: nextPreviewSkills }));
          return;
        }
      }

      const preview = await previewGitHubRepoSkills(repoUrl);
      const nextPreviewSkills = preview.skills.map((skill) =>
        mapGitHubPreviewSkillToPreviewSkill(skill, repoUrl)
      );
      setPreviewSkills(nextPreviewSkills);
      setPreviewCache((current) => ({ ...current, [repoUrl]: nextPreviewSkills }));
    } catch (err) {
      setPreviewStatus({
        kind: "error",
        title: lang === "zh" ? "预览加载失败" : "Failed to load preview",
        detail: String(err),
      });
      toast.error(String(err));
    } finally {
      setIsPreviewLoading(false);
    }
  }

  async function handleInstallPreviewSkill(skill: MarketplacePreviewSkill) {
    setPreviewInstallingIds((prev) => new Set(prev).add(skill.name));
    try {
      if (skill.sourceKind === "registry") {
        await installSkill(skill.registrySkillId);
      } else {
        await installGitHubPreviewSkill(skill.repoUrl, skill.sourcePath);
      }

      await rescan();
      toast.success(t("marketplace.installSuccess"));
    } catch (err) {
      toast.error(formatGitHubImportToast(err, t));
    } finally {
      setPreviewInstallingIds((prev) => {
        const next = new Set(prev);
        next.delete(skill.name);
        return next;
      });
    }
  }

  function openDetailSkill(skill: MarketplaceSkillDetail, trigger?: EventTarget | null) {
    if (trigger instanceof HTMLElement) {
      detailTriggerRef.current = trigger;
    }
    setDetailSkill(skill);
  }

  async function handleGitHubPreview() {
    try {
      return await previewGitHubRepoImport(githubRepoUrl, githubBranch);
    } catch {
      return null;
    }
  }

  async function handleGitHubImport(selections: GitHubSkillImportSelection[]) {
    try {
      const result = await importGitHubRepoSkills(githubRepoUrl, selections);
      await Promise.all([rescan(), loadRegistries(), loadCentralSkills()]);
      toast.success(
        lang === "zh" ? "GitHub 仓库技能已导入中央技能库" : "GitHub repo skills imported to Central"
      );
      return result;
    } catch (err) {
      // Preview snapshot lifecycle failures use a coded envelope; localize them
      // so the toast asks the user to preview the repository again.
      toast.error(formatGitHubImportToast(err, t));
      throw err;
    }
  }

  async function handleInstallImportedSkill(
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null
  ) {
    const result = await installCentralSkill(skillId, agentIds, method, projectPath);
    await Promise.all([
      rescan(),
      loadCentralSkills(),
      ...agentIds.map((agentId) => getSkillsByAgent(agentId)),
    ]);
    return result;
  }

  async function handleAfterImportSuccess() {
    const agentIds = Object.keys(skillsByAgent);
    if (agentIds.length === 0) return;
    await Promise.all(agentIds.map((agentId) => getSkillsByAgent(agentId)));
  }

  function handleResetGitHubImport() {
    resetGitHubImport();
    setGithubBranch("");
    setGithubRepoUrl("");
  }

  return (
    <>
    <MarketplaceShell
      activeTab={activeTab}
      detailSkill={detailSkill}
      detailTriggerRef={detailTriggerRef}
      githubImport={githubImport}
      githubBranch={githubBranch}
      githubRepoUrl={githubRepoUrl}
      installingIds={installingIds}
      isGitHubImportOpen={isGitHubImportOpen}
      isPreviewLoading={isPreviewLoading}
      isSkillsShLoading={isSkillsShLoading}
      lang={lang}
      onAfterImportSuccess={handleAfterImportSuccess}
      onGitHubImport={handleGitHubImport}
      onGitHubPreview={handleGitHubPreview}
      onInstallFromSource={handleInstallFromSource}
      onInstallImportedSkill={handleInstallImportedSkill}
      onInstallPreviewSkill={handleInstallPreviewSkill}
      onInstallSkillsSh={handleInstallSkillsSh}
      onOpenDetailSkill={openDetailSkill}
      onPreviewRepo={handlePreviewRepo}
      onResetGitHubImport={handleResetGitHubImport}
      onSearchSkillsSh={searchSkillsSh}
      previewInstallingIds={previewInstallingIds}
      previewRepo={previewRepo}
      previewSkills={previewSkills}
      previewStatus={previewStatus}
      publisherSearch={publisherSearch}
      recommendedSearch={recommendedSearch}
      selectedPublisher={selectedPublisher}
      selectedTag={selectedTag}
      setActiveTab={setActiveTab}
      setDetailSkill={setDetailSkill}
      setGithubBranch={setGithubBranch}
      setGithubRepoUrl={setGithubRepoUrl}
      setIsGitHubImportOpen={setIsGitHubImportOpen}
      setPublisherSearch={setPublisherSearch}
      setRecommendedSearch={setRecommendedSearch}
      setSelectedPublisher={setSelectedPublisher}
      setSelectedTag={setSelectedTag}
      skillsShError={skillsShError}
      skillsShQuery={skillsShQuery}
      skillsShResults={skillsShResults}
      viewModel={viewModel}
    />
    <InstallDialog
      open={isCentralInstallOpen}
      onOpenChange={setIsCentralInstallOpen}
      skill={centralInstallTarget}
      agents={viewModel.availableInstallAgents}
      onInstall={handleInstallImportedSkill}
    />
    </>
  );
}
