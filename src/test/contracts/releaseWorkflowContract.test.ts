/// <reference types="node" />

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

type Step = { name?: string; run?: string; uses?: string; with?: Record<string, unknown> };
type Job = {
  needs?: string | string[];
  permissions?: Record<string, string>;
  uses?: string;
  with?: Record<string, unknown>;
  steps?: Step[];
};
type WorkflowTrigger = {
  inputs?: Record<string, { required?: boolean }>;
  tags?: string[];
};
type Workflow = {
  on: Record<string, WorkflowTrigger | undefined>;
  permissions?: Record<string, string>;
  jobs: Record<string, Job>;
};

const source = readFileSync(".github/workflows/release-desktop.yml", "utf8");
const assetScriptSources = [
  readFileSync("scripts/generate-latest-json.mjs", "utf8"),
  readFileSync("scripts/release-preflight.mjs", "utf8"),
].join("\n");
const workflow = parse(source) as Workflow;
const needs = (job: Job) => (Array.isArray(job.needs) ? job.needs : job.needs ? [job.needs] : []);

describe("release workflow contract", () => {
  it("starts from an explicit tag and validates the frozen SHA through reusable CI", () => {
    expect(workflow.on.release).toBeUndefined();
    expect(workflow.on.push).toEqual({ tags: ["v*"] });
    expect(workflow.on.workflow_dispatch?.inputs?.tag.required).toBe(true);
    expect(workflow.jobs["quality-gate"].uses).toBe("./.github/workflows/ci.yml");
    expect(workflow.jobs["quality-gate"].with?.checkout_ref).toBe(
      "${{ needs.release-context.outputs.sha }}",
    );
    const contextSteps = workflow.jobs["release-context"].steps ?? [];
    const manualSource = contextSteps.find((step) => step.name === "Require canonical manual workflow source");
    expect(manualSource?.run).toContain('refs/heads/main');
    const checkoutTag = contextSteps.findIndex((step) => step.name === "Checkout frozen tag commit");
    const validateContext = contextSteps.findIndex((step) => step.name === "Resolve and validate release context");
    expect(checkoutTag).toBeGreaterThan(-1);
    expect(contextSteps[checkoutTag].run).toContain("--resolve-only");
    expect(contextSteps[checkoutTag].run).toContain("git checkout --detach");
    expect(validateContext).toBeGreaterThan(checkoutTag);
    expect(source).not.toContain("GITHUB_REF_NAME");
    expect(assetScriptSources).not.toContain("GITHUB_REF_NAME");
  });

  it("binds every required build to context and CI before aggregate", () => {
    for (const id of ["build-windows", "build-macos", "build-linux"]) {
      expect(needs(workflow.jobs[id])).toEqual(["release-context", "quality-gate"]);
      const checkout = workflow.jobs[id].steps?.find((step) => step.uses?.startsWith("actions/checkout@"));
      expect(checkout?.with?.ref).toBe("${{ needs.release-context.outputs.sha }}");
    }
    expect(needs(workflow.jobs.aggregate)).toEqual([
      "release-context",
      "quality-gate",
      "build-windows",
      "build-macos",
      "build-linux",
    ]);
    expect(needs(workflow.jobs.publish)).toEqual(["release-context", "aggregate"]);
  });

  it("cannot schedule draft creation when any required predecessor fails", () => {
    const schedulable = (results: Record<string, string>) =>
      needs(workflow.jobs.aggregate).every((id) => results[id] === "success");
    const success = Object.fromEntries(needs(workflow.jobs.aggregate).map((id) => [id, "success"]));
    expect(schedulable(success)).toBe(true);
    for (const failed of Object.keys(success)) {
      expect(schedulable({ ...success, [failed]: "failure" }), failed).toBe(false);
    }
  });

  it("keeps the sole public transition after upload and fresh verification", () => {
    expect(workflow.permissions).toEqual({ contents: "read" });
    expect(workflow.jobs.publish.permissions).toEqual({ contents: "write" });
    expect(source).not.toContain("always()");
    expect(source.match(/draft=false/g)).toHaveLength(1);
    const steps = workflow.jobs.publish.steps ?? [];
    const verifyTag = steps.findIndex((step) => step.name === "Verify remote tag target");
    const draft = steps.findIndex((step) => step.name === "Create or reuse same-tag draft");
    const upload = steps.findIndex((step) => step.name === "Upload validated assets");
    const fresh = steps.findIndex((step) => step.name === "Fresh-download and verify checksums");
    const publish = steps.findIndex((step) => step.run?.includes("draft=false"));
    expect(verifyTag).toBeGreaterThan(-1);
    expect(steps[verifyTag].run).toContain('git rev-parse "${RELEASE_TAG}^{commit}"');
    expect(draft).toBeGreaterThan(verifyTag);
    expect(source).not.toContain(".target_commitish == $sha");
    expect(upload).toBeGreaterThan(-1);
    expect(fresh).toBeGreaterThan(upload);
    expect(publish).toBeGreaterThan(fresh);
    expect(steps[publish].run).toContain('git rev-parse "${RELEASE_TAG}^{commit}"');
    expect(steps[publish].run?.indexOf("git rev-parse")).toBeLessThan(
      steps[publish].run?.indexOf("draft=false") ?? -1,
    );
  });
});
