/// <reference types="node" />

import { readFileSync, readdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

type WorkflowStep = {
  if?: string;
  name?: string;
  uses?: string;
  run?: string;
  with?: Record<string, unknown>;
};

type WorkflowJob = {
  "continue-on-error"?: boolean | string;
  if?: string;
  name?: string;
  needs?: string[];
  "runs-on"?: string;
  steps?: WorkflowStep[];
  strategy?: {
    "fail-fast"?: boolean;
    matrix?: { runner?: string[] };
  };
};

type WorkflowTrigger = {
  branches?: string[];
  inputs?: Record<string, unknown>;
  types?: string[];
} | null;

type Workflow = {
  jobs: Record<string, WorkflowJob>;
  on: Record<string, WorkflowTrigger>;
};

const workflow = parse(readFileSync(".github/workflows/ci.yml", "utf8")) as Workflow;
const dependabot = parse(readFileSync(".github/dependabot.yml", "utf8")) as {
  version: number;
  updates: Array<{
    "package-ecosystem": string;
    directory: string;
    schedule: { interval: string };
  }>;
};
const packageJobIds = ["windows-package", "linux-package", "macos-package"];
const packageEventGuard = "${{ github.event_name == 'workflow_dispatch' }}";
const workflowDocuments = readdirSync(".github/workflows")
  .filter((name) => /\.ya?ml$/.test(name))
  .map((name) => ({
    name,
    value: parse(readFileSync(`.github/workflows/${name}`, "utf8")) as unknown,
  }));

function findStep(job: WorkflowJob, predicate: (step: WorkflowStep) => boolean) {
  return job.steps?.find(predicate);
}

function collectUses(value: unknown): string[] {
  if (Array.isArray(value)) return value.flatMap(collectUses);
  if (!value || typeof value !== "object") return [];

  return Object.entries(value).flatMap(([key, candidate]) =>
    key === "uses" && typeof candidate === "string" ? [candidate] : collectUses(candidate),
  );
}

describe("CI workflow contract", () => {
  it("runs the quality gate for main pull requests and integration pushes", () => {
    expect(workflow.on.pull_request).toEqual({ branches: ["main"] });
    expect(workflow.on.push).toEqual({ branches: ["main", "dev"] });
    expect(workflow.on.workflow_dispatch).toBeNull();
    expect(workflow.on.release).toBeUndefined();
    expect(workflow.on.workflow_call).toEqual({
      inputs: {
        checkout_ref: {
          description: "Frozen commit SHA to validate",
          required: true,
          type: "string",
        },
      },
    });
  });

  it("keeps the required just-ci check stable and complete", () => {
    const ciJob = workflow.jobs.ci;
    expect(ciJob).toBeDefined();
    expect(ciJob.name).toBe("just-ci");
    expect(ciJob.needs).toEqual(["source-validation", "supply-chain"]);
    expect(ciJob.if).toBe("${{ always() }}");
    expect(findStep(ciJob, (step) => step.run === "node scripts/run-ci.mjs")).toBeDefined();
    const prerequisite = findStep(
      ciJob,
      (step) => step.name === "Require source and supply-chain checks",
    );
    expect(prerequisite?.if).toBe("${{ always() }}");
    expect(prerequisite?.run).toContain("needs.source-validation.result");
    expect(prerequisite?.run).toContain("needs.supply-chain.result");
    expect(findStep(ciJob, (step) => step.uses?.startsWith("actions/checkout@") ?? false)?.with?.ref)
      .toBe("${{ inputs.checkout_ref || github.sha }}");

    const rustSetup = findStep(ciJob, (step) =>
      step.uses?.startsWith("dtolnay/rust-toolchain@") ?? false
    );
    const rustComponents = String(rustSetup?.with?.components ?? "")
      .split(",")
      .map((component) => component.trim());
    expect(rustComponents).toEqual(expect.arrayContaining(["clippy", "rustfmt"]));
  });

  it("runs blocking Ubuntu and macOS source validation", () => {
    const sourceValidation = workflow.jobs["source-validation"];
    expect(sourceValidation).toBeDefined();
    expect(sourceValidation.name).toBe("source-validation (${{ matrix.runner }})");
    expect(sourceValidation["runs-on"]).toBe("${{ matrix.runner }}");
    expect(sourceValidation.if).toBeUndefined();
    expect(sourceValidation["continue-on-error"]).toBeUndefined();
    expect(sourceValidation.strategy).toEqual({
      "fail-fast": false,
      matrix: { runner: ["ubuntu-22.04", "macos-14"] },
    });
    expect(
      findStep(sourceValidation, (step) => step.run === "node scripts/run-ci.mjs"),
    ).toBeDefined();
  });

  it("runs the dependency audit as a blocking job", () => {
    const supplyChain = workflow.jobs["supply-chain"];
    expect(supplyChain).toBeDefined();
    expect(supplyChain.name).toBe("supply-chain");
    expect(supplyChain["runs-on"]).toBe("ubuntu-22.04");
    expect(supplyChain.if).toBeUndefined();
    expect(supplyChain["continue-on-error"]).toBeUndefined();
    expect(
      findStep(supplyChain, (step) => step.run === "pnpm audit:dependencies"),
    ).toBeDefined();
  });

  it("limits smoke packaging to direct manual dispatches", () => {
    for (const jobId of packageJobIds) {
      expect(workflow.jobs[jobId]?.if, `${jobId} event guard`).toBe(packageEventGuard);
    }
  });

  it("pins every external action to a full commit SHA", () => {
    for (const document of workflowDocuments) {
      for (const reference of collectUses(document.value)) {
        if (reference.startsWith("./")) continue;
        expect(reference, `${document.name}: ${reference}`).toMatch(
          /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+@[0-9a-f]{40}$/,
        );
      }
    }
  });

  it("keeps GitHub Actions pins on a weekly Dependabot schedule", () => {
    expect(dependabot.version).toBe(2);
    expect(dependabot.updates).toContainEqual({
      "package-ecosystem": "github-actions",
      directory: "/",
      schedule: { interval: "weekly" },
    });
  });
});
