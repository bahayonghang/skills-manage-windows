import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import * as S from "./marketplaceViewTestSupport";

const {
  makePreview,
  renderMarketplaceView,
  storeState,
  useTargetStore,
} = S;

describe("MarketplaceView GitHub preview workspace", () => {
  beforeEach(S.resetMarketplaceViewTestState);
  afterEach(S.cleanupMarketplaceViewTestState);

  it("renders the shared github preview workspace when preview data already exists", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.curated/openai-docs",
        skillId: "openai-docs",
        skillName: "OpenAI Docs",
        description: "OpenAI docs skill description",
        rootDirectory: "skills/.curated",
        skillDirectoryName: "openai-docs",
        downloadUrl: "https://example.com/openai-docs/SKILL.md",
        conflict: null,
      },
    ]);

    renderMarketplaceView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(await screen.findByTestId("github-import-preview-workspace")).toBeInTheDocument();
    expect(screen.getByTestId("github-import-repo-toolbar")).toBeInTheDocument();
    expect(screen.getByTestId("github-import-summary-list")).toBeInTheDocument();
    expect(screen.getByTestId("github-import-detail-pane")).toBeInTheDocument();
  });

  it("shows the remote workspace hint for SSH previews", async () => {
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        { id: "ssh-demo", kind: "ssh", label: "dckj", isActive: true },
      ],
      activeTarget: { id: "ssh-demo", kind: "ssh", label: "dckj", isActive: true },
    });
    storeState.githubImport.preview = makePreview(
      [
        {
          sourcePath: "skills/.curated/openai-docs",
          skillId: "openai-docs",
          skillName: "OpenAI Docs",
          description: "OpenAI docs skill description",
          rootDirectory: "skills/.curated",
          skillDirectoryName: "openai-docs",
          downloadUrl: "https://example.com/openai-docs/SKILL.md",
          conflict: null,
        },
      ],
      "github-preview-ssh",
    );

    renderMarketplaceView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(await screen.findByTestId("github-import-remote-workspace-hint")).toHaveTextContent(
      /Preview workspace is on the active SSH target|预览工作区位于当前 SSH 目标/i,
    );
  });

  it("does not show the remote workspace hint for local previews", async () => {
    storeState.githubImport.preview = makePreview(
      [
        {
          sourcePath: "skills/.curated/openai-docs",
          skillId: "openai-docs",
          skillName: "OpenAI Docs",
          description: "OpenAI docs skill description",
          rootDirectory: "skills/.curated",
          skillDirectoryName: "openai-docs",
          downloadUrl: "https://example.com/openai-docs/SKILL.md",
          conflict: null,
        },
      ],
      "github-preview-local-fixture",
    );

    renderMarketplaceView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(await screen.findByTestId("github-import-preview-workspace")).toBeInTheDocument();
    expect(screen.queryByTestId("github-import-remote-workspace-hint")).not.toBeInTheDocument();
  });

  it("switches the selected preview skill inside the import wizard", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.curated/openai-docs",
        skillId: "openai-docs",
        skillName: "OpenAI Docs",
        description: "First skill full description",
        rootDirectory: "skills/.curated",
        skillDirectoryName: "openai-docs",
        downloadUrl: "https://example.com/openai-docs/SKILL.md",
        conflict: null,
      },
      {
        sourcePath: "skills/.system/skill-creator",
        skillId: "skill-creator",
        skillName: "Skill Creator",
        description: "Second skill full description",
        rootDirectory: "skills/.system",
        skillDirectoryName: "skill-creator",
        downloadUrl: "https://example.com/skill-creator/SKILL.md",
        conflict: null,
      },
    ]);

    renderMarketplaceView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    const detailPane = await screen.findByTestId("github-import-detail-pane");
    expect(within(detailPane).getByText("OpenAI Docs")).toBeInTheDocument();
    expect(within(detailPane).queryByText("Skill Creator")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Skill Creator/ }));

    await waitFor(() => {
      expect(within(detailPane).getByText("Skill Creator")).toBeInTheDocument();
    });
    expect(within(detailPane).queryByText("OpenAI Docs")).not.toBeInTheDocument();
  });

  it("bulk selects and deselects all preview skills", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.curated/openai-docs",
        skillId: "openai-docs",
        skillName: "OpenAI Docs",
        description: "OpenAI docs skill description",
        rootDirectory: "skills/.curated",
        skillDirectoryName: "openai-docs",
        downloadUrl: "https://example.com/openai-docs/SKILL.md",
        conflict: null,
      },
      {
        sourcePath: "skills/.system/skill-creator",
        skillId: "skill-creator",
        skillName: "Skill Creator",
        description: "Create skills safely",
        rootDirectory: "skills/.system",
        skillDirectoryName: "skill-creator",
        downloadUrl: "https://example.com/skill-creator/SKILL.md",
        conflict: null,
      },
    ]);

    renderMarketplaceView();
    fireEvent.click(screen.getByRole("button", { name: /GitHub/i }));

    await screen.findByTestId("github-import-preview-workspace");
    const summaryList = screen.getByTestId("github-import-summary-list");
    const getSkillCheckboxes = () =>
      within(summaryList).getAllByRole("checkbox") as HTMLInputElement[];
    const bulkButtons = within(
      screen.getByTestId("github-import-bulk-selection-controls"),
    ).getAllByRole("button") as HTMLButtonElement[];
    const [selectAllButton, deselectAllButton] = bulkButtons;
    const getFooterActionButton = () => {
      const buttons = screen
        .getByTestId("github-import-shell-footer")
        .querySelectorAll("button");
      return buttons[buttons.length - 1] as HTMLButtonElement;
    };

    expect(getSkillCheckboxes()).toHaveLength(2);
    expect(getSkillCheckboxes().every((checkbox) => checkbox.checked)).toBe(true);
    expect(selectAllButton).toBeDisabled();
    expect(deselectAllButton).toBeEnabled();
    expect(getFooterActionButton()).toBeEnabled();

    fireEvent.click(deselectAllButton);

    await waitFor(() => {
      expect(getSkillCheckboxes().every((checkbox) => !checkbox.checked)).toBe(true);
    });
    expect(screen.getByTestId("github-import-repo-toolbar").textContent).toContain("0");
    expect(selectAllButton).toBeEnabled();
    expect(deselectAllButton).toBeDisabled();
    expect(getFooterActionButton()).toBeDisabled();

    fireEvent.click(selectAllButton);

    await waitFor(() => {
      expect(getSkillCheckboxes().every((checkbox) => checkbox.checked)).toBe(true);
    });
    expect(getFooterActionButton()).toBeEnabled();
  });

  it("preserves conflict rename state while bulk toggling preview skills", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.system/skill-creator",
        skillId: "skill-creator",
        skillName: "Skill Creator",
        description: "Create skills safely",
        rootDirectory: "skills/.system",
        skillDirectoryName: "skill-creator",
        downloadUrl: "https://example.com/skill-creator/SKILL.md",
        conflict: {
          existingSkillId: "skill-creator",
          existingName: "Skill Creator",
          existingCanonicalPath: "/Users/test/.skillsmanage/skills/skill-creator",
          proposedSkillId: "skill-creator",
          proposedName: "Skill Creator",
        },
      },
    ]);

    renderMarketplaceView();
    fireEvent.click(screen.getByRole("button", { name: /GitHub/i }));

    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /Rename|重命名/i }));
    fireEvent.change(screen.getByPlaceholderText(/New skill id|新的技能 ID/i), {
      target: { value: "skill-creator-renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Confirm|确认/i }));
    const [selectAllButton, deselectAllButton] = within(
      screen.getByTestId("github-import-bulk-selection-controls"),
    ).getAllByRole("button") as HTMLButtonElement[];
    fireEvent.click(deselectAllButton);
    fireEvent.click(selectAllButton);
    const footerButtons = screen
      .getByTestId("github-import-shell-footer")
      .querySelectorAll("button");
    fireEvent.click(footerButtons[footerButtons.length - 1]);

    const confirmSummary = await screen.findByTestId("github-import-confirm-summary");
    expect(confirmSummary).toHaveTextContent("skill-creator-renamed");
  });

  it("turns conflict resolution into a confirm summary after renaming", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.system/skill-creator",
        skillId: "skill-creator",
        skillName: "Skill Creator",
        description: "Create skills safely",
        rootDirectory: "skills/.system",
        skillDirectoryName: "skill-creator",
        downloadUrl: "https://example.com/skill-creator/SKILL.md",
        conflict: {
          existingSkillId: "skill-creator",
          existingName: "Skill Creator",
          existingCanonicalPath: "/Users/test/.skillsmanage/skills/skill-creator",
          proposedSkillId: "skill-creator",
          proposedName: "Skill Creator",
        },
      },
    ]);

    renderMarketplaceView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /Rename|重命名/i }));
    fireEvent.change(screen.getByPlaceholderText(/New skill id|新的技能 ID/i), {
      target: { value: "skill-creator-renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Confirm|确认/i }));
    fireEvent.click(screen.getByRole("button", { name: /Review import|检查导入内容/i }));

    const confirmSummary = await screen.findByTestId("github-import-confirm-summary");
    expect(confirmSummary).toBeInTheDocument();
    expect(confirmSummary).toHaveTextContent("skill-creator-renamed");
    expect(screen.getByTestId("github-import-shell-footer")).toHaveAttribute(
      "data-footer-mode",
      "confirm",
    );
  });
});
