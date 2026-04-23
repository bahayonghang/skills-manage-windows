import {
  compactHomePath,
  describeSkillsPattern,
  getPathBasename,
  normalizePathSeparators,
} from "@/lib/path";

describe("path helpers", () => {
  it("normalizes windows separators", () => {
    expect(normalizePathSeparators("C:\\Users\\alice\\.claude\\skills")).toBe(
      "C:/Users/alice/.claude/skills"
    );
  });

  it("compacts unix home paths", () => {
    expect(compactHomePath("/Users/alice/.agents/skills")).toBe("~/.agents/skills");
    expect(compactHomePath("/home/alice/projects/demo")).toBe("~/projects/demo");
  });

  it("compacts windows home paths", () => {
    expect(compactHomePath("C:\\Users\\alice\\.cursor\\skills")).toBe(
      "~/.cursor/skills"
    );
  });

  it("keeps tilde paths stable", () => {
    expect(compactHomePath("~/.skillsmanage/db.sqlite")).toBe("~/.skillsmanage/db.sqlite");
  });

  it("extracts basenames for unix and windows paths", () => {
    expect(getPathBasename("/Users/alice/.claude/skills/review")).toBe("review");
    expect(getPathBasename("C:\\Users\\alice\\.claude\\skills\\review")).toBe("review");
  });

  it("describes skill patterns relative to home", () => {
    expect(describeSkillsPattern("/Users/alice/.claude/skills")).toBe(".claude/skills");
    expect(describeSkillsPattern("C:\\Users\\alice\\.cursor\\skills")).toBe(
      ".cursor/skills"
    );
  });
});
