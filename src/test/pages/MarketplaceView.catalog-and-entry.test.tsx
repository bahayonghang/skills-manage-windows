import { beforeEach, describe, expect, it, vi, afterEach } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import * as S from "./marketplaceViewTestSupport";

const {
  mockInstallFromSkillsSh,
  mockLoadCentralSkills,
  mockLoadPreviewSkills,
  mockLoadRegistries,
  mockSearchSkillsSh,
  renderMarketplaceView,
  tauriBridge,
} = S;

describe("MarketplaceView catalog and entry flows", () => {
  beforeEach(S.resetMarketplaceViewTestState);
  afterEach(S.cleanupMarketplaceViewTestState);

  it("loads registries on mount", () => {
    renderMarketplaceView();
    expect(mockLoadRegistries).toHaveBeenCalledTimes(1);
  });

  it("shows recommended skills by default and filters them with search", () => {
    renderMarketplaceView();

    expect(screen.getByRole("button", { name: /Recommended|推荐/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "web-artifacts-builder" })).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText(/Search skills|搜索技能/i), {
      target: { value: "frontend-design" },
    });

    expect(screen.getByRole("button", { name: "frontend-design" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "web-artifacts-builder" })).not.toBeInTheDocument();
  });

  it("loads official directory preview skills from backend cache", async () => {
    renderMarketplaceView();

    fireEvent.click(screen.getByRole("button", { name: /Official Directory|官方源目录/i }));
    fireEvent.click(screen.getByRole("button", { name: /OpenAI/i }));
    fireEvent.click(screen.getByRole("button", { name: /Browse Skills|浏览 Skills/i }));

    await waitFor(() => {
      expect(mockLoadPreviewSkills).toHaveBeenCalledWith("openai");
    });
    expect(await screen.findByText("Knowledge Work Plugin")).toBeInTheDocument();
    expect(screen.getByText("Useful repo preview content")).toBeInTheDocument();
  });

  it("shows browser fallback copy when official preview runs without Tauri", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);

    renderMarketplaceView();

    fireEvent.click(screen.getByRole("button", { name: /Official Directory|官方源目录/i }));
    fireEvent.click(screen.getByRole("button", { name: /OpenAI/i }));
    fireEvent.click(screen.getByRole("button", { name: /Browse Skills|浏览 Skills/i }));

    expect(
      await screen.findByText(/Preview unavailable in browser mode|浏览器模式下暂不支持预览/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/desktop app|桌面应用/i)).toBeInTheDocument();
    expect(mockLoadPreviewSkills).not.toHaveBeenCalled();

    isTauriSpy.mockRestore();
  });

  it("opens the GitHub import wizard from the marketplace CTA", async () => {
    renderMarketplaceView();

    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(screen.getByLabelText(/GitHub repository URL|GitHub 仓库 URL/i)).toBeInTheDocument();
    const branchInput = screen.getByLabelText(/Branch \(optional\)|分支（可选）/i);
    fireEvent.change(branchInput, { target: { value: "dev" } });
    expect(branchInput).toHaveValue("dev");
  });

  it("searches and installs skills.sh results through marketplace store actions", async () => {
    mockSearchSkillsSh.mockResolvedValue(S.storeState.skillsShResults);
    mockInstallFromSkillsSh.mockResolvedValue("webapp-testing");
    mockLoadCentralSkills.mockImplementation(async () => {
      S.storeState.centralSkills = [
        {
          id: "webapp-testing",
          name: "webapp-testing",
          description: "Imported from skills.sh",
          file_path: "~/.agents/skills/webapp-testing/SKILL.md",
          is_central: true,
          source: "skills.sh",
          scanned_at: "2026-05-18T00:00:00Z",
          linked_agents: [],
          shared_root_agents: [],
        },
      ];
    });

    renderMarketplaceView();

    fireEvent.click(screen.getByRole("button", { name: /skills\.sh/i }));
    expect(screen.getByRole("button", { name: "webapp-testing" })).toBeInTheDocument();
    expect(screen.getByText(/68897 次安装/i)).toBeInTheDocument();
    expect(screen.getByText(/1234 星标/i)).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText(/skills\.sh/i), {
      target: { value: "testing" },
    });
    fireEvent.click(screen.getByRole("button", { name: /^搜索$/i }));

    await waitFor(() => {
      expect(mockSearchSkillsSh).toHaveBeenCalledWith("testing");
    });

    fireEvent.click(screen.getByRole("button", { name: /添加到中央技能库|Add to Central/i }));

    await waitFor(() => {
      expect(mockInstallFromSkillsSh).toHaveBeenCalledWith(
        "anthropics/skills",
        "webapp-testing"
      );
    });
    expect(mockLoadCentralSkills).toHaveBeenCalled();
    expect(await screen.findByRole("dialog")).toHaveTextContent(
      /Central install webapp-testing/i
    );
  });
});
