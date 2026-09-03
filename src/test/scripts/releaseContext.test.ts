/// <reference types="node" />

import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

type ExecOptions = {
  timeoutMs?: number;
  maxBuffer?: number;
};

type ExecFn = (command: string, args: string[], cwd: string, options?: ExecOptions) => string;

type ReleaseContextHelpers = {
  parseReleaseTag: (tag: string) => { tag: string; version: string };
  resolveReleaseTag: (args: {
    tag: string;
    cwd: string;
    exec?: ExecFn;
  }) => { tag: string; version: string; sha: string };
  validateVersionSet: (versions: {
    tag: string;
    packageVersion: string;
    tauriVersion: string;
    cargoVersion: string;
  }) => { tag: string; version: string };
  resolveRehearsalRef: (args: {
    sha: string;
    cwd: string;
    exec?: ExecFn;
  }) => { sha: string };
  run: ExecFn;
  RESOLVER_SUBPROCESS_TIMEOUT_MS: number;
  RESOLVER_OUTPUT_CAP_BYTES: number;
};

// @ts-expect-error The release context helper is an ESM Node script outside the TS source tree.
const releaseContextModule = await import("../../../scripts/release/release-context.mjs");
const {
  parseReleaseTag,
  resolveReleaseTag,
  resolveRehearsalRef,
  validateVersionSet,
  run,
  RESOLVER_SUBPROCESS_TIMEOUT_MS,
  RESOLVER_OUTPUT_CAP_BYTES,
} = releaseContextModule as ReleaseContextHelpers;

describe("release context", () => {
  it("accepts explicit semver tags and derives the version", () => {
    expect(parseReleaseTag("v1.2.3")).toEqual({ tag: "v1.2.3", version: "1.2.3" });
    expect(parseReleaseTag("v1.2.3-rc.1")).toEqual({
      tag: "v1.2.3-rc.1",
      version: "1.2.3-rc.1",
    });
  });

  it.each(["1.2.3", "v01.2.3", "main", "", "v1.2"])("rejects invalid tag %s", (tag) => {
    expect(() => parseReleaseTag(tag)).toThrow(/v<semver>/);
  });

  it("requires every package version to match the frozen tag", () => {
    expect(() =>
      validateVersionSet({
        tag: "v1.2.3",
        packageVersion: "1.2.3",
        tauriVersion: "1.2.4",
        cargoVersion: "1.2.3",
      }),
    ).toThrow(/tauri\.conf\.json version mismatch/);
  });

  it("uses a peeled tag commit, origin/main ancestry, and locked Cargo metadata", () => {
    const calls: string[][] = [];
    expect(resolveReleaseTag({
      tag: "v1.2.3",
      cwd: "repo",
      exec: (_command, args) => {
        calls.push(args);
        return args[0] === "rev-parse" ? "abc123" : "";
      },
    })).toEqual({ tag: "v1.2.3", version: "1.2.3", sha: "abc123" });
    expect(calls).toEqual([
      ["rev-parse", "v1.2.3^{commit}"],
      ["merge-base", "--is-ancestor", "abc123", "origin/main"],
    ]);

    const source = readFileSync("scripts/release/release-context.mjs", "utf8");
    expect(source).toContain('"--locked"');
  });

  it("accepts only an exact main-ancestor SHA for manual rehearsal", () => {
    const sha = "a".repeat(40);
    expect(resolveRehearsalRef({
      sha,
      cwd: "repo",
      exec: (_command, args) => args[0] === "rev-parse" ? sha : "",
    })).toEqual({ sha });
    expect(() => resolveRehearsalRef({ sha: "main", cwd: "repo", exec: () => "" })).toThrow(/40-character/);
  });

  it("bounds resolver subprocesses and maps timeout without waiting for the job", () => {
    expect(RESOLVER_SUBPROCESS_TIMEOUT_MS).toBe(120_000);
    expect(RESOLVER_OUTPUT_CAP_BYTES).toBe(1_048_576);
    const started = Date.now();
    expect(() =>
      run(process.execPath, ["-e", "setTimeout(() => {}, 60_000)"], process.cwd(), { timeoutMs: 400 }),
    ).toThrow(/release-context: .+ timed out after 400ms\./);
    expect(Date.now() - started).toBeLessThan(10_000);
  }, 15_000);

  it("maps oversized resolver output to a staged error", () => {
    expect(() =>
      run(process.execPath, ["-e", "process.stdout.write('x'.repeat(65536))"], process.cwd(), { maxBuffer: 1024 }),
    ).toThrow(/release-context: .+ output exceeded 1024 bytes\./);
  });
});
