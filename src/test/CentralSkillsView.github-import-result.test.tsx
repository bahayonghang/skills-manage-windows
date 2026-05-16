import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, within } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const { renderCentralSkillsView } = S;

describe("CentralSkillsView GitHub import result", () => {
  beforeEach(S.resetCentralSkillsViewTestState);
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("offers post-import platform installation for imported skills", async () => {
    renderCentralSkillsView({
      marketplaceOverrides: {
        githubImport: {
          isPreviewLoading: false,
          isImporting: false,
          preview: null,
          importResult: {
            repo: {
              owner: "dorukardahan",
              repo: "twitterapi-io-skill",
              branch: "main",
              normalizedUrl: "https://github.com/dorukardahan/twitterapi-io-skill",
            },
            importedSkills: [
              {
                sourcePath: "twitterapi-io-skill/SKILL.md",
                originalSkillId: "frontend-design",
                importedSkillId: "frontend-design",
                skillName: "frontend-design",
                targetDirectory: "/Users/test/.skillsmanage/skills/frontend-design",
                resolution: "overwrite",
              },
            ],
            skippedSkills: [],
          },
          previewedRepoUrl: "https://github.com/dorukardahan/twitterapi-io-skill",
          error: null,
        },
      },
    });

    fireEvent.click(screen.getByTestId("central-toolbar-more"));
    fireEvent.click(await screen.findByText("从 GitHub 导入"));

    const dialog = await screen.findByRole("dialog", { name: /GitHub import wizard/i });
    expect(
      within(dialog).getByRole("button", { name: /^安装到平台$/i })
    ).toBeInTheDocument();
  });
});
