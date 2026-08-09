import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi, type Mock } from "vitest";

import { UpdateCenterTabContent } from "@/components/central/updateCenter/UpdateCenterTabContent";
import { emptyDecisionState } from "@/components/central/updateCenter/decisionAggregation";
import type {
  FailedRepository,
  SkillRefreshMode,
  SkillUpdateInventory,
} from "@/types/skillUpdateInventory";

function inventoryWith(
  failedRepositories: FailedRepository[],
): SkillUpdateInventory {
  return {
    updatable: [],
    remoteAdded: [],
    remoteMissing: [],
    unsupported: [],
    platformDuplicates: [],
    deletedPlatformCopies: [],
    orphans: [],
    failedRepositories,
    generatedAt: "2026-08-05T00:00:00Z",
  };
}

type RetryHandler = (repositoryIds: string[], mode?: SkillRefreshMode) => void;

function renderFailedTab(
  inventory: SkillUpdateInventory,
  overrides: {
    retryRepositories?: Mock<RetryHandler>;
    retryingRepositoryIds?: string[];
    actionsDisabled?: boolean;
  } = {},
) {
  const retryRepositories =
    overrides.retryRepositories ?? vi.fn<RetryHandler>();
  render(
    <UpdateCenterTabContent
      tab="failed"
      inventory={inventory}
      decisions={emptyDecisionState()}
      handlers={{
        updateUpdatable: vi.fn(),
        toggleAllUpdatable: vi.fn(),
        updateAdded: vi.fn(),
        updateMissing: vi.fn(),
        updateDuplicates: vi.fn(),
        updateDeletedPlatformCopies: vi.fn(),
        retryRepositories,
      }}
      existingSkillSources={new Map()}
      repositorySources={new Map()}
      retryingRepositoryIds={overrides.retryingRepositoryIds ?? []}
      actionsDisabled={overrides.actionsDisabled ?? false}
    />,
  );
  return { retryRepositories };
}

const snapshotFailure: FailedRepository = {
  repositoryId: "repo-a",
  error: "This repository could not be checked.",
  errorCode: "central_updates.repository_check_failed",
  retry: "retryable",
};

const decisionFailure: FailedRepository = {
  repositoryId: "repo-b",
  error: "The tracked source path no longer contains a skill.",
  errorCode: "central_updates.skill_source_missing",
  retry: "decision_required",
};

describe("Update Center failed tab", () => {
  it("retries a single repository without a mode override", async () => {
    const { retryRepositories } = renderFailedTab(
      inventoryWith([snapshotFailure]),
    );

    await userEvent.click(screen.getByRole("button", { name: "重试" }));

    expect(retryRepositories).toHaveBeenCalledWith(["repo-a"], undefined);
  });

  it("re-checks a decision-required row in incremental mode", async () => {
    const { retryRepositories } = renderFailedTab(
      inventoryWith([decisionFailure]),
    );

    await userEvent.click(
      screen.getByRole("button", { name: "用增量和删减模式重查" }),
    );

    expect(retryRepositories).toHaveBeenCalledWith(["repo-b"], "sync");
  });

  it("counts only retryable rows in the batch action", async () => {
    const { retryRepositories } = renderFailedTab(
      inventoryWith([snapshotFailure, decisionFailure]),
    );

    const batch = screen.getByRole("button", { name: "重试全部失败项（1）" });
    await userEvent.click(batch);

    expect(retryRepositories).toHaveBeenCalledWith(["repo-a"]);
  });

  it("disables the batch action when nothing is retryable", () => {
    renderFailedTab(inventoryWith([decisionFailure]));

    expect(
      screen.getByRole("button", { name: "重试全部失败项（0）" }),
    ).toBeDisabled();
  });

  it("offers no action for rows stored before the classification existed", () => {
    renderFailedTab(
      inventoryWith([
        {
          repositoryId: "repo-c",
          error: "This repository could not be checked.",
          errorCode: "central_updates.repository_check_failed",
        },
      ]),
    );

    expect(screen.queryByRole("button", { name: "重试" })).toBeNull();
    expect(
      screen.getByRole("button", { name: "重试全部失败项（0）" }),
    ).toBeDisabled();
  });

  it("disables the row action while its retry is in flight", () => {
    renderFailedTab(inventoryWith([snapshotFailure]), {
      retryingRepositoryIds: ["repo-a"],
    });

    expect(screen.getByRole("button", { name: "重试" })).toBeDisabled();
  });

  it("disables every action while another update job runs", () => {
    renderFailedTab(inventoryWith([snapshotFailure]), {
      actionsDisabled: true,
    });

    expect(screen.getByRole("button", { name: "重试" })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "重试全部失败项（1）" }),
    ).toBeDisabled();
  });

  it("shows the localized reason instead of the backend sentence", () => {
    renderFailedTab(inventoryWith([decisionFailure]));

    expect(
      screen.getByText(
        "该技能记录的源路径已不存在，且未能唯一定位到新位置。用增量和删减模式重查后可选择保留或删除。",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/tracked source path no longer/i),
    ).not.toBeInTheDocument();
  });
});
