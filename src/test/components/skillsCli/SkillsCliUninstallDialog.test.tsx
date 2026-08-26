import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import { SkillsCliUninstallDialog } from "@/components/skillsCli/SkillsCliUninstallDialog";
import { ipcFixtureError } from "@/lib/ipc/errors";
import type { SkillsCliRemovePlan } from "@/types";

const { showSkillsCliActionToast } = vi.hoisted(() => ({
  showSkillsCliActionToast: vi.fn(),
}));

vi.mock("@/components/skillsCli/skillsCliActionToast", () => ({
  showSkillsCliActionToast,
  SKILLS_CLI_ACTION_TOAST_ID: "skills-cli-action",
  SKILLS_CLI_ACTION_TOAST_DURATION_MS: 2800,
}));

const ASYNC_UI_TIMEOUT_MS = 5_000;

function plan(overrides: Partial<SkillsCliRemovePlan> = {}): SkillsCliRemovePlan {
  return {
    skillName: "demo-skill",
    ownedCanonical: true,
    managedPlacements: [{ agentId: "cursor", displayName: "Cursor" }],
    retainedDirectCopies: [],
    conflicts: [],
    confirmable: true,
    ...overrides,
  };
}

describe("SkillsCliUninstallDialog", () => {
  beforeEach(() => {
    showSkillsCliActionToast.mockClear();
  });

  it("renders owned, managed, retained, and conflict buckets from the backend plan", async () => {
    render(
      <SkillsCliUninstallDialog
        open
        skillNames={["demo-skill"]}
        isMutating={false}
        onOpenChange={vi.fn()}
        previewRemoveGlobal={async () =>
          plan({
            retainedDirectCopies: [{ agentId: "amp", displayName: "Amp" }],
            conflicts: [
              {
                agentId: "codex",
                displayName: "Codex",
                reasonCode: "skills_cli.placement_conflict",
              },
            ],
            confirmable: false,
          })
        }
        removeGlobalBatch={vi.fn()}
        onRemoved={vi.fn()}
      />,
    );
    const dialog = await screen.findByRole(
      "dialog",
      { name: /卸载 demo-skill/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const surface = within(dialog);
    expect(
      await surface.findByTestId("skills-cli-uninstall-owned", {}, { timeout: ASYNC_UI_TIMEOUT_MS }),
    ).toHaveTextContent("将删除 1 个受管文件夹");
    expect(surface.getByTestId("skills-cli-uninstall-managed")).toHaveTextContent(
      "将删除 1 个受管链接",
    );
    expect(surface.getByTestId("skills-cli-uninstall-retained")).toHaveTextContent(
      "demo-skill（Amp）",
    );
    expect(surface.getByTestId("skills-cli-uninstall-conflicts")).toHaveTextContent(
      "该平台路径与受管技能冲突",
    );
    expect(surface.getByRole("button", { name: "卸载" })).toBeDisabled();
    expect(dialog.textContent).not.toMatch(/--keep-links|--force|skills remove/);
    expect(dialog.textContent).not.toContain("/tmp/");
  });

  it("keeps failed names and uses a destructive toast on partial remove", async () => {
    const onRemoved = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <SkillsCliUninstallDialog
        open
        skillNames={["keep", "drop"]}
        isMutating={false}
        onOpenChange={onOpenChange}
        previewRemoveGlobal={async (name) => plan({ skillName: name })}
        removeGlobalBatch={async () => ({
          succeeded: [{ skillName: "drop" }],
          failed: [{ skillName: "keep", errorCode: "skills_cli.cli_unavailable" }],
          skipped: [],
        })}
        onRemoved={onRemoved}
      />,
    );
    const dialog = await screen.findByRole(
      "dialog",
      { name: /卸载 2 个技能/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const confirm = await within(dialog).findByRole("button", { name: "卸载" });
    await waitFor(() => expect(confirm).toBeEnabled(), {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    fireEvent.click(confirm);
    expect(await screen.findByText(/keep：无法执行 Skills CLI 软件包/)).toBeInTheDocument();
    expect(onRemoved).toHaveBeenCalledWith(["drop"]);
    expect(onOpenChange).not.toHaveBeenCalledWith(false);
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "destructiveError",
      message: "已卸载 1，失败 1。",
    });
  });

  it("shows a coded preview error and clears busy when preview throws", async () => {
    render(
      <SkillsCliUninstallDialog
        open
        skillNames={["demo-skill"]}
        isMutating={false}
        onOpenChange={vi.fn()}
        previewRemoveGlobal={async () => {
          throw ipcFixtureError(
            "skills_cli.cli_unavailable",
            "The Skills CLI package could not be executed.",
          );
        }}
        removeGlobalBatch={vi.fn()}
        onRemoved={vi.fn()}
      />,
    );
    const dialog = await screen.findByRole(
      "dialog",
      { name: /卸载 demo-skill/ },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const surface = within(dialog);
    expect(
      await surface.findByRole("alert", {}, { timeout: ASYNC_UI_TIMEOUT_MS }),
    ).toHaveTextContent("无法执行 Skills CLI 软件包");
    expect(surface.getByRole("button", { name: "卸载" })).toBeDisabled();
    expect(dialog).toHaveAttribute("aria-busy", "false");
  });
});
