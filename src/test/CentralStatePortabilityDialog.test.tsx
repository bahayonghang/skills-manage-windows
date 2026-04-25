import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";

import { CentralStatePortabilityDialog } from "../components/central/CentralStatePortabilityDialog";
import type { SkillportStateImportPreview } from "../types";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn(),
  open: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-fs", () => ({
  writeTextFile: vi.fn(),
  readTextFile: vi.fn(),
}));

import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";

const manifestJson = JSON.stringify({
  kind: "skillport/state-export",
  version: 1,
  exportedAt: "2026-04-25T00:00:00Z",
  exportedFrom: { app: "SkillPort" },
  githubSources: [{ name: "OpenAI Skills" }],
  centralSkills: [{ id: "openai-docs" }],
  unrestorableSkills: [{ id: "local-skill" }],
});

const preview: SkillportStateImportPreview = {
  githubSources: [
    {
      name: "OpenAI Skills",
      url: "https://github.com/openai/skills",
      status: "will_add",
    },
  ],
  skills: [
    {
      id: "openai-docs",
      name: "openai-docs",
      sourcePath: "skills/.system/openai-docs/SKILL.md",
      status: "ready",
    },
    {
      id: "frontend-design",
      name: "frontend-design",
      sourcePath: "skills/frontend-design/SKILL.md",
      status: "conflict",
      existingSkillId: "frontend-design",
    },
  ],
  summary: {
    sourcesToAdd: 1,
    sourcesExisting: 0,
    ready: 1,
    conflicts: 1,
    missing: 0,
    unrestorable: 0,
  },
};

describe("CentralStatePortabilityDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("saves the exported state JSON", async () => {
    vi.mocked(save).mockResolvedValue("D:\\exports\\skillport-state.json");
    vi.mocked(writeTextFile).mockResolvedValue(undefined);
    const exportState = vi.fn().mockResolvedValue(manifestJson);

    render(
      <CentralStatePortabilityDialog
        open
        onOpenChange={vi.fn()}
        exportState={exportState}
        previewImport={vi.fn()}
        importState={vi.fn()}
      />
    );

    await waitFor(() => expect(exportState).toHaveBeenCalled());
    fireEvent.click(screen.getByTestId("central-portability-save-export"));

    await waitFor(() =>
      expect(writeTextFile).toHaveBeenCalledWith("D:\\exports\\skillport-state.json", manifestJson)
    );
  });

  it("loads a JSON file and previews it before import", async () => {
    vi.mocked(open).mockResolvedValue("D:\\imports\\skillport-state.json");
    vi.mocked(readTextFile).mockResolvedValue(manifestJson);
    const previewImport = vi.fn().mockResolvedValue(preview);

    render(
      <CentralStatePortabilityDialog
        open
        onOpenChange={vi.fn()}
        exportState={vi.fn().mockResolvedValue(manifestJson)}
        previewImport={previewImport}
        importState={vi.fn()}
      />
    );

    fireEvent.click(screen.getByTestId("central-portability-import-tab"));
    fireEvent.click(screen.getByTestId("central-portability-choose-file"));

    await waitFor(() => expect(readTextFile).toHaveBeenCalledWith("D:\\imports\\skillport-state.json"));
    expect(previewImport).toHaveBeenCalledWith(manifestJson);
    expect(await screen.findByText("frontend-design")).toBeInTheDocument();
  });

  it("submits ready and conflict resolutions", async () => {
    const previewImport = vi.fn().mockResolvedValue(preview);
    const importState = vi.fn().mockResolvedValue({
      sourcesAdded: 1,
      sourcesSkipped: 0,
      importedSkills: [
        {
          sourcePath: "skills/.system/openai-docs/SKILL.md",
          importedSkillId: "openai-docs",
          skillName: "openai-docs",
        },
      ],
      skippedSkills: [],
      failedSkills: [],
      tagsRestored: 1,
    });
    const onAfterImport = vi.fn();

    render(
      <CentralStatePortabilityDialog
        open
        onOpenChange={vi.fn()}
        exportState={vi.fn().mockResolvedValue(manifestJson)}
        previewImport={previewImport}
        importState={importState}
        onAfterImport={onAfterImport}
      />
    );

    fireEvent.click(screen.getByTestId("central-portability-import-tab"));
    fireEvent.change(screen.getByTestId("central-portability-json-input"), {
      target: { value: manifestJson },
    });
    fireEvent.click(screen.getByTestId("central-portability-preview"));

    await screen.findByText("frontend-design");
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "rename" } });
    fireEvent.change(screen.getByPlaceholderText("新技能 ID"), {
      target: { value: "frontend-design-imported" },
    });
    fireEvent.click(screen.getByTestId("central-portability-run-import"));

    await waitFor(() =>
      expect(importState).toHaveBeenCalledWith(manifestJson, [
        {
          skillId: "openai-docs",
          sourcePath: "skills/.system/openai-docs/SKILL.md",
          resolution: "overwrite",
          renamedSkillId: null,
        },
        {
          skillId: "frontend-design",
          sourcePath: "skills/frontend-design/SKILL.md",
          resolution: "rename",
          renamedSkillId: "frontend-design-imported",
        },
      ])
    );
    expect(onAfterImport).toHaveBeenCalled();
  });
});
