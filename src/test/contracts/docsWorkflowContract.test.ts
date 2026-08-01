/// <reference types="node" />

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

type WorkflowStep = {
  id?: string;
  if?: string;
  name?: string;
  run?: string;
  uses?: string;
  with?: Record<string, unknown>;
};

type WorkflowJob = {
  environment?: { name?: string; url?: string };
  env?: Record<string, string>;
  if?: string;
  needs?: string | string[];
  outputs?: Record<string, string>;
  permissions?: Record<string, string>;
  steps?: WorkflowStep[];
  "timeout-minutes"?: number;
};

type Workflow = {
  concurrency?: { group?: string; "cancel-in-progress"?: boolean };
  jobs: Record<string, WorkflowJob>;
  on: Record<string, { types?: string[] } | null | undefined>;
  permissions?: Record<string, string>;
};

const workflow = parse(readFileSync(".github/workflows/docs.yml", "utf8")) as Workflow;

function steps(job: WorkflowJob) {
  return job.steps ?? [];
}

function findStep(job: WorkflowJob, predicate: (step: WorkflowStep) => boolean) {
  return steps(job).find(predicate);
}

function allSteps() {
  return Object.values(workflow.jobs).flatMap(steps);
}

describe("documentation workflow contract", () => {
  it("shares one production path between releases and canonical main dispatches", () => {
    expect(workflow.on.release).toEqual({ types: ["published"] });
    expect(workflow.on.workflow_dispatch).toBeNull();
    expect(workflow.on.push).toBeUndefined();
    expect(workflow.concurrency?.["cancel-in-progress"]).toBe(false);

    const manualGuard = findStep(
      workflow.jobs.build,
      (step) => step.name === "Require canonical manual workflow source",
    );
    expect(manualGuard?.if).toBe("${{ github.event_name == 'workflow_dispatch' }}");
    expect(manualGuard?.run).toContain("refs/heads/main");
  });

  it("builds once and uploads one official Pages artifact", () => {
    const buildSteps = steps(workflow.jobs.build);
    expect(buildSteps.filter((step) => step.run === "pnpm docs:build")).toHaveLength(1);
    expect(allSteps().filter((step) => step.run === "pnpm docs:build")).toHaveLength(1);
    const configureSteps = buildSteps.filter((step) =>
      step.uses?.startsWith("actions/configure-pages@") ?? false,
    );
    expect(configureSteps).toHaveLength(1);
    const uploadSteps = buildSteps.filter((step) =>
      step.uses?.startsWith("actions/upload-pages-artifact@") ?? false,
    );
    expect(uploadSteps).toHaveLength(1);
    const upload = uploadSteps[0];
    expect(upload?.with?.path).toBe("dist-docs");
    expect(allSteps().some((step) => step.uses?.startsWith("actions/upload-artifact@"))).toBe(false);
    expect(allSteps().some((step) => step.uses?.startsWith("peaceiris/actions-gh-pages@"))).toBe(false);
  });

  it("deploys the built artifact with least privilege and no rebuild", () => {
    const deploy = workflow.jobs.deploy;
    expect(deploy.needs).toBe("build");
    expect(workflow.permissions).toEqual({ contents: "read" });
    expect(deploy.permissions).toEqual({ pages: "write", "id-token": "write" });
    expect(deploy.environment).toEqual({
      name: "github-pages",
      url: "${{ steps.deployment.outputs.page_url }}",
    });
    expect(deploy.outputs?.page_url).toBe("${{ steps.deployment.outputs.page_url }}");
    expect(
      steps(deploy).filter(
        (step) => step.id === "deployment"
          && (step.uses?.startsWith("actions/deploy-pages@") ?? false),
      ),
    ).toHaveLength(1);
    expect(
      steps(deploy).some(
        (step) =>
          (step.uses?.startsWith("actions/checkout@") ?? false)
          || step.run === "pnpm install --frozen-lockfile"
          || step.run === "pnpm docs:build",
      ),
    ).toBe(false);
  });

  it("smoke-tests the deployed canonical URL and SkillPort identity", () => {
    const smoke = workflow.jobs.smoke;
    expect(smoke.needs).toBe("deploy");
    expect(smoke.permissions).toEqual({});
    expect(smoke.env?.PAGES_URL).toBe("${{ needs.deploy.outputs.page_url }}");
    const run = findStep(smoke, (step) => step.name === "Verify deployed documentation")?.run ?? "";
    expect(run).toContain("https://bahayonghang.github.io/skills-manage-windows/");
    expect(run).toContain("200");
    expect(run).toContain("SkillPort");
    expect(run).toMatch(/for attempt in \{1\.\.[0-9]+\}/);

    const attempts = Number(run.match(/for attempt in \{1\.\.([0-9]+)\}/)?.[1]);
    const requestTimeout = Number(run.match(/--max-time ([0-9]+)/)?.[1]);
    const retryDelay = Number(run.match(/sleep ([0-9]+)/)?.[1]);
    const retryBudgetSeconds = attempts * requestTimeout + (attempts - 1) * retryDelay;
    expect(retryBudgetSeconds).toBeLessThan((smoke["timeout-minutes"] ?? 0) * 60);
  });

  it("pins every Pages workflow action to a full commit SHA", () => {
    for (const step of allSteps()) {
      if (!step.uses) continue;
      expect(step.uses).toMatch(
        /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+@[0-9a-f]{40}$/,
      );
    }
  });
});
