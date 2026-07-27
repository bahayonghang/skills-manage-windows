/// <reference types="node" />

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

type WorkflowStep = {
  uses?: string;
  run?: string;
  with?: Record<string, unknown>;
};

type WorkflowJob = {
  if?: string;
  name?: string;
  steps?: WorkflowStep[];
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
const packageJobIds = ["windows-package", "linux-package", "macos-package"];
const packageEventGuard = "${{ github.event_name == 'workflow_dispatch' }}";

function findStep(job: WorkflowJob, predicate: (step: WorkflowStep) => boolean) {
  return job.steps?.find(predicate);
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
    expect(ciJob.if).toBeUndefined();
    expect(findStep(ciJob, (step) => step.run === "node scripts/run-ci.mjs")).toBeDefined();
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

  it("limits smoke packaging to direct manual dispatches", () => {
    for (const jobId of packageJobIds) {
      expect(workflow.jobs[jobId]?.if, `${jobId} event guard`).toBe(packageEventGuard);
    }
  });
});
