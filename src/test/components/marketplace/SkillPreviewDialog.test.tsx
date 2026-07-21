import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { SkillPreviewDialog } from "@/components/marketplace/SkillPreviewDialog";

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({
    children,
    open,
  }: {
    children: React.ReactNode;
    open: boolean;
  }) => (open ? <div>{children}</div> : null),
  DialogContent: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div className={className}>{children}</div>
  ),
  DialogHeader: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div className={className}>{children}</div>
  ),
  DialogTitle: ({ children }: { children: React.ReactNode }) => <h1>{children}</h1>,
  DialogBody: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div className={className}>{children}</div>
  ),
  DialogFooter: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div className={className}>{children}</div>
  ),
  DialogClose: () => <button type="button">Close</button>,
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

describe("SkillPreviewDialog", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        text: async () => mockContent,
      })
    );
  });

  it("renders frontmatter card in markdown preview", async () => {
    const { container } = render(
      <SkillPreviewDialog
        open
        onOpenChange={vi.fn()}
        skillName="baoyu-imagine"
        downloadUrl="https://example.com/skills/baoyu-imagine/SKILL.md"
        description="AI image generation skill"
        sourceLabel="Repo One"
        sourceUrl="https://github.com/acme/repo-one"
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
    expect(
      container.querySelector('[data-skill-markdown-variant="detail"]')
    ).not.toBeNull();
  });

  it("keeps raw source tab showing original frontmatter fences", async () => {
    render(
      <SkillPreviewDialog
        open
        onOpenChange={vi.fn()}
        skillName="baoyu-imagine"
        downloadUrl="https://example.com/skills/baoyu-imagine/SKILL.md"
        description="AI image generation skill"
        sourceLabel="Repo One"
        sourceUrl="https://github.com/acme/repo-one"
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
});
