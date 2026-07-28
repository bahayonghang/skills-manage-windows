import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import * as S from "./marketplaceViewTestSupport";

const {
  makePreview,
  renderMarketplaceView,
  storeState,
  useTargetStore,
} = S;

describe("MarketplaceView GitHub SSH and result flows", () => {
  beforeEach(S.resetMarketplaceViewTestState);
  afterEach(S.cleanupMarketplaceViewTestState);

  it("shows inline SSH password repair before remote github import", async () => {
    const storedTarget = {
      id: "ssh-demo",
      kind: "ssh" as const,
      label: "dckj",
      authMethod: "password" as const,
      hasStoredPassword: true,
      credentialStatus: "stored" as const,
      isActive: true,
    };
    const updateSshTargetPassword = async (_targetId: string, _password: string) => {
      useTargetStore.setState({
        targets: [
          { id: "local", kind: "local", label: "Local", isActive: false },
          storedTarget,
        ],
        activeTarget: storedTarget,
      });
      return {
        ok: true,
        remoteHome: "/home/fixture",
        remoteOs: "Linux",
        credentialStatus: "stored" as const,
        message: "SSH password saved.",
      };
    };
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        {
          id: "ssh-demo",
          kind: "ssh",
          label: "dckj",
          authMethod: "password",
          hasStoredPassword: false,
          isActive: true,
        },
      ],
      activeTarget: {
        id: "ssh-demo",
        kind: "ssh",
        label: "dckj",
        authMethod: "password",
        hasStoredPassword: false,
        isActive: true,
      },
      updateSshTargetPassword,
    });
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
    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /Review import|检查导入内容/i }));

    const repairPanel = await screen.findByTestId("github-import-ssh-password-repair");
    expect(repairPanel).toHaveTextContent(/Save the active SSH password|保存当前 SSH 密码/i);
    expect(screen.getByRole("button", { name: /^Import$|^导入$/i })).toBeDisabled();

    fireEvent.change(screen.getByLabelText(/SSH password for dckj|dckj 的 SSH 密码/i), {
      target: { value: "secret" },
    });
    const savePasswordButton = screen.getByRole("button", { name: /Save password|保存密码/i });
    await waitFor(() => {
      expect(savePasswordButton).toBeEnabled();
    });
    fireEvent.click(savePasswordButton);
    const footerButtons = screen
      .getByTestId("github-import-shell-footer")
      .querySelectorAll("button");
    const importButton = footerButtons[footerButtons.length - 1] as HTMLButtonElement;

    await waitFor(() => {
      expect(importButton).toBeEnabled();
    });
    await waitFor(() => {
      expect(
        screen.getByText(/SSH password saved for dckj|已保存 dckj 的 SSH 密码/i),
      ).toBeInTheDocument();
    });
  });

  it("allows remote github import with a session-only SSH password", async () => {
    const sessionTarget = {
      id: "ssh-demo",
      kind: "ssh" as const,
      label: "dckj",
      authMethod: "password" as const,
      hasStoredPassword: true,
      credentialStatus: "session" as const,
      isActive: true,
    };
    const updateSshTargetPassword = async (_targetId: string, _password: string) => {
      useTargetStore.setState({
        targets: [
          { id: "local", kind: "local", label: "Local", isActive: false },
          sessionTarget,
        ],
        activeTarget: sessionTarget,
      });
      return {
        ok: true,
        remoteHome: "/home/fixture",
        remoteOs: "Linux",
        credentialStatus: "session" as const,
        credentialError: "credential vault locked",
        message: "session only",
      };
    };
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        { ...sessionTarget, hasStoredPassword: false, credentialStatus: "missing" },
      ],
      activeTarget: {
        ...sessionTarget,
        hasStoredPassword: false,
        credentialStatus: "missing",
      },
      updateSshTargetPassword,
    });
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
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库|å¯¼å…¥ GitHub ä»“åº“/i }));
    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /Review import|检查导入内容|æ£€æŸ¥å¯¼å…¥å†…å®¹/i }));
    fireEvent.change(screen.getByLabelText(/SSH password for dckj|dckj 的 SSH 密码|dckj çš„ SSH å¯†ç /i), {
      target: { value: "secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Save password|保存密码|ä¿å­˜å¯†ç /i }));

    await waitFor(() => {
      expect(screen.getByTestId("github-import-ssh-password-repair").textContent).toMatch(
        /session|本次会话|ä¼šè¯/i,
      );
    });
    const footerButtons = screen
      .getByTestId("github-import-shell-footer")
      .querySelectorAll("button");
    expect(footerButtons[footerButtons.length - 1]).toBeEnabled();
  });

  it("renders the result hub when an import result already exists", async () => {
    storeState.githubImport.importResult = {
      repo: {
        owner: "openai",
        repo: "skills",
        branch: "main",
        normalizedUrl: "https://github.com/openai/skills",
      },
      importedSkills: [
        {
          sourcePath: "skills/.curated/openai-docs",
          originalSkillId: "openai-docs",
          importedSkillId: "openai-docs",
          skillName: "OpenAI Docs",
          targetDirectory: "/Users/test/.skillsmanage/skills/openai-docs",
          resolution: "overwrite",
        },
      ],
      skippedSkills: ["legacy-skill"],
    };

    renderMarketplaceView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    const resultHub = await screen.findByTestId("github-import-result-hub");
    expect(resultHub).toBeInTheDocument();
    expect(within(resultHub).getByRole("button", { name: /Continue platform setup|继续配置平台安装/i })).toBeInTheDocument();
    expect(within(resultHub).getByRole("button", { name: /Open Central|打开中央技能库/i })).toBeInTheDocument();
    expect(within(resultHub).getByRole("button", { name: /Start another import|开始新的导入/i })).toBeInTheDocument();
    expect(within(resultHub).getByText("legacy-skill")).toBeInTheDocument();
  });

  it("shows settings guidance when github preview fails with auth or rate-limit help", async () => {
    storeState.githubImport.error =
      "github_import.rate_limited:GitHub rate limited the request.";

    renderMarketplaceView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(
      await screen.findByText(/GitHub Personal Access Token/i),
    ).toBeInTheDocument();
  });
});
