import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen } from "@testing-library/react";
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

    fireEvent.click(screen.getByTestId("central-toolbar-more"));
    fireEvent.click(await screen.findByText("从 GitHub 导入"));
    fireEvent.click(await screen.findByRole("button", { name: /检查导入内容/i }));

    expect(await screen.findByTestId("github-import-confirm-summary")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /返回预览修改/i })).toBeInTheDocument();
  });
});
