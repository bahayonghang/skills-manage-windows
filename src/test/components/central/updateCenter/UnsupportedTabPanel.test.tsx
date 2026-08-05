import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { UpdateCenterTabContent } from "@/components/central/updateCenter/UpdateCenterTabContent";
import { emptyDecisionState } from "@/components/central/updateCenter/decisionAggregation";
import type { SkillUpdateInventory } from "@/types/skillUpdateInventory";

const inventory: SkillUpdateInventory = {
  updatable: [],
  remoteAdded: [],
  remoteMissing: [],
  unsupported: [
    { skillId: "local-only", reasonCode: "unknown_source" },
    { skillId: "missing-path", reasonCode: "missing_source_path" },
  ],
  platformDuplicates: [],
  deletedPlatformCopies: [],
  orphans: [],
  failedRepositories: [],
  generatedAt: "2026-08-03T00:00:00Z",
};

describe("Update Center unsupported tab", () => {
  it("shows each uncheckable skill with localized reason text", () => {
    render(
      <UpdateCenterTabContent
        tab="unsupported"
        inventory={inventory}
        decisions={emptyDecisionState()}
        handlers={{
          updateUpdatable: vi.fn(),
          toggleAllUpdatable: vi.fn(),
          updateAdded: vi.fn(),
          updateMissing: vi.fn(),
          updateDuplicates: vi.fn(),
          updateDeletedPlatformCopies: vi.fn(),
          retryRepositories: vi.fn(),
        }}
        existingSkillSources={new Map()}
        repositorySources={new Map()}
        retryingRepositoryIds={[]}
        actionsDisabled={false}
      />,
    );

    expect(screen.getByText("local-only")).toBeInTheDocument();
    expect(screen.getByText("missing-path")).toBeInTheDocument();
    expect(screen.getByText("没有可用于自动检查的远端来源。")).toBeInTheDocument();
    expect(screen.getByText("GitHub 来源缺少仓库内路径。")).toBeInTheDocument();
    expect(screen.queryByText(/Source is unknown/i)).not.toBeInTheDocument();
  });
});
