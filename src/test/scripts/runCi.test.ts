/// <reference types="node" />

import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

type CiStep = { args: string[]; command: string; name: string };
type ExecuteStep = (context: {
  lane: string;
  signal: AbortSignal;
  step: CiStep;
}) => Promise<void>;
type RunCiHelpers = {
  CI_LANES: Record<string, CiStep[]>;
  parseCiArgs: (args: string[]) => { lane: string };
  spawnStep: (context: {
    lane: string;
    signal: AbortSignal;
    step: CiStep;
    writeError: (message: string) => void;
    writeLine: (message: string) => void;
  }) => Promise<void>;
  runCi: (options?: {
    executeStep?: ExecuteStep;
    lane?: string;
    now?: () => number;
    summaryPath?: string | null;
    writeError?: (message: string) => void;
    writeLine?: (message: string) => void;
  }) => Promise<void>;
};

// @ts-expect-error The CI runner is an ESM Node script outside the TS source tree.
const { CI_LANES, parseCiArgs, runCi, spawnStep } = (await import("../../../scripts/run-ci.mjs")) as RunCiHelpers;

const tempDirs: string[] = [];

afterEach(() => {
  for (const tempDir of tempDirs.splice(0)) {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

describe("CI command lanes", () => {
  it("keeps quick, common, and platform ownership explicit", () => {
    const packageJson = JSON.parse(readFileSync("package.json", "utf8")) as {
      scripts: Record<string, string>;
    };

    expect(CI_LANES.quick[0]).toEqual({
      name: "version",
      command: "pnpm",
      args: ["version:check"],
    });
    expect(CI_LANES.quick.map((step) => step.name)).toEqual([
      "version",
      "docs-generated",
      "typecheck",
      "lint",
      "capability",
      "size",
      "entrypoints",
      "fmt",
    ]);
    expect(CI_LANES.common.map((step) => step.name)).toEqual([
      ...CI_LANES.quick.map((step) => step.name),
      "ipc-codegen",
      "test",
      "build",
      "docs",
    ]);
    expect(CI_LANES.common[CI_LANES.common.length - 1]).toEqual({
      name: "docs",
      command: "pnpm",
      args: ["docs:site:build"],
    });
    expect(packageJson.scripts["docs:site:build"]).toBe("vitepress build docs");
    expect(packageJson.scripts["docs:build"]).toBe(
      "pnpm docs:gen:check && pnpm docs:site:build",
    );
    expect(CI_LANES["rust-platform"].map((step) => step.name)).toEqual(["clippy", "test"]);
  });

  it("selects one lane or the default all lanes", async () => {
    const calls: string[] = [];
    const executeStep: ExecuteStep = async ({ lane, step }) => {
      calls.push(`${lane}:${step.name}`);
    };

    await runCi({ lane: "quick", executeStep, summaryPath: null });
    expect(calls).toEqual(CI_LANES.quick.map((step) => `quick:${step.name}`));

    calls.length = 0;
    await runCi({ executeStep, summaryPath: null });
    expect(calls).toEqual(expect.arrayContaining([
      ...CI_LANES.common.map((step) => `common:${step.name}`),
      ...CI_LANES["rust-platform"].map((step) => `rust-platform:${step.name}`),
    ]));
    expect(calls).toHaveLength(CI_LANES.common.length + CI_LANES["rust-platform"].length);

    calls.length = 0;
    await runCi({ lane: "all", executeStep, summaryPath: null });
    expect(calls).toEqual(expect.arrayContaining([
      ...CI_LANES.common.map((step) => `common:${step.name}`),
      ...CI_LANES["rust-platform"].map((step) => `rust-platform:${step.name}`),
    ]));
    expect(calls).toHaveLength(CI_LANES.common.length + CI_LANES["rust-platform"].length);
  });

  it("rejects unknown or malformed lane arguments", async () => {
    expect(parseCiArgs([])).toEqual({ lane: "all" });
    expect(parseCiArgs(["--lane", "common"])).toEqual({ lane: "common" });
    expect(parseCiArgs(["--lane", "all"])).toEqual({ lane: "all" });
    expect(() => parseCiArgs(["--lane", "missing"])).toThrowError(/Unknown CI lane: missing/);
    expect(() => parseCiArgs(["common"])).toThrowError(/Usage:/);
    await expect(runCi({ lane: "missing", summaryPath: null })).rejects.toThrowError(
      /Unknown CI lane: missing/,
    );

    const cli = spawnSync(
      process.execPath,
      ["scripts/run-ci.mjs", "--lane", "missing"],
      { cwd: process.cwd(), encoding: "utf8" },
    );
    expect(cli.status).toBe(1);
    expect(cli.stderr).toContain("Unknown CI lane: missing");
  });

  it("stops a lane after its first failed step", async () => {
    const calls: string[] = [];
    const executeStep: ExecuteStep = async ({ step }) => {
      calls.push(step.name);
      if (step.name === "typecheck") throw new Error("typecheck failed");
    };

    await expect(runCi({ lane: "common", executeStep, summaryPath: null })).rejects.toThrowError(
      "typecheck failed",
    );
    expect(calls).toEqual(["version", "docs-generated", "typecheck"]);
  });

  it("aborts a running sibling lane after the first failure", async () => {
    let startPlatform!: () => void;
    const platformStarted = new Promise<void>((resolve) => {
      startPlatform = resolve;
    });
    let platformAborted = false;
    const executeStep: ExecuteStep = async ({ lane, signal }) => {
      if (lane === "common") {
        await platformStarted;
        throw new Error("common failed");
      }

      startPlatform();
      await new Promise<void>((_resolve, reject) => {
        signal.addEventListener("abort", () => {
          platformAborted = true;
          reject(new Error("platform aborted"));
        }, { once: true });
      });
    };

    await expect(runCi({ executeStep, summaryPath: null })).rejects.toThrowError("common failed");
    expect(platformAborted).toBe(true);
  });

  it("terminates the spawned command tree after cancellation", async () => {
    const grandchildScript = "setInterval(() => undefined, 1000);";
    const parentScript = [
      "const { spawn } = require('node:child_process');",
      `const child = spawn(process.execPath, ['-e', ${JSON.stringify(grandchildScript)}], { stdio: 'ignore' });`,
      "console.log(`ready:${child.pid}`);",
      "setInterval(() => undefined, 1000);",
    ].join("");
    const controller = new AbortController();
    let markReady!: (pid: number) => void;
    const ready = new Promise<number>((resolve) => {
      markReady = resolve;
    });

    const execution = spawnStep({
      lane: "test",
      step: { name: "process-tree", command: process.execPath, args: ["-e", parentScript] },
      signal: controller.signal,
      writeLine: (message) => {
        const match = message.match(/ready:(\d+)/);
        if (match) markReady(Number(match[1]));
      },
      writeError: () => undefined,
    });

    const grandchildPid = await ready;
    controller.abort(new Error("test cancellation"));
    await expect(execution).rejects.toThrowError(/stopped after sibling failure/);

    const processExists = () => {
      try {
        process.kill(grandchildPid, 0);
        return true;
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ESRCH") return false;
        throw error;
      }
    };
    const deadline = Date.now() + 2_000;
    while (processExists() && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    expect(processExists()).toBe(false);
  });

  it("writes deterministic step timing and lane results to the job summary", async () => {
    const rootDir = mkdtempSync(join(tmpdir(), "skillport-ci-summary-"));
    tempDirs.push(rootDir);
    const summaryPath = join(rootDir, "summary.md");
    let clock = 0;
    const now = () => {
      clock += 1_000;
      return clock;
    };

    await runCi({
      lane: "rust-platform",
      executeStep: async () => undefined,
      now,
      summaryPath,
      writeLine: () => undefined,
      writeError: () => undefined,
    });

    const summary = readFileSync(summaryPath, "utf8");
    expect(summary).toContain("## CI lane summary");
    expect(summary).toContain("| rust-platform | clippy | success | 1.0 |");
    expect(summary).toContain("| rust-platform | test | success | 1.0 |");
    expect(summary).toContain("| rust-platform | success |");
  });
});
