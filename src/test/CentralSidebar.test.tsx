import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { TFunction } from "i18next";

import { CentralSidebar } from "../components/central/CentralSidebar";
import { FacetSection } from "../components/central/FacetSection";
import type { FacetCounts } from "../lib/centralFacetCounts";
import type { SkillRepositoryWithStats, SkillTag, TagGroup } from "../types";
import zh from "../i18n/locales/zh.json";

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
    is_unknown: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    skill_count: 1,
    unknown_skill_count: 1,
  },
];

const tagGroups: TagGroup[] = [
  {
    id: "group-design",
    name: "设计类",
    color: "#38bdf8",
    sort_order: 0,
    is_builtin: false,
    created_at: "2026-05-11T00:00:00Z",
    updated_at: "2026-05-11T00:00:00Z",
  },
];

const tags: SkillTag[] = [
  {
    id: "frontend-visual-design",
    name: "前端与视觉设计",
    group_id: "group-design",
    is_builtin: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
  },
  {
    id: "uncategorized",
    name: "未分类",
    is_builtin: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
  },
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

function renderSidebar() {
  const handlers = {
    startResize: vi.fn(),
    handleResizeKeyDown: vi.fn(),
    onToggleRepo: vi.fn(),
    onToggleTag: vi.fn(),
    onClearAll: vi.fn(),
    onSelectSmartView: vi.fn(),
  };

  // M4：sidebar 默认折叠为 48px rail。本套测试断言展开后的 facet 内容，
  // 通过 localStorage 提前 pin，使首次渲染即处于 expanded 状态。
  window.localStorage.setItem("central.sidebarPinned", "true");

  render(
    <CentralSidebar
      t={t}
      width={286}
      facetCounts={facetCounts}
      repositories={repositories}
      tags={tags}
      tagGroups={tagGroups}
      selectedRepos={[]}
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
    expect(toggle).toHaveClass("rounded-2xl");
  });

  it("collapses and expands all top-level groups", () => {
    const { sidebar } = renderSidebar();
    const toggle = within(sidebar).getByTestId("sidebar-bulk-expansion-toggle");

    expect(within(sidebar).getByTestId("central-saved-views-content")).not.toHaveClass("hidden");
    expect(within(sidebar).getByTestId("central-tag-groups-section-content")).not.toHaveClass(
      "hidden"
    );
    expect(within(sidebar).getByTestId("sidebar-section-smart-content")).not.toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-repos-content")).not.toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-tags-content")).not.toHaveClass("hidden");

    fireEvent.click(toggle);

    expect(within(sidebar).getByTestId("central-saved-views-content")).toHaveClass("hidden");
    expect(within(sidebar).getByTestId("central-tag-groups-section-content")).toHaveClass(
      "hidden"
    );
    expect(within(sidebar).getByTestId("sidebar-section-smart-content")).toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-repos-content")).toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-tags-content")).toHaveClass("hidden");

    fireEvent.click(toggle);

    expect(within(sidebar).getByTestId("central-saved-views-content")).not.toHaveClass("hidden");
    expect(within(sidebar).getByTestId("central-tag-groups-section-content")).not.toHaveClass(
      "hidden"
    );
    expect(within(sidebar).getByTestId("sidebar-section-smart-content")).not.toHaveClass("hidden");
    expect(within(sidebar).getByTestId("sidebar-section-repos-content")).not.toHaveClass(
      "hidden"
    );
    expect(within(sidebar).getByTestId("sidebar-section-tags-content")).not.toHaveClass("hidden");
  });

  it("applies the global signal to owner and tag subgroups while preserving local toggles", () => {
    const { sidebar } = renderSidebar();
    const toggle = within(sidebar).getByTestId("sidebar-bulk-expansion-toggle");

    fireEvent.click(within(sidebar).getByRole("button", { name: "收起 openai" }));
    expect(within(sidebar).queryByTestId("repo-github-openai-skills-main")).not.toBeInTheDocument();
    expect(within(sidebar).getByTestId("tag-frontend-visual-design")).toBeInTheDocument();

    fireEvent.click(within(sidebar).getByTestId("tag-group-group-design"));
    expect(within(sidebar).queryByTestId("tag-frontend-visual-design")).not.toBeInTheDocument();

    fireEvent.click(toggle);
    fireEvent.click(toggle);

    expect(within(sidebar).getByTestId("repo-github-openai-skills-main")).toBeInTheDocument();
    expect(within(sidebar).getByTestId("tag-frontend-visual-design")).toBeInTheDocument();

    fireEvent.click(within(sidebar).getByRole("button", { name: "收起 openai" }));
    expect(within(sidebar).queryByTestId("repo-github-openai-skills-main")).not.toBeInTheDocument();
    expect(within(sidebar).getByTestId("sidebar-section-tags-content")).not.toHaveClass("hidden");
  });
});

