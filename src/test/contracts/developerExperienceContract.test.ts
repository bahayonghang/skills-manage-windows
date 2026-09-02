/// <reference types="node" />

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";
import { parseFrontmatter } from "@/lib/frontmatter";

type WorkflowStep = {
  run?: string;
  uses?: string;
  with?: Record<string, unknown>;
};

type Workflow = {
  jobs?: Record<string, { steps?: WorkflowStep[] }>;
};

const packageJson = JSON.parse(readFileSync("package.json", "utf8")) as {
  engines?: { node?: string };
  packageManager?: string;
  scripts: Record<string, string>;
};
const nodeVersion = readFileSync(".node-version", "utf8").trim();
const rustToolchain = readFileSync("rust-toolchain.toml", "utf8");
const justfile = readFileSync("justfile", "utf8");
const template = readFileSync(".github/pull_request_template.md", "utf8");
const workflows = readdirSync(".github/workflows")
  .filter((name) => /\.ya?ml$/.test(name))
  .map((name) => parse(readFileSync(`.github/workflows/${name}`, "utf8")) as Workflow);

function allSteps() {
  return workflows.flatMap((workflow) => Object.values(workflow.jobs ?? {}).flatMap((job) => job.steps ?? []));
}

describe("developer and PR experience contract", () => {
  it("declares one Node/pnpm/Rust toolchain", () => {
    expect(nodeVersion).toBe("26");
    expect(packageJson.packageManager).toBe("pnpm@10.34.5");
    expect(packageJson.engines?.node).toBe("26.x");
    expect(rustToolchain).toContain('channel = "1.98.0"');
    expect(rustToolchain).toContain('components = ["rustfmt", "clippy"]');
  });

  it("keeps hosted Actions on the declared Node, pnpm, and Rust versions", () => {
    const steps = allSteps();
    const pnpmSetupSteps = steps.filter((step) => step.uses?.startsWith("pnpm/action-setup@"));
    expect(pnpmSetupSteps.length).toBeGreaterThan(0);
    expect(pnpmSetupSteps.every((step) => step.with?.version === "10.34.5")).toBe(true);

    const nodeSetupSteps = steps.filter((step) => step.uses?.startsWith("actions/setup-node@"));
    expect(nodeSetupSteps.length).toBeGreaterThan(0);
    expect(nodeSetupSteps.every((step) => step.with?.["node-version"] === "26")).toBe(true);

    const rustSetupSteps = steps.filter((step) => step.uses?.startsWith("dtolnay/rust-toolchain@"));
    expect(rustSetupSteps.length).toBeGreaterThan(0);
    expect(rustSetupSteps.every((step) => step.with?.toolchain === "1.98.0")).toBe(true);
  });

  it("exposes read-only doctor and audit gates while local check/ci auto-sync version", () => {
    expect(packageJson.scripts.doctor).toBe("node scripts/check/doctor.mjs");
    expect(justfile).toMatch(/doctor:\r?\n\s+node scripts\/check\/doctor\.mjs/);
    expect(justfile).toMatch(/check: sync-version\r?\n\s+node scripts\/check\/run-ci\.mjs --lane quick/);
    expect(justfile).toMatch(/ci: sync-version\r?\n\s+node scripts\/check\/run-ci\.mjs/);
    expect(justfile).toMatch(/audit:\r?\n\s+pnpm audit:dependencies/);
  });

  it("keeps repository scripts in classified subfolders", () => {
    const entries = readdirSync("scripts", { withFileTypes: true });
    expect(entries.every((entry) => entry.isDirectory())).toBe(true);
    expect(entries.map((entry) => entry.name).sort()).toEqual([
      "build",
      "check",
      "docs",
      "lib",
      "release",
    ]);
  });

  it("runs CI lanes through run-ci.mjs without the just CLI so version drift still fails", () => {
    const runs = allSteps().flatMap((step) => step.run ?? []);
    expect(runs.some((run) => /\bjust\b/.test(run))).toBe(false);
    expect(runs.filter((run) => /sync-version\.mjs --check/.test(run)).length).toBeGreaterThan(0);
  });

  it("requires review inputs for task and promotion pull requests", () => {
    for (const heading of [
      "## User problem",
      "## Scope",
      "## Risk and rollback",
      "## Validation",
      "## UI evidence",
      "## Packaging and release impact",
      "## Merge path",
    ]) {
      expect(template).toContain(heading);
    }
    expect(template.toLowerCase()).toContain("task prs target `dev` and use squash merge");
    expect(template).toContain("Promotion PRs target `main`, use a merge commit");
    expect(template).toContain("fast-forwarded to the promotion merge SHA");
  });

  it("documents the same local checks and branch model in both languages", () => {
    const english = readFileSync("README.md", "utf8");
    const chinese = readFileSync("README_CN.md", "utf8");
    const contributing = readFileSync("CONTRIBUTING.md", "utf8");
    const agents = readFileSync("AGENTS.md", "utf8");
    const quality = readFileSync(".trellis/spec/quality/ci-quality-gate.md", "utf8");

    for (const document of [english, chinese, contributing, agents, quality]) {
      expect(document).toContain("Node 26");
      expect(document).toContain("pnpm 10.34.5");
      expect(document).toContain("Rust 1.98.0");
      expect(document).toContain("just doctor");
      expect(document).toContain("just check");
      expect(document).toContain("just ci");
      expect(document).toContain("just audit");
    }

    for (const document of [english, chinese, contributing, agents, quality]) {
      expect(document).toContain("dev");
      expect(document).toContain("main");
      expect(document.toLowerCase()).toContain("squash");
      expect(document.toLowerCase()).toContain("merge commit");
      expect(document.toLowerCase()).toContain("fast-forward");
    }
  });
});

type SkillCatalogEntry = {
  name: string;
  entrypoint: string;
  raw: string;
};

type SkillCatalogIssue = "duplicate-name" | "duplicate-entrypoint" | "bootstarp";

function collectSkills(root: string): SkillCatalogEntry[] {
  if (!existsSync(root) || !statSync(root).isDirectory()) {
    return [];
  }
  const entries: SkillCatalogEntry[] = [];
  for (const dirent of readdirSync(root, { withFileTypes: true })) {
    if (!dirent.isDirectory()) {
      continue;
    }
    const skillMd = join(root, dirent.name, "SKILL.md");
    if (!existsSync(skillMd)) {
      continue;
    }
    const raw = readFileSync(skillMd, "utf8");
    const parsed = parseFrontmatter(raw);
    const name = String(parsed.frontmatterData.name ?? dirent.name);
    entries.push({
      name,
      entrypoint: dirent.name,
      raw: `${parsed.frontmatterRaw}\n${dirent.name}`,
    });
  }
  return entries;
}

function catalogIssues(entries: SkillCatalogEntry[]): SkillCatalogIssue[] {
  const issues: SkillCatalogIssue[] = [];
  const names = entries.map((entry) => entry.name);
  const entrypoints = entries.map((entry) => entry.entrypoint);
  if (new Set(names).size !== names.length) {
    issues.push("duplicate-name");
  }
  if (new Set(entrypoints).size !== entrypoints.length) {
    issues.push("duplicate-entrypoint");
  }
  if (entries.some((entry) => /bootstarp/i.test(`${entry.name}\n${entry.entrypoint}\n${entry.raw}`))) {
    issues.push("bootstarp");
  }
  return issues;
}

describe("Trellis skill catalog contract", () => {
  const skillRoots = [".agents/skills", ".claude/skills"] as const;

  it("fails on duplicate skill names", () => {
    expect(
      catalogIssues([
        { name: "trellis-spec-bootstrap", entrypoint: "a", raw: "name: trellis-spec-bootstrap" },
        { name: "trellis-spec-bootstrap", entrypoint: "b", raw: "name: trellis-spec-bootstrap" },
      ]),
    ).toEqual(["duplicate-name"]);
  });

  it("fails on duplicate skill entrypoints", () => {
    expect(
      catalogIssues([
        { name: "one", entrypoint: "trellis-spec-bootstrap", raw: "name: one" },
        { name: "two", entrypoint: "trellis-spec-bootstrap", raw: "name: two" },
      ]),
    ).toEqual(["duplicate-entrypoint"]);
  });

  it("fails on a bootstarp misspelling", () => {
    expect(
      catalogIssues([
        {
          name: "trellis-spec-bootstarp",
          entrypoint: "trellis-spec-bootstarp",
          raw: "name: trellis-spec-bootstarp",
        },
      ]),
    ).toEqual(["bootstarp"]);
  });

  it("keeps unique catalogs without trellis-spec-bootstarp when skill trees exist", () => {
    for (const root of skillRoots) {
      if (!existsSync(root)) {
        continue;
      }
      expect(existsSync(join(root, "trellis-spec-bootstarp"))).toBe(false);
      expect(existsSync(join(root, "trellis-spec-bootstrap", "SKILL.md"))).toBe(true);
      expect(catalogIssues(collectSkills(root))).toEqual([]);
    }
  });
});

