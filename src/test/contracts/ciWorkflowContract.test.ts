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
  "timeout-minutes"?: number;
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
  concurrency: {
    "cancel-in-progress": boolean | string;
    group: string;
  };
  jobs: Record<string, WorkflowJob>;
  on: Record<string, WorkflowTrigger>;
  permissions: Record<string, string>;
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
const requiredLaneIds = ["common", "windows-rust", "linux-rust", "macos-rust", "supply-chain"];
const requiredLaneTimeouts: Record<string, number> = {
  common: 15,
  "windows-rust": 25,
  "linux-rust": 20,
  "macos-rust": 25,
  "supply-chain": 10,
};
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
  it("runs for task and promotion pull requests without a top-level path filter", () => {
    expect(workflow.on.pull_request).toEqual({ branches: ["dev", "main"] });
    expect(workflow.on.push).toBeUndefined();
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
    expect(workflow.concurrency).toEqual({
      group: "ci-${{ github.event.pull_request.number || github.ref }}",
      "cancel-in-progress": "${{ github.event_name == 'pull_request' }}",
    });
    expect(workflow.permissions).toEqual({ contents: "read" });
  });

  it("runs all required lanes independently with bounded timeouts", () => {
    for (const jobId of requiredLaneIds) {
      const job = workflow.jobs[jobId];
      expect(job, jobId).toBeDefined();
      expect(job.needs, `${jobId} must not wait for another required lane`).toBeUndefined();
      expect(job.if, `${jobId} must not be skipped`).toBeUndefined();
      expect(job["continue-on-error"], `${jobId} must block`).toBeUndefined();
      expect(job["timeout-minutes"], `${jobId} timeout`).toBe(requiredLaneTimeouts[jobId]);
      const checkout = findStep(job, (step) => step.uses?.startsWith("actions/checkout@") ?? false);
      expect(checkout?.with?.ref, `${jobId} frozen checkout`).toBe(
        "${{ inputs.checkout_ref || github.sha }}",
      );
    }

    expect(workflow.jobs.common["runs-on"]).toBe("ubuntu-22.04");
    expect(findStep(
      workflow.jobs.common,
      (step) => step.run === "node scripts/run-ci.mjs --lane common",
    )).toBeDefined();

    const platformJobs = {
      "windows-rust": "windows-2022",
      "linux-rust": "ubuntu-22.04",
      "macos-rust": "macos-14",
    };
    for (const [jobId, runner] of Object.entries(platformJobs)) {
      const job = workflow.jobs[jobId];
      expect(job["runs-on"]).toBe(runner);
      expect(findStep(
        job,
        (step) => step.run === "node scripts/run-ci.mjs --lane rust-platform",
      )).toBeDefined();
      const rustSetup = findStep(job, (step) =>
        step.uses?.startsWith("dtolnay/rust-toolchain@") ?? false
      );
      expect(String(rustSetup?.with?.components ?? "").split(",")).toContain("clippy");
    }
  });

  it("keeps just-ci as a fail-closed aggregate only", () => {
    const ciJob = workflow.jobs.ci;
    expect(ciJob).toBeDefined();
    expect(ciJob.name).toBe("just-ci");
    expect(ciJob.needs).toEqual(requiredLaneIds);
    expect(ciJob.if).toBe("${{ always() }}");
    expect(ciJob["continue-on-error"]).toBeUndefined();
    expect(ciJob["runs-on"]).toBe("ubuntu-22.04");
    expect(ciJob["timeout-minutes"]).toBe(5);
    expect(ciJob.steps).toHaveLength(1);
    expect(findStep(ciJob, (step) => step.uses !== undefined)).toBeUndefined();
    const aggregate = findStep(ciJob, (step) => step.name === "Require every required CI lane");
    for (const jobId of requiredLaneIds) {
      const expression = jobId === "common"
        ? `needs.${jobId}.result`
        : `needs['${jobId}'].result`;
      expect(aggregate?.run).toContain(expression);
    }
    expect(aggregate?.run).toContain('!= "success"');
    expect(aggregate?.run).toContain("exit 1");
  });

  it("runs the dependency audit as a blocking job", () => {
    const supplyChain = workflow.jobs["supply-chain"];
    expect(supplyChain).toBeDefined();
    expect(supplyChain.name).toBe("supply-chain");
    expect(supplyChain["runs-on"]).toBe("ubuntu-22.04");
    expect(supplyChain.if).toBeUndefined();
    expect(supplyChain["continue-on-error"]).toBeUndefined();
    expect(
      findStep(supplyChain, (step) => step.run?.includes("pnpm audit:dependencies") ?? false),
    ).toBeDefined();
  });

  it("limits smoke packaging to direct manual dispatches", () => {
    for (const jobId of packageJobIds) {
      expect(workflow.jobs[jobId]?.if, `${jobId} event guard`).toBe(packageEventGuard);
      expect(workflow.jobs[jobId]?.["timeout-minutes"], `${jobId} timeout`).toBeGreaterThan(0);
    }

    const windowsBuild = findStep(
      workflow.jobs["windows-package"],
      (step) => step.name === "Build Windows MSI smoke package",
    );
    expect(windowsBuild?.run).toContain("pnpm tauri build");
    expect(windowsBuild?.run).toContain("--target x86_64-pc-windows-msvc");
    expect(windowsBuild?.run).toContain("--bundles msi");
    expect(windowsBuild?.run).toContain("--no-sign");

    const linuxDeps = findStep(
      workflow.jobs["linux-package"],
      (step) => step.name === "Install Linux system deps",
    );
    expect(linuxDeps?.run).toContain("xdg-utils");

    const macSidecars = findStep(
      workflow.jobs["macos-package"],
      (step) => step.name === "Build universal sidecars",
    );
    expect(macSidecars?.run?.match(/--bin release-signature-verifier/g)).toHaveLength(2);
    expect(macSidecars?.run).toContain(
      "-output src-tauri/target/universal-apple-darwin/release/release-signature-verifier",
    );
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
