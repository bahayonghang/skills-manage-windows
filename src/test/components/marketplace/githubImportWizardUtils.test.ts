/**
 * Toast-facing translation of GitHub import failures.
 *
 * The confirmed-import surface is a toast, so a preview snapshot lifecycle
 * failure or branch-selection failure must reach the user as a localized
 * sentence instead of the raw `github_import.<code>:<summary>` envelope. Every
 * other failure must keep its exact historical text.
 */
import { describe, expect, it } from "vitest";

import i18n from "@/i18n";
import {
  formatGitHubImportToast,
  isPreviewSnapshotFailure,
} from "@/components/marketplace/githubImportWizardUtils";
import { IpcInvokeError } from "@/lib/ipc";

describe("formatGitHubImportToast", () => {
  it.each([
    ["github_import.preview_missing:gone", "preview_missing"],
    ["github_import.preview_expired:expired", "preview_expired"],
    ["github_import.preview_mismatch:mismatch", "preview_mismatch"],
    ["github_import.preview_integrity:changed", "preview_integrity"],
    ["github_import.preview_busy:busy", "preview_busy"],
  ])("localizes %s", (envelope, code) => {
    expect(isPreviewSnapshotFailure(envelope)).toBe(true);
    const formatted = formatGitHubImportToast(new Error(envelope), i18n.t);

    expect(formatted).toBe(i18n.t(`backendErrors.github_import.${code}`));
    expect(formatted).not.toContain("github_import.");
  });

  it("keeps uncoded failures on their exact historical message", () => {
    const message =
      "No importable skills found in this repository. Supported layouts include subpaths.";

    expect(isPreviewSnapshotFailure(message)).toBe(false);
    expect(formatGitHubImportToast(message, i18n.t)).toBe(message);
  });

  it.each(["branch_invalid", "branch_conflict"])(
    "localizes github_import.%s without classifying it as a snapshot failure",
    (code) => {
      const envelope = `github_import.${code}:private detail`;
      expect(isPreviewSnapshotFailure(envelope)).toBe(false);

      const formatted = formatGitHubImportToast(new Error(envelope), i18n.t);
      expect(formatted).toBe(i18n.t(`backendErrors.github_import.${code}`));
      expect(formatted).not.toContain("private detail");
    },
  );

  it.each(["preview_mismatch", "branch_conflict"])(
    "preserves the github_import.%s code from a structured IPC rejection",
    (code) => {
      const error = new IpcInvokeError({
        code: `github_import.${code}`,
        message: "Reviewed public backend summary.",
        retryable: false,
      });

      expect(formatGitHubImportToast(error, i18n.t)).toBe(
        i18n.t(`backendErrors.github_import.${code}`),
      );
    },
  );

  it("does not reshape an unrelated colon-prefixed message", () => {
    const message = "skills.sh: the requested skill was not found";

    expect(formatGitHubImportToast(message, i18n.t)).toBe(message);
  });
});
