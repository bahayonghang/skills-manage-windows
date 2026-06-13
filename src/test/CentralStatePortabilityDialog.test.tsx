import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi, beforeEach } from "vitest";

import defaultCapability from "../../src-tauri/capabilities/default.json";
import { CentralStatePortabilityDialog } from "../components/central/CentralStatePortabilityDialog";
import type { SkillportStateImportPreview, SkillportStatePortabilityJob } from "../types";

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
    {
      id: "broken-skill",
      name: "broken-skill",
      sourcePath: "skills/broken-skill/SKILL.md",
      status: "unrestorable",
      reason: "invalid_frontmatter",
      detail: "Skill 'skills/broken-skill' is missing valid frontmatter.",
    },
  ],
  summary: {
    sourcesToAdd: 1,
    sourcesExisting: 0,
    sourcesDuplicate: 0,
    ready: 1,
    conflicts: 1,
    missing: 0,
    unrestorable: 1,
    duplicateSkipped: 0,
  },
  warnings: [],
};

const idlePortabilityJob: SkillportStatePortabilityJob = {
  phase: null,
  status: "idle",
  total: 0,
  completed: 0,
};

function renderDialog(props: Partial<ComponentProps<typeof CentralStatePortabilityDialog>> = {}) {
  const exportState = vi.fn().mockResolvedValue(manifestJson);
  const result = render(
    <CentralStatePortabilityDialog
      open
      onOpenChange={vi.fn()}
      exportState={exportState}
      previewImport={vi.fn()}
      importState={vi.fn()}
      portabilityJob={idlePortabilityJob}
      onCancelJob={vi.fn()}
      {...props}
    />
  );
  return { ...result, exportState };
}

async function flushInitialExport(result: ReturnType<typeof renderDialog>) {
  await waitFor(() => expect(result.exportState).toHaveBeenCalled());
  await waitFor(() =>
    expect(screen.getByTestId("central-portability-export-json")).toHaveValue(
      JSON.stringify(JSON.parse(manifestJson), null, 2)
    )
  );
}

describe("CentralStatePortabilityDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("keeps Tauri FS permissions for user-selected JSON files", () => {
    const capability = defaultCapability as { permissions: Array<string | { identifier: string }> };
    const scopePermission = capability.permissions.find(
      (permission): permission is { identifier: string } =>
        typeof permission === "object" && permission.identifier === "fs:scope"
    );

    expect(capability.permissions).toContain("dialog:allow-open");
    expect(capability.permissions).toContain("dialog:allow-save");
    expect(capability.permissions).not.toContain("dialog:default");
    expect(capability.permissions).toContain("fs:allow-read-text-file");
    expect(capability.permissions).toContain("fs:allow-write-text-file");
    expect(scopePermission).toBeDefined();
  });

  it("saves the exported state JSON", async () => {
    vi.mocked(save).mockResolvedValue("D:\\exports\\skillport-state.json");
    vi.mocked(writeTextFile).mockResolvedValue(undefined);
    const exportState = vi.fn().mockResolvedValue(manifestJson);

    renderDialog({ exportState });

    await waitFor(() => expect(exportState).toHaveBeenCalled());
    fireEvent.click(screen.getByTestId("central-portability-save-export"));

    await waitFor(() =>
      expect(writeTextFile).toHaveBeenCalledWith(
        "D:\\exports\\skillport-state.json",
        JSON.stringify(JSON.parse(manifestJson), null, 2)
      )
    );
  });

  it("switches export JSON between raw and pretty views", async () => {
    const exportState = vi.fn().mockResolvedValue(manifestJson);

    renderDialog({ exportState });

    const textarea = await screen.findByTestId("central-portability-export-json");
    expect(textarea).toHaveValue(JSON.stringify(JSON.parse(manifestJson), null, 2));

    fireEvent.click(screen.getByTestId("central-portability-raw-json"));
    expect(textarea).toHaveValue(manifestJson);
  });

  it("loads a JSON file and previews it before import", async () => {
    vi.mocked(open).mockResolvedValue("D:\\imports\\skillport-state.json");
    vi.mocked(readTextFile).mockResolvedValue(manifestJson);
    const previewImport = vi.fn().mockResolvedValue(preview);

    renderDialog({ previewImport });

    fireEvent.click(screen.getByTestId("central-portability-import-tab"));
    fireEvent.click(screen.getByTestId("central-portability-choose-file"));

    await waitFor(() => expect(readTextFile).toHaveBeenCalledWith("D:\\imports\\skillport-state.json"));
    expect(previewImport).toHaveBeenCalledWith(manifestJson);
    expect(await screen.findByText("frontend-design")).toBeInTheDocument();
  });

  it("renders unrestorable preview diagnostics", async () => {
    const previewImport = vi.fn().mockResolvedValue(preview);

    renderDialog({ previewImport });

    fireEvent.click(screen.getByTestId("central-portability-import-tab"));
    fireEvent.change(screen.getByTestId("central-portability-json-input"), {
      target: { value: manifestJson },
    });
    fireEvent.click(screen.getByTestId("central-portability-preview"));

    expect(await screen.findByText("broken-skill")).toBeInTheDocument();
    expect(screen.getAllByText("不可恢复").length).toBeGreaterThan(0);
    expect(
      screen.getByText((_, element) =>
        element?.textContent ===
        "导出的技能元数据无效: Skill 'skills/broken-skill' is missing valid frontmatter."
      )
    ).toBeInTheDocument();
  });

  it("formats import JSON without clearing invalid input", async () => {
    const result = renderDialog();
    await flushInitialExport(result);

    fireEvent.click(screen.getByTestId("central-portability-import-tab"));
    fireEvent.change(screen.getByTestId("central-portability-json-input"), {
      target: { value: "{bad json" },
    });
    fireEvent.click(screen.getByTestId("central-portability-format-import"));

    expect(screen.getByTestId("central-portability-json-input")).toHaveValue("{bad json");
    expect(screen.getByText(/无法格式化 JSON/)).toBeInTheDocument();

    fireEvent.change(screen.getByTestId("central-portability-json-input"), {
      target: { value: manifestJson },
    });
    fireEvent.click(screen.getByTestId("central-portability-format-import"));

    expect(screen.getByTestId("central-portability-json-input")).toHaveValue(
      JSON.stringify(JSON.parse(manifestJson), null, 2)
    );
  });

  it("renders duplicate summary diagnostics", async () => {
    const duplicatePreview: SkillportStateImportPreview = {
      ...preview,
      skills: [
        ...preview.skills,
        {
          id: "openai-docs",
          name: "openai-docs",
          sourcePath: "skills/.system/openai-docs/SKILL.md",
          status: "duplicate_skipped",
          reason: "duplicate_in_json",
        },
      ],
      summary: {
        ...preview.summary,
        duplicateSkipped: 1,
        sourcesDuplicate: 1,
      },
    };
    const previewImport = vi.fn().mockResolvedValue(duplicatePreview);

    renderDialog({ previewImport });

    fireEvent.click(screen.getByTestId("central-portability-import-tab"));
    fireEvent.change(screen.getByTestId("central-portability-json-input"), {
      target: { value: manifestJson },
    });
    fireEvent.click(screen.getByTestId("central-portability-preview"));

    expect(await screen.findByText("JSON 内重复")).toBeInTheDocument();
    expect(screen.getByText("源重复")).toBeInTheDocument();
    expect(screen.getByText("重复已跳过")).toBeInTheDocument();
  });

  it("renders repo inspection warnings without blocking ready skills", async () => {
    const warningPreview: SkillportStateImportPreview = {
      ...preview,
      skills: [
        {
          id: "repo-warning-skill",
          name: "repo-warning-skill",
          sourcePath: "skills/repo-warning-skill/SKILL.md",
          status: "ready",
        },
      ],
      summary: {
        ...preview.summary,
        ready: 1,
        conflicts: 0,
        unrestorable: 0,
      },
      warnings: [
        {
          reason: "repo_unavailable",
          detail: "GitHub rate limit was exceeded",
          repoUrl: "https://github.com/other/skills/tree/main",
        },
      ],
    };
    const previewImport = vi.fn().mockResolvedValue(warningPreview);

    renderDialog({ previewImport });

    fireEvent.click(screen.getByTestId("central-portability-import-tab"));
    fireEvent.change(screen.getByTestId("central-portability-json-input"), {
      target: { value: manifestJson },
    });
    fireEvent.click(screen.getByTestId("central-portability-preview"));

    expect(await screen.findByText("1 条预览警告")).toBeInTheDocument();
    expect(screen.getByText(/无法检查源仓库/)).toBeInTheDocument();
    expect(screen.getByText(/https:\/\/github.com\/other\/skills\/tree\/main/)).toBeInTheDocument();
    expect(screen.getByText(/源仓库暂时无法检查/)).toBeInTheDocument();
    expect(screen.getByTestId("central-portability-run-import")).toHaveTextContent(
      "导入 1 个技能"
    );
    expect(screen.getByTestId("central-portability-run-import")).not.toBeDisabled();
  });

  it("submits ready and conflict resolutions", async () => {
    const previewImport = vi.fn().mockResolvedValue(preview);
    const onOpenChange = vi.fn();
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

    renderDialog({ onOpenChange, previewImport, importState, onAfterImport });

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
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("keeps the dialog open and renders failed imports on partial success", async () => {
    const previewImport = vi.fn().mockResolvedValue(preview);
    const onOpenChange = vi.fn();
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
      failedSkills: [
        {
          skillId: "frontend-design",
          sourcePath: "skills/frontend-design/SKILL.md",
          error: "Target directory already exists.",
        },
      ],
      tagsRestored: 1,
    });

    renderDialog({ onOpenChange, previewImport, importState });

    fireEvent.click(screen.getByTestId("central-portability-import-tab"));
    fireEvent.change(screen.getByTestId("central-portability-json-input"), {
      target: { value: manifestJson },
    });
    fireEvent.click(screen.getByTestId("central-portability-preview"));

    await screen.findByText("frontend-design");
    fireEvent.click(screen.getByTestId("central-portability-run-import"));

    expect(await screen.findByText("1 个技能导入失败")).toBeInTheDocument();
    expect(screen.getByText("Target directory already exists.")).toBeInTheDocument();
    expect(onOpenChange).not.toHaveBeenCalledWith(false);
  });

  it("shows progress and calls cancel for running portability jobs", async () => {
    const onCancelJob = vi.fn().mockResolvedValue(undefined);

    const result = renderDialog({
      onCancelJob,
      portabilityJob: {
        phase: "importing",
        status: "running",
        total: 4,
        completed: 2,
        message: "Importing",
      },
    });
    await waitFor(() => expect(result.exportState).toHaveBeenCalled());

    expect(screen.getByTestId("central-portability-progress")).toBeInTheDocument();
    await act(async () => {
      fireEvent.click(screen.getByTestId("central-portability-cancel-job"));
    });

    expect(onCancelJob).toHaveBeenCalled();
    expect(screen.getByTestId("central-portability-save-export")).toBeDisabled();
  });

  it("only clears the preview for manifest errors", async () => {
    const previewImport = vi
      .fn()
      .mockResolvedValueOnce(preview)
      .mockRejectedValueOnce(new Error("Repository access denied"))
      .mockRejectedValueOnce(new Error("Invalid SkillPort state JSON: bad json"));

    renderDialog({ previewImport });

    fireEvent.click(screen.getByTestId("central-portability-import-tab"));
    fireEvent.change(screen.getByTestId("central-portability-json-input"), {
      target: { value: manifestJson },
    });
    fireEvent.click(screen.getByTestId("central-portability-preview"));
    expect(await screen.findByText("frontend-design")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("central-portability-preview"));
    expect(await screen.findByText("frontend-design")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("central-portability-preview"));
    await waitFor(() =>
      expect(screen.queryByText("frontend-design")).not.toBeInTheDocument()
    );
  });
});
