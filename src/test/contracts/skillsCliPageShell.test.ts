/// <reference types="node" />

import { readFileSync } from "node:fs";
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

const PAGE_SHELL_SOURCES = [
  "src/pages/SkillsCliView.tsx",
  "src/pages/skillsCliViewModel.ts",
  "src/components/skillsCli/SkillsCliHeader.tsx",
  "src/components/skillsCli/SkillsCliToolbar.tsx",
  "src/components/skillsCli/SkillsCliGroupHeader.tsx",
  "src/components/skillsCli/SkillsCliBatchBar.tsx",
  "src/components/skillsCli/SkillsCliCleanupDialog.tsx",
  "src/components/skillsCli/SkillsCliInstallMount.tsx",
  "src/components/skillsCli/skillsCliActionToast.tsx",
];

const PAGE_SHELL_OWNED_PATHS = [
  "src/pages/SkillsCliView.tsx",
  "src/components/skillsCli/SkillsCliHeader.tsx",
  "src/pages/skillsCliViewModel.ts",
] as const;

const INSTALL_WIZARD_SOURCES = [
  "src/pages/skillsCliInstallViewModel.ts",
  "src/components/skillsCli/SkillsCliInstallSurface.tsx",
  "src/components/skillsCli/SkillsCliInstallDialog.tsx",
  "src/components/skillsCli/SkillsCliInstallDialog.steps.tsx",
  "src/stores/skillsCliRecentSourcesStore.ts",
];

describe("Skills CLI page-shell contracts", () => {
  it("locks named container query classes and forbids viewport grid breakpoints", () => {
    expect(SKILLS_CLI_CONTENT_CONTAINER_CLASS).toContain("@container/skills-cli");
    expect(SKILLS_CLI_GRID_CLASS).toContain("grid-cols-2");
    expect(SKILLS_CLI_GRID_CLASS).toContain("@min-[900px]/skills-cli:grid-cols-3");
    expect(SKILLS_CLI_GRID_CLASS).toContain("@min-[1180px]/skills-cli:grid-cols-4");
    expect(SKILLS_CLI_GRID_CLASS).not.toMatch(
      /(?:^|\s)(?:sm|md|lg|xl|2xl|min-\[[0-9]+px\]):grid-cols-/,
    );
    const page = read("src/pages/SkillsCliView.tsx");
    expect(page).toContain("SKILLS_CLI_CONTENT_CONTAINER_CLASS");
    expect(page).toContain("SKILLS_CLI_GRID_CLASS");
    expect(page).not.toMatch(/md:grid-cols-[34]/);
    expect(page).not.toMatch(/(?:^|\s)(?:md|lg):/);
    expect(SKILLS_CLI_GRID_CLASS).toContain("gap-3");
  });

  it("forbids prototype hex, remote fonts/CDN, and hardcoded display fonts", () => {
    const joined = PAGE_SHELL_SOURCES.map((file) => read(file)).join("\n");
    expect(joined).not.toMatch(/#(?:[0-9a-fA-F]{3,8})\b/);
    expect(joined).not.toMatch(/fonts\.googleapis|cdn\.jsdelivr|unpkg\.com|fonts\.gstatic/);
    expect(joined).not.toMatch(/fontFamily:\s*["'](?:Inter|Geist|PingFang)/);
  });

  it("keeps install-wizard from owning the page shell files", () => {
    const joined = INSTALL_WIZARD_SOURCES.map((file) => read(file)).join("\n");
    for (const owned of PAGE_SHELL_OWNED_PATHS) {
      expect(joined).not.toContain(owned);
    }
    expect(joined).not.toMatch(/from ["']@\/pages\/SkillsCliView["']/);
    expect(joined).not.toMatch(
      /from ["']@\/components\/skillsCli\/SkillsCliHeader["']/,
    );
    expect(joined).not.toMatch(/from ["']@\/pages\/skillsCliViewModel["']/);
  });
});
