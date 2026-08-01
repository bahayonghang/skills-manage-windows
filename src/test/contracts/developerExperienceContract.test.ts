/// <reference types="node" />

import { readFileSync, readdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

type WorkflowStep = {
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
    expect(nodeVersion).toBe("22");
    expect(packageJson.packageManager).toBe("pnpm@10.12.3");
    expect(packageJson.engines?.node).toBe("22.x");
    expect(rustToolchain).toContain('channel = "1.97.0"');
    expect(rustToolchain).toContain('components = ["rustfmt", "clippy"]');
  });

  it("keeps hosted Actions on the declared Node, pnpm, and Rust versions", () => {
    const steps = allSteps();
    const pnpmSetupSteps = steps.filter((step) => step.uses?.startsWith("pnpm/action-setup@"));
    expect(pnpmSetupSteps.length).toBeGreaterThan(0);
    expect(pnpmSetupSteps.every((step) => step.with?.version === "10.12.3")).toBe(true);

    const nodeSetupSteps = steps.filter((step) => step.uses?.startsWith("actions/setup-node@"));
    expect(nodeSetupSteps.length).toBeGreaterThan(0);
    expect(nodeSetupSteps.every((step) => step.with?.["node-version"] === "22")).toBe(true);

    const rustSetupSteps = steps.filter((step) => step.uses?.startsWith("dtolnay/rust-toolchain@"));
    expect(rustSetupSteps.length).toBeGreaterThan(0);
    expect(rustSetupSteps.every((step) => step.with?.toolchain === "1.97.0")).toBe(true);
  });

  it("exposes read-only doctor and quick check entries without weakening full gates", () => {
    expect(packageJson.scripts.doctor).toBe("node scripts/doctor.mjs");
    expect(justfile).toMatch(/doctor:\r?\n\s+node scripts\/doctor\.mjs/);
    expect(justfile).toMatch(/check:\r?\n\s+node scripts\/run-ci\.mjs --lane quick/);
    expect(justfile).toMatch(/ci:\r?\n\s+node scripts\/run-ci\.mjs/);
    expect(justfile).toMatch(/audit:\r?\n\s+pnpm audit:dependencies/);
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
      expect(document).toContain("Node 22");
      expect(document).toContain("pnpm 10.12.3");
      expect(document).toContain("Rust 1.97.0");
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
