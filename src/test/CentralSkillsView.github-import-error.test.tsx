import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const { ASYNC_UI_TIMEOUT_MS, toast, renderCentralSkillsView } = S;

describe("CentralSkillsView GitHub import error", () => {
  beforeEach(S.resetCentralSkillsViewTestState);
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("reports github import failures as import errors", async () => {
    const mockImport = vi
      .fn()
      .mockRejectedValue("SSH password for target 'dckj' is not available.");
    renderCentralSkillsView({
      marketplaceOverrides: {
        githubImport: {
          isPreviewLoading: false,
          isImporting: false,
          preview: {
            repo: {
              owner: "anthropics",
              repo: "skills",
              branch: "main",
              normalizedUrl: "https://github.com/anthropics/skills",
            },
            skills: [
              {
                sourcePath: "skills/first/SKILL.md",
                skillId: "frontend-design",
                skillName: "frontend-design",
                description: "First imported skill",
                rootDirectory: "skills",
                skillDirectoryName: "first",
                downloadUrl: "https://example.com/first",
                conflict: null,
              },
            ],
            importResult: null,
            previewedRepoUrl: "https://github.com/anthropics/skills",
            error: null,
          },
          importResult: null,
          previewedRepoUrl: "https://github.com/anthropics/skills",
          error: null,
        },
        importGitHubRepoSkills: mockImport,
      },
    });

    fireEvent.click(screen.getByTestId("central-github-import-open"));
    const dialog = await screen.findByRole(
      "dialog",
      { name: /GitHub import wizard/i },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const wizard = within(dialog);

    fireEvent.click(
      await wizard.findByRole(
        "button",
        { name: /检查导入内容|Review import/i },
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    );
    fireEvent.click(
      await wizard.findByRole(
        "button",
        { name: /^导入$|^Import$/i },
        { timeout: ASYNC_UI_TIMEOUT_MS },
      ),
    );

    await waitFor(
      () => {
        expect(toast.error).toHaveBeenCalledWith(
          expect.stringContaining("GitHub"),
        );
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(toast.error).not.toHaveBeenCalledWith(
      expect.stringContaining("Install failed"),
    );
  });
});
