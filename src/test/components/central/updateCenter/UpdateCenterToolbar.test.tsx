import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { UpdateCenterToolbar } from "@/components/central/updateCenter/UpdateCenterToolbar";
import type { UpdateCenterTab } from "@/stores/updateCenterStore";

const TAB_ORDER: readonly UpdateCenterTab[] = [
  "updatable",
  "added",
  "missing",
  "failed",
  "duplicates",
  "deletedPlatformCopies",
  "orphans",
];

const EMPTY_COUNTS: Record<UpdateCenterTab, number> = {
  updatable: 0,
  added: 0,
  missing: 0,
  failed: 0,
  duplicates: 0,
  deletedPlatformCopies: 0,
  orphans: 0,
};

describe("UpdateCenterToolbar", () => {
  it("renders an explicit refresh mode control", () => {
    const onRefreshModeChange = vi.fn();

    render(
      <UpdateCenterToolbar
        scopeKind="all"
        onScopeKindChange={vi.fn()}
        refreshMode="regular"
        onRefreshModeChange={onRefreshModeChange}
        isRefreshing={false}
        onRefresh={vi.fn()}
        lastRefreshedAt={null}
        activeTab="updatable"
        onTabChange={vi.fn()}
        tabOrder={TAB_ORDER}
        counts={EMPTY_COUNTS}
        scopeEnabled={{ all: true, repositories: false, platform: false, skills: false }}
      />,
    );

    const modeSelect = screen.getByLabelText("刷新模式");
    expect(modeSelect).toHaveValue("regular");

    fireEvent.change(modeSelect, { target: { value: "sync" } });

    expect(onRefreshModeChange).toHaveBeenCalledWith("sync");
  });
});
