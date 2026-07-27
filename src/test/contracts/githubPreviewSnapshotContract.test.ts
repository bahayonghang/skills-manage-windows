/**
 * GitHub immutable preview snapshot contract.
 *
 * Preview registers an opaque backend token; every later content read and the
 * confirmed import must carry it. This test guards the renderer side of that
 * contract so a future change cannot reintroduce an optional workspace id or a
 * branch-resolved import path.
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { IPC_COMMANDS } from "@/lib/ipc";

const SRC_ROOT = join(process.cwd(), "src");

function collectSourceFiles(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      if (dir === SRC_ROOT && name === "test") continue;
      collectSourceFiles(full, out);
    } else if (/\.(ts|tsx)$/.test(name)) {
      out.push(full);
    }
  }
  return out;
}

describe("github preview snapshot contract", () => {
  it("types the snapshot-bound commands instead of leaving them untyped", () => {
    for (const command of [
      "preview_github_repo_import",
      "fetch_github_skill_markdown",
      "import_github_repo_skills",
      "discard_github_repo_preview_snapshot",
    ]) {
      expect(Object.keys(IPC_COMMANDS)).toContain(command);
    }
  });

  it("has no production reference to the removed optional workspace payload", () => {
    const offenders = collectSourceFiles(SRC_ROOT).filter((file) =>
      readFileSync(file, "utf8").includes("previewWorkspaceId"),
    );

    expect(offenders).toEqual([]);
  });

  it("keeps discard_github_repo_preview_workspace fully removed", () => {
    const offenders = collectSourceFiles(SRC_ROOT).filter((file) =>
      readFileSync(file, "utf8").includes(
        "discard_github_repo_preview_workspace",
      ),
    );

    expect(offenders).toEqual([]);
  });

  it("routes every import invocation through the single store action", () => {
    const callers = collectSourceFiles(SRC_ROOT).filter((file) => {
      if (file.endsWith(join("lib", "ipc", "commandMap.ts"))) return false;
      return readFileSync(file, "utf8").includes(
        'invoke("import_github_repo_skills"',
      );
    });

    expect(
      callers.map((file) => file.replace(`${SRC_ROOT}/`, "")),
    ).toHaveLength(1);
    expect(callers[0]).toContain("marketplaceStore.githubImportSlice.ts");
  });
});
