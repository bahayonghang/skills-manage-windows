import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { TFunction } from "i18next";

import { CentralSidebar } from "@/components/central/CentralSidebar";
import { FacetSection } from "@/components/central/FacetSection";
import type { FacetCounts } from "@/lib/centralFacetCounts";
import type { SkillRepositoryWithStats } from "@/types";
import zh from "@/i18n/locales/zh.json";

type TranslationObj = { [key: string]: TranslationObj | string };

function translate(key: string, options?: Record<string, unknown>): string {
  const parts = key.split(".");
  let result: TranslationObj | string = zh as unknown as TranslationObj;
  for (const part of parts) {
    if (result && typeof result === "object") {
      result = result[part];
    } else {
      return key;
    }
  }
  if (typeof result !== "string") return key;
  return result.replace(/\{\{(\w+)\}\}/g, (_match, name) => String(options?.[name] ?? ""));
}

const t = ((key: string, options?: Record<string, unknown>) =>
  translate(key, options)) as TFunction;

const repositories: SkillRepositoryWithStats[] = [
  {
    id: "github-openai-skills-main",
    name: "openai/skills",
    source_type: "github",
    owner: "openai",
    repo: "skills",
    branch: "main",
    url: "https://github.com/openai/skills",
    pinned: false,
    is_unknown: false,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    skill_count: 1,
    unknown_skill_count: 0,
  },
  {
    id: "local-unknown",
    name: "本地 / 未知来源",
    source_type: "local",
    owner: null,
    repo: null,
    branch: null,
    url: null,
    pinned: false,
    is_unknown: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    skill_count: 1,
    unknown_skill_count: 1,
  },
];

const searchableRepositories: SkillRepositoryWithStats[] = [
  repositories[0],
  {
    ...repositories[0],
    id: "github-openai-agents-main",
    name: "openai/agents",
    repo: "agents",
    url: "https://github.com/openai/agents",
    skill_count: 2,
  },
  {
    ...repositories[0],
    id: "github-anthropic-tools-main",
    name: "anthropic/tools",
    owner: "anthropic",
    repo: "tools",
    url: "https://github.com/anthropic/tools",
    skill_count: 3,
  },
  repositories[1],
];

const facetCounts: FacetCounts = {
  repositories: {
    all: 2,
    unassigned: 1,
    "github-openai-skills-main": 1,
    "local-unknown": 1,
  },
  tags: {
    "frontend-visual-design": 1,
    uncategorized: 1,
  },
  smartViews: {
    all: 2,
    uncategorized: 1,
    updates: 0,
    aiReview: 0,
  },
};

function renderSidebar(
  overrides: Partial<{
    repositories: SkillRepositoryWithStats[];
    selectedRepos: string[];
    onToggleRepositoryPin: (repository: SkillRepositoryWithStats) => void;
    onSyncNewSource: () => void;
  }> = {}
) {
  const handlers = {
    startResize: vi.fn(),
    handleResizeKeyDown: vi.fn(),
    onToggleRepo: vi.fn(),
    onToggleTag: vi.fn(),
    onClearAll: vi.fn(),
    onSelectSmartView: vi.fn(),
    onToggleRepositoryPin: vi.fn(),
  };

  // M4：sidebar 默认折叠为 48px rail。本套测试断言展开后的 facet 内容，
  // 通过 localStorage 提前 pin，使首次渲染即处于 expanded 状态。
  window.localStorage.setItem("central.sidebarPinned", "true");

  render(
    <CentralSidebar
      t={t}
      width={286}
      facetCounts={facetCounts}
      repositories={overrides.repositories ?? repositories}
      selectedRepos={overrides.selectedRepos ?? []}
      selectedTags={[]}
      savedViewsSlot={
        <>
          <FacetSection title="已保存视图" testId="central-saved-views">
            <div data-testid="saved-view-fixture">保存视图内容</div>
          </FacetSection>
          <FacetSection title="标签分组" testId="central-tag-groups-section">
            <div data-testid="tag-group-fixture">标签分组内容</div>
          </FacetSection>
        </>
      }
      {...handlers}
      onToggleRepositoryPin={overrides.onToggleRepositoryPin ?? handlers.onToggleRepositoryPin}
      onSyncNewSource={overrides.onSyncNewSource}
    />
  );

  return {
    sidebar: screen.getByTestId("central-sidebar-v2"),
    handlers,
  };
}

describe("CentralSidebar", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("renders a distinct global expand and collapse control", () => {
    const { sidebar } = renderSidebar();

    const toggle = within(sidebar).getByTestId("sidebar-bulk-expansion-toggle");

    expect(toggle).toHaveTextContent("收起全部分组");
    expect(toggle).toHaveAccessibleName("收起中央技能库侧栏全部分组");
    expect(toggle).toHaveClass("rounded-xl");
  });

  it("collapses and expands all top-level groups", () => {
    const { sidebar } = renderSidebar();
    const toggle = within(sidebar).getByTestId("sidebar-bulk-expansion-toggle");

    // 标签区块已迁至顶部筛选行：侧栏展开态不再渲染 sidebar-section-tags。
    expect(within(sidebar).queryByTestId("sidebar-section-tags")).not.toBeInTheDocument();

    expect(within(sidebar).getByTestId("central-saved-views-content")).not.toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-smart-content")).not.toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-repos-content")).not.toHaveClass("hidden");

    fireEvent.click(toggle);

    expect(within(sidebar).getByTestId("central-saved-views-content")).toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-smart-content")).toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-repos-content")).toHaveClass("hidden");

    fireEvent.click(toggle);

    expect(within(sidebar).getByTestId("central-saved-views-content")).not.toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-smart-content")).not.toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-repos-content")).not.toHaveClass(
      "hidden"
    );
  });

  it("applies the global signal to owner subgroups while preserving local toggles", () => {
    const { sidebar } = renderSidebar();
    const toggle = within(sidebar).getByTestId("sidebar-bulk-expansion-toggle");

    fireEvent.click(within(sidebar).getByRole("button", { name: "收起 openai" }));
    expect(within(sidebar).queryByTestId("repo-github-openai-skills-main")).not.toBeInTheDocument();

    fireEvent.click(toggle);
    fireEvent.click(toggle);

    expect(within(sidebar).getByTestId("repo-github-openai-skills-main")).toBeInTheDocument();

    fireEvent.click(within(sidebar).getByRole("button", { name: "收起 openai" }));
    expect(within(sidebar).queryByTestId("repo-github-openai-skills-main")).not.toBeInTheDocument();
    expect(within(sidebar).getByTestId("sidebar-section-repos-content")).not.toHaveClass("hidden");
  });

  it("shows pinned repository styling and does not select the row when pin is clicked", () => {
    const pinnedRepositories = repositories.map((repo) =>
      repo.id === "github-openai-skills-main" ? { ...repo, pinned: true } : repo
    );
    const onToggleRepositoryPin = vi.fn();
    const { sidebar, handlers } = renderSidebar({
      repositories: pinnedRepositories,
      onToggleRepositoryPin,
    });

    const repoButton = within(sidebar).getByTestId("repo-github-openai-skills-main");
    const repoRow = repoButton.closest("[data-pinned]");
    expect(repoRow).toHaveAttribute("data-pinned", "true");

    const pinButton = within(sidebar).getByTestId("repo-pin-github-openai-skills-main");
    expect(pinButton).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(pinButton);

    expect(onToggleRepositoryPin).toHaveBeenCalledWith(
      expect.objectContaining({ id: "github-openai-skills-main", pinned: true })
    );
    expect(handlers.onToggleRepo).not.toHaveBeenCalled();
  });

  it("renders the sync new source action with localized copy", () => {
    const onSyncNewSource = vi.fn();
    const { sidebar } = renderSidebar({ onSyncNewSource });

    const button = within(sidebar).getByTestId("sidebar-sync-new-source");
    expect(button).toHaveTextContent("同步新来源");
    expect(button).not.toHaveTextContent("central.v2.sidebarSyncNewSource");

    fireEvent.click(button);
    expect(onSyncNewSource).toHaveBeenCalled();
  });

  it("renders a localized repository search input in the repositories section", () => {
    const { sidebar } = renderSidebar();

    const input = within(sidebar).getByTestId("sidebar-repository-search");
    expect(input).toHaveAccessibleName("搜索仓库");
    expect(input).toHaveAttribute("placeholder", "搜索仓库...");
  });

  it("filters repositories by owner while keeping all matching owner rows", () => {
    const { sidebar } = renderSidebar({ repositories: searchableRepositories });
    const input = within(sidebar).getByTestId("sidebar-repository-search");

    fireEvent.change(input, { target: { value: "openai" } });

    expect(within(sidebar).getByTestId("owner-openai")).toBeInTheDocument();
    expect(within(sidebar).getByTestId("repo-github-openai-skills-main")).toBeInTheDocument();
    expect(within(sidebar).getByTestId("repo-github-openai-agents-main")).toBeInTheDocument();
    expect(within(sidebar).queryByTestId("owner-anthropic")).not.toBeInTheDocument();
    expect(within(sidebar).queryByTestId("repo-github-anthropic-tools-main")).not.toBeInTheDocument();
  });

  it("filters repositories by repo name and clears back to the full tree", () => {
    const { sidebar } = renderSidebar({ repositories: searchableRepositories });
    const input = within(sidebar).getByTestId("sidebar-repository-search");

    fireEvent.change(input, { target: { value: "agents" } });

    expect(input).toHaveValue("agents");
    expect(within(sidebar).getByTestId("owner-openai")).toBeInTheDocument();
    expect(within(sidebar).getByTestId("repo-github-openai-agents-main")).toBeInTheDocument();
    expect(within(sidebar).queryByTestId("repo-github-openai-skills-main")).not.toBeInTheDocument();
    expect(within(sidebar).queryByTestId("repo-github-anthropic-tools-main")).not.toBeInTheDocument();

    fireEvent.click(within(sidebar).getByTestId("sidebar-repository-search-clear"));

    expect(input).toHaveValue("");
    expect(within(sidebar).getByTestId("repo-github-openai-skills-main")).toBeInTheDocument();
    expect(within(sidebar).getByTestId("repo-github-openai-agents-main")).toBeInTheDocument();
    expect(within(sidebar).getByTestId("repo-github-anthropic-tools-main")).toBeInTheDocument();
  });

  it("shows a localized empty state when repository search has no matches", () => {
    const { sidebar } = renderSidebar({ repositories: searchableRepositories });
    const input = within(sidebar).getByTestId("sidebar-repository-search");

    fireEvent.change(input, { target: { value: "missing-repo" } });

    expect(within(sidebar).getByText("没有匹配的仓库")).toBeInTheDocument();
    expect(within(sidebar).queryByTestId("repo-github-openai-skills-main")).not.toBeInTheDocument();
  });

  it("selects a repository through the existing handler after filtering", () => {
    const { sidebar, handlers } = renderSidebar({ repositories: searchableRepositories });
    const input = within(sidebar).getByTestId("sidebar-repository-search");

    fireEvent.change(input, { target: { value: "agents" } });
    fireEvent.click(within(sidebar).getByTestId("repo-github-openai-agents-main"));

    expect(handlers.onToggleRepo).toHaveBeenCalledWith("github-openai-agents-main");
  });

  it("keeps saved views, tag groups, and selection footer visible during repo search", () => {
    const { sidebar } = renderSidebar({
      repositories: searchableRepositories,
      selectedRepos: ["github-openai-skills-main"],
    });
    const input = within(sidebar).getByTestId("sidebar-repository-search");

    fireEvent.change(input, { target: { value: "missing-repo" } });

    expect(within(sidebar).getByTestId("saved-view-fixture")).toBeInTheDocument();
    expect(within(sidebar).getByTestId("tag-group-fixture")).toBeInTheDocument();
    expect(within(sidebar).getByText("清空筛选")).toBeInTheDocument();
    expect(within(sidebar).getByText("已应用 1 个筛选")).toBeInTheDocument();
  });
});

