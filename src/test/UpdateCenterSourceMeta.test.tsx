import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { RemoteMissingTabPanel } from "@/components/central/updateCenter/RemoteMissingTabPanel";
import { UpdatableTabPanel } from "@/components/central/updateCenter/UpdatableTabPanel";
import type { RepositorySourceDisplayInfo } from "@/lib/centralConflictSource";
import type {
  RemoteMissingSkill,
  UpdatableSkill,
} from "@/types/skillUpdateInventory";

const repositorySources = new Map<string, RepositorySourceDisplayInfo>([
  [
    "github-openai-skills-main",
    {
      label: "openai/skills",
      url: "https://github.com/openai/skills",
    },
  ],
]);

const updatable: UpdatableSkill = {
  repositoryId: "github-openai-skills-main",
  diagnostics: {
    cachePolicy: "bypass",
    cacheHit: false,
    localHash: "fnv1a64:local",
    remoteHash: "fnv1a64:remote",
  },
  state: {
    skill_id: "planning-with-files-zh",
    source_type: "github",
    source_url: "https://github.com/openai/skills",
    ref: "main",
    source_path: "skills/planning-with-files-zh",
    status: "update_available",
  },
};

const missing: RemoteMissingSkill = {
  repositoryId: "github-openai-skills-main",
  state: {
    skill_id: "removed-skill",
    source_type: "github",
    source_url: "https://github.com/openai/skills",
    ref: "main",
    source_path: "skills/removed-skill",
    status: "remote_missing",
  },
};

describe("Update Center source metadata", () => {
  it("shows repository, path, and URL for updatable skills", () => {
    render(
      <UpdatableTabPanel
        items={[updatable]}
        state={{ "planning-with-files-zh": { selected: true } }}
        repositorySources={repositorySources}
        onChange={vi.fn()}
        onToggleAll={vi.fn()}
      />,
    );

    const row = screen.getByText("planning-with-files-zh").closest("label")!;
    expect(within(row).getByText("openai/skills")).toBeInTheDocument();
    expect(within(row).getByText("skills/planning-with-files-zh")).toBeInTheDocument();
    expect(within(row).getByText("https://github.com/openai/skills")).toBeInTheDocument();
    expect(within(row).getByText(/Bypass cache|绕过缓存/)).toBeInTheDocument();
    expect(
      within(row).getByText("fnv1a64:local -> fnv1a64:remote"),
    ).toBeInTheDocument();
  });

  it("uses distinct visual treatments for source metadata kinds", () => {
    render(
      <UpdatableTabPanel
        items={[updatable]}
        state={{ "planning-with-files-zh": { selected: true } }}
        repositorySources={repositorySources}
        onChange={vi.fn()}
        onToggleAll={vi.fn()}
      />,
    );

    const row = screen.getByText("planning-with-files-zh").closest("label")!;
    const metaChip = (key: string) =>
      row.querySelector<HTMLElement>(`[data-source-meta-key="${key}"]`)!;

    expect(metaChip("repository")).toHaveClass("bg-sky-500/10", "ring-sky-500/30");
    expect(metaChip("path")).toHaveClass(
      "bg-emerald-500/10",
      "ring-emerald-500/30",
    );
    expect(metaChip("url")).toHaveClass("bg-cyan-500/10", "ring-cyan-500/30");
    expect(metaChip("cache")).toHaveClass("bg-muted/35", "ring-border/60");
    expect(metaChip("hash")).toHaveClass(
      "bg-violet-500/10",
      "ring-violet-500/30",
    );
  });

  it("uses repository labels for removed remote skills", () => {
    render(
      <RemoteMissingTabPanel
        items={[missing]}
        state={{ "removed-skill": { decision: "keep", removeAgentIds: [] } }}
        repositorySources={repositorySources}
        onChange={vi.fn()}
      />,
    );

    const row = screen.getByText("removed-skill").closest("article")!;
    expect(within(row).getByText("openai/skills")).toBeInTheDocument();
    expect(within(row).getByText("skills/removed-skill")).toBeInTheDocument();
    expect(within(row).getByText("https://github.com/openai/skills")).toBeInTheDocument();
  });

  it("falls back to the repository id when repository metadata is missing", () => {
    render(
      <UpdatableTabPanel
        items={[updatable]}
        state={{ "planning-with-files-zh": { selected: true } }}
        repositorySources={new Map()}
        onChange={vi.fn()}
        onToggleAll={vi.fn()}
      />,
    );

    const row = screen.getByText("planning-with-files-zh").closest("label")!;
    expect(within(row).getByText("github-openai-skills-main")).toBeInTheDocument();
  });
});
