import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MarketplaceSkillDetailDrawer } from "@/components/marketplace/MarketplaceSkillDetailDrawer";

const mockResolveSkillsShUrl = vi.fn();
const mockBrowseSkillsShDirectory = vi.fn();
const mockReadSkillsShFile = vi.fn();

vi.mock("@/stores/marketplaceStore", () => ({
  useMarketplaceStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      triggerSkillExplanation: vi.fn(),
      resolveSkillsShUrl: mockResolveSkillsShUrl,
      browseSkillsShDirectory: mockBrowseSkillsShDirectory,
      readSkillsShFile: mockReadSkillsShFile,
    }),
}));

vi.mock("react-markdown", () => ({
  default: ({
    children,
    remarkPlugins,
  }: {
    children: string;
    remarkPlugins?: unknown[];
  }) => (
    <div
      data-testid="react-markdown"
      data-has-remark-gfm={remarkPlugins && remarkPlugins.length > 0 ? "true" : "false"}
    >
      {children}
    </div>
  ),
}));

const skill = {
  id: "skill-1",
  name: "baoyu-imagine",
  downloadUrl: "https://example.com/skills/baoyu-imagine/SKILL.md",
  description: "AI image generation skill",
  sourceLabel: "Repo One",
  sourceUrl: "https://github.com/acme/repo-one",
  installed: false,
};

const installedSkill = {
  ...skill,
  installed: true,
};

const mockContent = `---
name: baoyu-imagine
version: 1.57.0
metadata:
  openclaw:
    requires:
      anyBins:
        - bun
        - npx
---

# Image Generation

Body content.`;

describe("MarketplaceSkillDetailDrawer", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    mockResolveSkillsShUrl.mockReset();
    mockBrowseSkillsShDirectory.mockReset();
    mockReadSkillsShFile.mockReset();
    mockResolveSkillsShUrl.mockResolvedValue(
      "https://raw.githubusercontent.com/anthropics/skills/main/webapp-testing/SKILL.md"
    );
    mockBrowseSkillsShDirectory.mockResolvedValue([
      { name: "SKILL.md", path: "webapp-testing/SKILL.md", is_dir: false },
      { name: "references", path: "webapp-testing/references", is_dir: true },
      {
        name: "guide.md",
        path: "webapp-testing/references/guide.md",
        is_dir: false,
      },
    ]);
    mockReadSkillsShFile.mockImplementation(async (_source: string, path: string) =>
      path.endsWith("guide.md") ? "# Guide" : mockContent
    );
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        text: async () => mockContent,
      })
    );
  });

  describe("SkillDetailModalShell integration", () => {
    it("renders SkillDetailModalShell (not SkillDetailPanelShell) when open", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      const modal = await screen.findByTestId("skill-detail-modal");
      expect(modal).toBeInTheDocument();
      expect(modal).toHaveAttribute("role", "dialog");
      expect(modal).toHaveAttribute("aria-modal", "true");

      expect(screen.getByTestId("skill-detail-modal-overlay")).toBeInTheDocument();
    });

    it("does not render modal content when closed", () => {
      render(
        <MarketplaceSkillDetailDrawer
          open={false}
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      expect(screen.queryByTestId("skill-detail-modal")).not.toBeInTheDocument();
    });
  });

  describe("content structure preservation", () => {
    it("renders header with skill name", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      await waitFor(() => {
        expect(screen.getByRole("heading", { name: skill.name })).toBeInTheDocument();
      });
    });

    it("renders two-column layout", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      const layout = await screen.findByTestId("skill-detail-two-column-layout");
      expect(layout).toBeInTheDocument();
    });

    it("renders right sidebar with metadata", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      const sidebar = await screen.findByTestId("skill-detail-right-sidebar");
      expect(sidebar).toBeInTheDocument();
      expect(within(sidebar).getByText("Repo One")).toBeInTheDocument();
    });
  });

  describe("install button states", () => {
    it("shows install button in available state when skill is not installed", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      await screen.findByTestId("skill-detail-modal");

      const installButtons = screen.getAllByRole("button", { name: /安装/i });
      const enabledInstallButton = installButtons.find(
        (btn) => !btn.hasAttribute("disabled")
      );
      expect(enabledInstallButton).toBeDefined();
    });

    it("shows installing state with spinner when isInstalling is true", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={true}
        />
      );

      await screen.findByTestId("skill-detail-modal");

      const installButtons = screen.getAllByRole("button", { name: /安装/i });
      const disabledButtons = installButtons.filter((btn) =>
        btn.hasAttribute("disabled")
      );
      expect(disabledButtons.length).toBeGreaterThan(0);
    });

    it("shows installed state when skill is already installed", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={installedSkill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      await screen.findByTestId("skill-detail-modal");

      const installedButtons = screen.getAllByRole("button", { name: /已安装/i });
      expect(installedButtons.length).toBeGreaterThan(0);
      for (const btn of installedButtons) {
        expect(btn).toBeDisabled();
      }
    });

    it("calls onInstall when install button is clicked", async () => {
      const onInstall = vi.fn();
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={onInstall}
          isInstalling={false}
        />
      );

      await screen.findByTestId("skill-detail-modal");

      const installButtons = screen.getAllByRole("button", { name: /安装/i });
      const enabledButton = installButtons.find(
        (btn) => !btn.hasAttribute("disabled")
      );
      expect(enabledButton).toBeDefined();
      fireEvent.click(enabledButton!);
      expect(onInstall).toHaveBeenCalled();
    });
  });

  describe("frontmatter and content rendering", () => {
    it("renders frontmatter card in markdown preview", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      await waitFor(() => {
        expect(screen.getByRole("heading", { name: /Frontmatter/i })).toBeInTheDocument();
      });

      const frontmatter = screen.getByRole("heading", { name: /Frontmatter/i }).closest("section");
      expect(frontmatter).not.toBeNull();
      expect(within(frontmatter as HTMLElement).getByText("baoyu-imagine")).toBeInTheDocument();
      expect(within(frontmatter as HTMLElement).getByText(/v1\.57\.0/i)).toBeInTheDocument();
      expect(within(frontmatter as HTMLElement).getByText("bun")).toBeInTheDocument();
      expect(within(frontmatter as HTMLElement).getByText("npx")).toBeInTheDocument();
      expect(screen.getByTestId("react-markdown")).toHaveTextContent("# Image Generation");
    });

    it("keeps raw source tab showing original frontmatter fences", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={skill}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      await waitFor(() => {
        expect(screen.getByTestId("react-markdown")).toBeInTheDocument();
      });

      fireEvent.click(screen.getByRole("button", { name: /原始源码/i }));

      await waitFor(() => {
        expect(screen.getByText(/name: baoyu-imagine/i)).toBeInTheDocument();
      });
      expect(screen.getByText(/---/)).toBeInTheDocument();
    });

    it("loads a skills.sh file tree and opens remote files through store actions", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={{
            ...skill,
            id: "skills.sh:anthropics/skills:webapp-testing",
            name: "webapp-testing",
            downloadUrl: "https://github.com/anthropics/skills",
            source: "anthropics/skills",
            skillId: "webapp-testing",
            remoteKind: "skills_sh",
            installs: 68897,
            stars: 1234,
          }}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      await waitFor(() => {
        expect(screen.getAllByText("SKILL.md").length).toBeGreaterThan(0);
      });
      expect(mockBrowseSkillsShDirectory).toHaveBeenCalledWith(
        "anthropics/skills",
        "webapp-testing"
      );
      expect(mockReadSkillsShFile).toHaveBeenCalledWith(
        "anthropics/skills",
        "webapp-testing/SKILL.md"
      );
      expect(mockBrowseSkillsShDirectory).toHaveBeenCalledTimes(1);
      expect(mockReadSkillsShFile).toHaveBeenCalledTimes(1);

      fireEvent.click(screen.getByRole("button", { name: "展开 webapp-testing" }));
      fireEvent.click(await screen.findByRole("button", { name: "展开 references" }));
      fireEvent.click(await screen.findByRole("button", { name: "打开 guide.md" }));

      await waitFor(() => {
        expect(mockReadSkillsShFile).toHaveBeenCalledWith(
          "anthropics/skills",
          "webapp-testing/references/guide.md"
        );
      });
      expect(mockReadSkillsShFile).toHaveBeenCalledTimes(2);
      expect(await screen.findByText("# Guide")).toBeInTheDocument();
    });

    it("labels skills.sh install action as adding to Central", async () => {
      render(
        <MarketplaceSkillDetailDrawer
          open
          skill={{
            ...skill,
            id: "skills.sh:anthropics/skills:webapp-testing",
            name: "webapp-testing",
            downloadUrl: "https://github.com/anthropics/skills",
            source: "anthropics/skills",
            skillId: "webapp-testing",
            remoteKind: "skills_sh",
          }}
          onOpenChange={vi.fn()}
          onInstall={vi.fn()}
          isInstalling={false}
        />
      );

      await screen.findByTestId("skill-detail-modal");
      expect(
        screen.getAllByRole("button", { name: /添加到中央技能库|Add to Central/i }).length
      ).toBeGreaterThan(0);
    });
  });
});
