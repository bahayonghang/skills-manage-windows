import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { LocalArchiveImportWizard } from "@/components/central/LocalArchiveImportWizard";
import { SkillImportLauncher } from "@/components/central/SkillImportLauncher";
import i18n from "@/i18n";
import {
  createInitialLocalArchiveImportState,
  useLocalArchiveImportStore,
} from "@/stores/localArchiveImportSlice";
import { ipcInvokeCalls, mockIpcCommands } from "@/test/support/ipcMock";
import type { LocalArchiveImportResult, LocalArchivePreview } from "@/types";

const preview: LocalArchivePreview = {
  archiveDisplayName: "demo.zip",
  fingerprint: { sha256: "a".repeat(64), byteLen: 128 },
  skills: [
    {
      rootDirectory: "",
      skillId: "demo-skill",
      skillName: "Demo Skill",
      description: "Preview description",
      skillMdPath: "SKILL.md",
      files: [
        { path: "SKILL.md", byteLen: 80 },
        { path: "references/readme.md", byteLen: 48 },
      ],
      fileCount: 2,
      totalExpandedBytes: 128,
      conflict: {
        existingSkillId: "demo-skill",
        existingName: "Old Demo",
        existingCanonicalPath: "C:/Users/alice/.skillsmanage/skills/demo-skill",
        proposedSkillId: "demo-skill",
        proposedName: "Demo Skill",
      },
    },
  ],
  totalFiles: 2,
  totalExpandedBytes: 128,
  totalCompressedBytes: 96,
  archiveByteLen: 128,
};

const imported: LocalArchiveImportResult = {
  importedSkillId: "renamed-skill",
  skillName: "Demo Skill",
  rootDirectory: "",
  resolution: "rename",
  fileCount: 2,
  totalExpandedBytes: 128,
  replacedExisting: false,
};

function renderWizard(onAfterImportSuccess = vi.fn().mockResolvedValue(undefined)) {
  useLocalArchiveImportStore.getState().openWizard();
  render(
    <LocalArchiveImportWizard
      t={i18n.t}
      onAfterImportSuccess={onAfterImportSuccess}
    />,
  );
  return { onAfterImportSuccess };
}

describe("local archive import", () => {
  beforeEach(() => {
    useLocalArchiveImportStore.setState(createInitialLocalArchiveImportState());
    vi.mocked(openDialog).mockReset();
  });

  it("routes launcher intents and disables local ZIP for remote targets", async () => {
    const onOpenIntent = vi.fn();
    const { rerender } = render(
      <SkillImportLauncher t={i18n.t} isRemoteTarget={false} onOpenIntent={onOpenIntent} />,
    );
    fireEvent.click(screen.getByTestId("central-add-skill-launcher"));
    fireEvent.click(await screen.findByTestId("central-add-skill-github"));
    expect(onOpenIntent).toHaveBeenCalledWith("github");

    rerender(
      <SkillImportLauncher t={i18n.t} isRemoteTarget onOpenIntent={onOpenIntent} />,
    );
    fireEvent.click(screen.getByTestId("central-add-skill-launcher"));
    const zipItem = await screen.findByTestId("central-add-skill-local-zip");
    expect(zipItem).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(zipItem);
    expect(onOpenIntent).not.toHaveBeenCalledWith("local_zip");
  });

  it("keeps choose state when file selection is cancelled", async () => {
    vi.mocked(openDialog).mockResolvedValue(null);
    renderWizard();
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("local-archive-choose-file"));
    await waitFor(() => expect(openDialog).toHaveBeenCalledTimes(1));
    expect(ipcInvokeCalls("preview_local_skill_archive")).toHaveLength(0);
    expect(useLocalArchiveImportStore.getState().step).toBe("choose");
  });

  it("previews, renames, imports, refreshes Central, and resets on close", async () => {
    vi.mocked(openDialog).mockResolvedValue("C:/Users/alice/private/demo.zip");
    mockIpcCommands({
      preview_local_skill_archive: preview,
      import_local_skill_archive: imported,
    });
    const { onAfterImportSuccess } = renderWizard();
    const dialog = await screen.findByRole("dialog");
    const wizard = within(dialog);

    fireEvent.click(wizard.getByTestId("local-archive-choose-file"));
    expect(await wizard.findByText("Demo Skill")).toBeInTheDocument();
    expect(wizard.getByText("demo.zip")).toBeInTheDocument();
    expect(wizard.getByTestId("github-import-file-tree")).toBeInTheDocument();
    expect(wizard.getByText("SKILL.md")).toBeInTheDocument();
    fireEvent.click(
      wizard.getByRole("radio", {
        name: i18n.t("central.localArchiveWizard.resolutionRename"),
      }),
    );
    fireEvent.change(wizard.getByTestId("local-archive-rename-input"), {
      target: { value: "renamed-skill" },
    });
    fireEvent.click(wizard.getByTestId("local-archive-import-confirm"));

    expect(await wizard.findByText(i18n.t("central.localArchiveWizard.importSuccess"))).toBeInTheDocument();
    expect(onAfterImportSuccess).toHaveBeenCalledTimes(1);
    expect(ipcInvokeCalls("import_local_skill_archive")[0]?.args).toMatchObject({
      archivePath: "C:/Users/alice/private/demo.zip",
      resolution: "rename",
      renamedSkillId: "renamed-skill",
    });

    fireEvent.click(wizard.getByRole("button", { name: i18n.t("common.close") }));
    expect(useLocalArchiveImportStore.getState()).toMatchObject({
      isOpen: false,
      step: "choose",
      archivePath: null,
      preview: null,
      importResult: null,
    });
  });

  it("shows localized safe errors without backend payloads", async () => {
    vi.mocked(openDialog).mockResolvedValue("C:/Users/alice/private/demo.zip");
    mockIpcCommands({
      preview_local_skill_archive: () =>
        Promise.reject(
          "local_archive.invalid_archive_entry:The archive contains ../../token-secret password=hunter2",
        ),
    });
    renderWizard();
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("local-archive-choose-file"));

    expect(
      await within(dialog).findByText(
        i18n.t("backendErrors.local_archive.invalid_archive_entry"),
      ),
    ).toBeInTheDocument();
    expect(dialog).not.toHaveTextContent("alice");
    expect(dialog).not.toHaveTextContent("token-secret");
    expect(dialog).not.toHaveTextContent("hunter2");
  });

  it("keeps preview open and localizes import failures without payloads", async () => {
    vi.mocked(openDialog).mockResolvedValue("C:/Users/alice/private/demo.zip");
    mockIpcCommands({
      preview_local_skill_archive: preview,
      import_local_skill_archive: () =>
        Promise.reject(
          "local_archive.rollback_failed:restore C:/Users/alice token-secret password=hunter2",
        ),
    });
    renderWizard();
    const dialog = await screen.findByRole("dialog");
    const wizard = within(dialog);
    fireEvent.click(wizard.getByTestId("local-archive-choose-file"));
    expect(await wizard.findByText("Demo Skill")).toBeInTheDocument();
    fireEvent.click(wizard.getByTestId("local-archive-import-confirm"));

    expect(
      await wizard.findByText(i18n.t("backendErrors.local_archive.rollback_failed")),
    ).toBeInTheDocument();
    expect(useLocalArchiveImportStore.getState().step).toBe("preview");
    expect(dialog).not.toHaveTextContent("alice");
    expect(dialog).not.toHaveTextContent("token-secret");
    expect(dialog).not.toHaveTextContent("hunter2");
  });

  it("maps unknown failures to a localized generic error", async () => {
    vi.mocked(openDialog).mockResolvedValue("C:/Users/alice/private/demo.zip");
    mockIpcCommands({
      preview_local_skill_archive: () => Promise.reject("DB password=hunter2"),
    });
    renderWizard();
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("local-archive-choose-file"));

    expect(
      await within(dialog).findByText(i18n.t("backendErrors.local_archive.unknown")),
    ).toBeInTheDocument();
    expect(dialog).not.toHaveTextContent("hunter2");
  });
});
