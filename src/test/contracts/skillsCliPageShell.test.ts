/// <reference types="node" />

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

import {
  SKILLS_CLI_CONTENT_CONTAINER_CLASS,
  SKILLS_CLI_GRID_CLASS,
} from "@/pages/skillsCliViewModel";

const ROOT = resolve(process.cwd());

function read(relativePath: string): string {
  return readFileSync(resolve(ROOT, relativePath), "utf8");
}

function readTask(taskName: string): string {
  const activePath = `.trellis/tasks/${taskName}/task.json`;
  const archivedPath = `.trellis/tasks/archive/2026-08/${taskName}/task.json`;
  return read(
    existsSync(resolve(ROOT, activePath)) ? activePath : archivedPath,
  );
}

const PAGE_SHELL_SOURCES = [
  "src/pages/SkillsCliView.tsx",
  "src/pages/skillsCliViewModel.ts",
  "src/components/skillsCli/SkillsCliHeader.tsx",
  "src/components/skillsCli/SkillsCliToolbar.tsx",
  "src/components/skillsCli/SkillsCliGroupHeader.tsx",
  "src/components/skillsCli/SkillsCliInstallMount.tsx",
  "src/components/skillsCli/skillsCliActionToast.tsx",
];

describe("Skills CLI page-shell contracts", () => {
  it("locks named container query classes and forbids viewport grid breakpoints", () => {
    expect(SKILLS_CLI_CONTENT_CONTAINER_CLASS).toContain(
      "@container/skills-cli",
    );
    expect(SKILLS_CLI_GRID_CLASS).toContain("grid-cols-2");
    expect(SKILLS_CLI_GRID_CLASS).toContain(
      "@min-[900px]/skills-cli:grid-cols-3",
    );
    expect(SKILLS_CLI_GRID_CLASS).toContain(
      "@min-[1180px]/skills-cli:grid-cols-4",
    );
    expect(SKILLS_CLI_GRID_CLASS).not.toMatch(
      /(?:^|\s)(?:sm|md|lg|xl|2xl|min-\[[0-9]+px\]):grid-cols-/,
    );
    const page = read("src/pages/SkillsCliView.tsx");
    expect(page).toContain("SKILLS_CLI_CONTENT_CONTAINER_CLASS");
    expect(page).toContain("SKILLS_CLI_GRID_CLASS");
    expect(page).not.toMatch(/md:grid-cols-[34]/);
  });

  it("forbids prototype hex, remote fonts/CDN, and hardcoded display fonts", () => {
    const joined = PAGE_SHELL_SOURCES.map((file) => read(file)).join("\n");
    expect(joined).not.toMatch(/#(?:[0-9a-fA-F]{3,8})\b/);
    expect(joined).not.toMatch(
      /fonts\.googleapis|cdn\.jsdelivr|unpkg\.com|fonts\.gstatic/,
    );
    expect(joined).not.toMatch(/fontFamily:\s*["'](?:Inter|Geist|PingFang)/);
  });

  it("keeps install-wizard from owning the page shell files", () => {
    const task = JSON.parse(readTask("08-26-install-wizard")) as {
      relatedFiles?: string[];
    };
    const related = task.relatedFiles ?? [];
    expect(related).not.toContain("src/pages/SkillsCliView.tsx");
    expect(related).not.toContain(
      "src/components/skillsCli/SkillsCliHeader.tsx",
    );
    expect(related).not.toContain("src/pages/skillsCliViewModel.ts");
  });
});
