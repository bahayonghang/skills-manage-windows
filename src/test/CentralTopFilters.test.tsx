import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { TFunction } from "i18next";

import { CentralTopFilters } from "../components/central/CentralTopFilters";
import type { FacetCounts } from "../lib/centralFacetCounts";
import type { SkillTag } from "../types";

const t = ((key: string, options?: Record<string, unknown>) => {
  if (options && "count" in options) return `${key}:${options.count}`;
  return key;
}) as TFunction;

const tags: SkillTag[] = [
  {
    id: "t1",
    name: "frontend",
    is_builtin: false,
    created_at: "",
    updated_at: "",
  },
  {
    id: "uncategorized",
    name: "未分类",
    is_builtin: true,
    created_at: "",
    updated_at: "",
  },
];

const facetCounts: FacetCounts = {
  repositories: { all: 1 },
  tags: { t1: 3, uncategorized: 1 },
  smartViews: { all: 1, uncategorized: 0, updates: 0, aiReview: 0 },
};

function renderFilters(
  overrides: Partial<Parameters<typeof CentralTopFilters>[0]> = {},
) {
  const props = {
    t,
    tags,
    selectedTagIds: [] as string[],
    onToggleTag: vi.fn(),
    facetCounts,
    activeSource: null as "github" | "local" | "manual" | null,
    onToggleSource: vi.fn(),
    tagGroupsSlot: null,
    ...overrides,
  };
  render(<CentralTopFilters {...props} />);
  return props;
}

describe("CentralTopFilters", () => {
  it("点击标签 pill 切换该标签", async () => {
    const onToggleTag = vi.fn();
    renderFilters({ onToggleTag });
    await userEvent.click(screen.getByTestId("top-filter-tag-t1"));
    expect(onToggleTag).toHaveBeenCalledWith("t1");
  });

  it("不把 uncategorized 当作普通标签 pill 渲染", () => {
    renderFilters();
    expect(
      screen.queryByTestId("top-filter-tag-uncategorized"),
    ).not.toBeInTheDocument();
  });

  it("隐藏空内置标签并显示已有技能的内置标签", () => {
    renderFilters({
      tags: [
        ...tags,
        {
          id: "frontend-development",
          name: "前端开发",
          is_builtin: true,
          created_at: "",
          updated_at: "",
        },
        {
          id: "backend-development",
          name: "后端开发",
          is_builtin: true,
          created_at: "",
          updated_at: "",
        },
      ],
      facetCounts: {
        ...facetCounts,
        tags: {
          ...facetCounts.tags,
          "frontend-development": 0,
          "backend-development": 2,
        },
      },
    });

    expect(
      screen.queryByTestId("top-filter-tag-frontend-development"),
    ).not.toBeInTheDocument();
    expect(
      screen.getByTestId("top-filter-tag-backend-development"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("top-filter-tag-t1")).toBeInTheDocument();
  });

  it("点击来源 pill 切换来源", async () => {
    const onToggleSource = vi.fn();
    renderFilters({ onToggleSource });
    await userEvent.click(screen.getByTestId("top-filter-source-github"));
    expect(onToggleSource).toHaveBeenCalledWith("github");
  });

  it("点击已选来源的「全部」清空来源", async () => {
    const onToggleSource = vi.fn();
    renderFilters({ onToggleSource, activeSource: "github" });
    await userEvent.click(screen.getByTestId("top-filter-source-all"));
    expect(onToggleSource).toHaveBeenCalledWith(null);
  });
});
