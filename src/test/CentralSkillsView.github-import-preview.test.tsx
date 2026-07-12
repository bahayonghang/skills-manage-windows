import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, within } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const { renderCentralSkillsView } = S;

describe("CentralSkillsView GitHub import preview", () => {
  beforeEach(S.resetCentralSkillsViewTestState);
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("shows the redesigned github confirm summary in the shared wizard", async () => {
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
                conflict: {
                  existingSkillId: "frontend-design",
                  existingName: "frontend-design",
                  existingCanonicalPath: "/Users/test/.skillsmanage/skills/frontend-design",
                  proposedSkillId: "frontend-design",
                  proposedName: "frontend-design",
                },
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
      },
    });

    fireEvent.click(screen.getByTestId("central-github-import-open"));
    const dialog = await screen.findByRole("dialog", {
      name: /GitHub import wizard/i,
    });
    const wizard = within(dialog);

    fireEvent.click(
      await wizard.findByRole("button", { name: /检查导入内容|Review import/i }),
    );

    expect(
      await wizard.findByTestId("github-import-confirm-summary"),
    ).toBeInTheDocument();
    expect(
      wizard.getByRole("button", { name: /返回预览修改|Back to preview/i }),
    ).toBeInTheDocument();
  });
});
