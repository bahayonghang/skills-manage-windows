/// <reference types="node" />

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { parse } from "yaml";

type Step = {
  name?: string;
  run?: string;
  uses?: string;
  with?: Record<string, unknown>;
  env?: Record<string, string>;
  shell?: string;
};
type Job = {
  needs?: string | string[];
  if?: string;
  environment?: string;
  permissions?: Record<string, string>;
  uses?: string;
  with?: Record<string, unknown>;
  steps?: Step[];
  "timeout-minutes"?: number;
  strategy?: {
    "fail-fast"?: boolean;
    matrix?: { installer?: string[] };
  };
};
type WorkflowTrigger = {
  inputs?: Record<string, { required?: boolean; default?: string; type?: string }>;
  tags?: string[];
};
type Workflow = {
  on: Record<string, WorkflowTrigger | undefined>;
  permissions?: Record<string, string>;
  jobs: Record<string, Job>;
};

const source = readFileSync(".github/workflows/release-desktop.yml", "utf8");
const assetScriptSources = [
  readFileSync("scripts/release/generate-latest-json.mjs", "utf8"),
  readFileSync("scripts/release/release-preflight.mjs", "utf8"),
].join("\n");
const releaseContextSource = readFileSync("scripts/release/release-context.mjs", "utf8");
const workflow = parse(source) as Workflow;
const needs = (job: Job) => (Array.isArray(job.needs) ? job.needs : job.needs ? [job.needs] : []);
const packageJson = JSON.parse(readFileSync("package.json", "utf8")) as { engines?: { node?: string } };
const rustToolchain = readFileSync("rust-toolchain.toml", "utf8");
const expectedNodeEngine = packageJson.engines?.node ?? "";
const expectedNodeMajor = expectedNodeEngine.replace(/\.x$/, "");
const expectedRustChannel = /^channel\s*=\s*"([^"]+)"/m.exec(rustToolchain)?.[1] ?? "";

function stepIndex(steps: Step[], predicate: (step: Step) => boolean) {
  return steps.findIndex(predicate);
}

describe("release workflow contract", () => {
  it("starts from an explicit tag and validates the frozen SHA through reusable CI", () => {
    expect(workflow.on.release).toBeUndefined();
    expect(workflow.on.push).toEqual({ tags: ["v*"] });
    expect(workflow.on.workflow_dispatch?.inputs?.mode.default).toBe("rehearsal");
    expect(workflow.on.workflow_dispatch?.inputs?.rehearsal_ref.type).toBe("string");
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
    expect(contextSteps[checkoutTag].run).toContain("--mode");
    expect(contextSteps[checkoutTag].run).toContain("--sha");
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
      "windows-install-smoke",
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
    expect(workflow.jobs["build-windows"].environment).toBe("desktop-signing");
    expect(workflow.jobs.publish.environment).toBe("desktop-release");
    expect(workflow.jobs.publish.permissions).toEqual({ contents: "write", "id-token": "write", attestations: "write" });
    expect(workflow.jobs.publish.if).toContain("mode == 'publish'");
    expect(source).not.toContain("always()");
    expect(source.match(/draft=false/g)).toHaveLength(1);
    const steps = workflow.jobs.publish.steps ?? [];
    const verifyTag = steps.findIndex((step) => step.name === "Verify remote tag target");
    const draft = steps.findIndex((step) => step.name === "Create or reuse same-tag draft");
    const upload = steps.findIndex((step) => step.name === "Upload validated assets");
    const fresh = steps.findIndex((step) => step.name === "Fresh-download and verify checksums and provenance");
    const attest = steps.findIndex((step) => step.uses?.startsWith("actions/attest@"));
    const publish = steps.findIndex((step) => step.run?.includes("draft=false"));
    expect(verifyTag).toBeGreaterThan(-1);
    expect(steps[verifyTag].run).toContain('git rev-parse "${RELEASE_TAG}^{commit}"');
    expect(draft).toBeGreaterThan(verifyTag);
    expect(source).not.toContain(".target_commitish == $sha");
    expect(upload).toBeGreaterThan(-1);
    expect(attest).toBeGreaterThan(-1);
    expect(steps.filter((step) => step.uses?.startsWith("actions/attest@"))).toHaveLength(1);
    expect(steps[attest].with?.["predicate-type"]).toBeUndefined();
    expect(steps[attest].with?.predicate).toBeUndefined();
    expect(steps[attest].with?.["create-storage-record"]).toBe(false);
    expect(steps.find((step) => step.name === "Generate build provenance predicate")).toBeUndefined();
    expect(fresh).toBeGreaterThan(upload);
    expect(publish).toBeGreaterThan(fresh);
    expect(steps[publish].run).toContain('git rev-parse "${RELEASE_TAG}^{commit}"');
    expect(steps[publish].run?.indexOf("git rev-parse")).toBeLessThan(
      steps[publish].run?.indexOf("draft=false") ?? -1,
    );
  });

  it("keeps rehearsal side-effect free while requiring the Windows smoke contract", () => {
    expect(releaseContextSource).toContain("Rehearsal ref must be an exact 40-character commit SHA");
    expect(workflow.jobs["windows-install-smoke"].if).toContain("workflow_dispatch");
    expect(needs(workflow.jobs["windows-install-smoke"])).toEqual(["release-context", "build-windows"]);
    expect(workflow.jobs.publish.steps?.some((step) => step.run?.includes("gh attestation verify"))).toBe(true);
    expect(source).toContain("authenticode=not-configured");
    expect(source).toContain("pnpm tauri signer sign");
    expect(source).not.toContain("Select-Object -Single");
    const macPrepare = workflow.jobs["build-macos"].steps?.find(
      (step) => step.name === "Prepare macOS assets",
    );
    expect(macPrepare?.run).not.toContain("mapfile");
    expect(macPrepare?.run).toContain("shopt -s nullglob");
    expect(macPrepare?.run).toContain('APPS=("$BUNDLE_ROOT/macos"/*.app)');
    expect(macPrepare?.run).toContain('DMGS=("$BUNDLE_ROOT/dmg"/*.dmg)');
    expect(source).toContain('Expected exactly one final NSIS asset');
  });

  it("builds every macOS universal sidecar and installs Linux bundle dependencies", () => {
    const macSidecars = workflow.jobs["build-macos"].steps?.find(
      (step) => step.name === "Build universal CLI",
    );
    expect(macSidecars?.run?.match(/--bin release-signature-verifier/g)).toHaveLength(2);
    expect(macSidecars?.run).toContain(
      "-output src-tauri/target/universal-apple-darwin/release/release-signature-verifier",
    );

    const linuxDeps = workflow.jobs["build-linux"].steps?.find(
      (step) => step.name === "Install Linux system deps",
    );
    expect(linuxDeps?.run).toContain("xdg-utils");
  });

  it("pins release-context toolchains by parsed step index before resolver use", () => {
    expect(expectedNodeEngine).toMatch(/^\d+\.x$/);
    expect(expectedRustChannel).toMatch(/^\d+\.\d+\.\d+$/);
    const contextJob = workflow.jobs["release-context"];
    const contextSteps = contextJob.steps ?? [];
    const windowsSteps = workflow.jobs["build-windows"].steps ?? [];
    const pinnedNode = windowsSteps.find((step) => step.uses?.startsWith("actions/setup-node@"));
    const pinnedRust = windowsSteps.find((step) => step.uses?.startsWith("dtolnay/rust-toolchain@"));
    expect(pinnedNode?.uses).toMatch(/^actions\/setup-node@[0-9a-f]{40}$/);
    expect(pinnedRust?.uses).toMatch(/^dtolnay\/rust-toolchain@[0-9a-f]{40}$/);

    const nodeSetup = stepIndex(contextSteps, (step) => step.uses?.startsWith("actions/setup-node@") ?? false);
    const nodeAssert = stepIndex(
      contextSteps,
      (step) =>
        typeof step.run === "string" &&
        !step.run.includes("release-context.mjs") &&
        (step.run.includes("node --version") || step.run.includes("process.versions.node")),
    );
    const firstResolver = stepIndex(contextSteps, (step) => step.run?.includes("release-context.mjs") ?? false);
    const resolveOnly = stepIndex(
      contextSteps,
      (step) => (step.run?.includes("release-context.mjs") && step.run.includes("--resolve-only")) ?? false,
    );
    const rustSetup = stepIndex(contextSteps, (step) => step.uses?.startsWith("dtolnay/rust-toolchain@") ?? false);
    const rustAssert = stepIndex(
      contextSteps,
      (step) =>
        typeof step.run === "string" &&
        !step.run.includes("release-context.mjs") &&
        step.run.includes("rustc --version"),
    );
    const fullResolver = stepIndex(
      contextSteps,
      (step) => (step.run?.includes("release-context.mjs") && !step.run.includes("--resolve-only")) ?? false,
    );

    expect(nodeSetup).toBeGreaterThan(-1);
    expect(nodeAssert).toBeGreaterThan(nodeSetup);
    expect(firstResolver).toBeGreaterThan(nodeAssert);
    expect(resolveOnly).toBe(firstResolver);
    expect(rustSetup).toBeGreaterThan(resolveOnly);
    expect(rustAssert).toBeGreaterThan(rustSetup);
    expect(fullResolver).toBeGreaterThan(rustAssert);

    expect(contextSteps[nodeSetup].uses).toBe(pinnedNode?.uses);
    expect(contextSteps[nodeSetup].with?.["node-version"]).toBe(expectedNodeMajor);
    expect(contextSteps[nodeAssert].run).toContain(expectedNodeEngine);
    expect(contextSteps[rustSetup].uses).toBe(pinnedRust?.uses);
    expect(contextSteps[rustSetup].with?.toolchain).toBe(expectedRustChannel);
    expect(contextSteps[rustAssert].run).toContain(expectedRustChannel);
    expect(contextJob["timeout-minutes"]).toBe(15);
  });

  it("bounds the Windows installer matrix and invokes the smoke helper", () => {
    const smoke = workflow.jobs["windows-install-smoke"];
    expect(smoke["timeout-minutes"]).toBe(20);
    expect(smoke.strategy?.matrix?.installer).toEqual(["nsis", "msi"]);
    expect(needs(workflow.jobs.aggregate)).toContain("windows-install-smoke");
    const helper = (smoke.steps ?? []).find((step) => step.run?.includes("scripts/release/windows-installer-smoke.ps1"));
    expect(helper).toBeDefined();
    expect(helper?.env?.INSTALLER_KIND).toBe("${{ matrix.installer }}");
    expect(helper?.run).toContain("-InstallerKind");
    expect(helper?.run).toContain("-ArtifactPath");
    expect(helper?.run).toContain("-ExpectedVersion");
    expect(helper?.run).toContain("-InstallRoot");
    expect(helper?.run).toContain("RUNNER_TEMP");
    expect(helper?.run).toContain("-SigningStatePath");
    expect(helper?.run).not.toContain("Start-Process -Wait");
    expect(source).toContain('Expected exactly one final NSIS asset');
    expect(source).toContain('Expected exactly one final MSI asset');
  });
});
