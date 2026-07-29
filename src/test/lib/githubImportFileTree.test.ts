import { describe, expect, it } from "vitest";

import {
  buildGitHubImportFileTree,
  flattenGitHubImportFileTree,
  getGitHubImportFileManifestIssue,
} from "@/lib/githubImportFileTree";

describe("githubImportFileTree", () => {
  it("builds stable directory-first structure and aggregates counts and bytes", () => {
    const model = buildGitHubImportFileTree([
      { path: "SKILL.md", byteLen: 40 },
      { path: "references/guide.md", byteLen: 12 },
      { path: "assets/icons/logo.png", byteLen: 100 },
      { path: "assets/theme.json", byteLen: 20 },
    ]);

    expect(model.fileCount).toBe(4);
    expect(model.directoryCount).toBe(3);
    expect(model.totalByteLen).toBe(172);
    expect(model.roots.map((node) => node.name)).toEqual([
      "assets",
      "references",
      "SKILL.md",
    ]);
    expect(model.roots[0]).toMatchObject({
      path: "assets",
      byteLen: 120,
      descendantFileCount: 2,
    });
    expect(model.defaultExpandedPaths).toEqual(["assets", "references"]);
  });

  it("flattens only expanded branches", () => {
    const model = buildGitHubImportFileTree([
      { path: "SKILL.md", byteLen: 10 },
      { path: "references/deep/example.md", byteLen: 5 },
    ]);

    const initialRows = flattenGitHubImportFileTree(
      model.roots,
      new Set(model.defaultExpandedPaths),
    );
    expect(initialRows.map((row) => row.node.path)).toEqual([
      "references",
      "references/deep",
      "SKILL.md",
    ]);

    const expandedRows = flattenGitHubImportFileTree(
      model.roots,
      new Set([...model.defaultExpandedPaths, "references/deep"]),
    );
    expect(expandedRows.map((row) => row.node.path)).toContain(
      "references/deep/example.md",
    );
  });

  it("rejects missing, unsafe, duplicate, conflicting, or incomplete manifests", () => {
    expect(getGitHubImportFileManifestIssue(undefined)).toBe("missing");
    expect(getGitHubImportFileManifestIssue([])).toBe("empty");
    expect(
      getGitHubImportFileManifestIssue([
        { path: "../SKILL.md", byteLen: 1 },
      ]),
    ).toBe("invalid-path");
    expect(
      getGitHubImportFileManifestIssue([
        { path: "SKILL.md", byteLen: 1 },
        { path: "SKILL.md", byteLen: 1 },
      ]),
    ).toBe("duplicate-path");
    expect(
      getGitHubImportFileManifestIssue([
        { path: "SKILL.md", byteLen: 1 },
        { path: "assets", byteLen: 1 },
        { path: "assets/logo.png", byteLen: 1 },
      ]),
    ).toBe("path-conflict");
    expect(
      getGitHubImportFileManifestIssue([
        { path: "README.md", byteLen: 1 },
      ]),
    ).toBe("missing-skill-markdown");
  });
});
