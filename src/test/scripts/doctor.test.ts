/// <reference types="node" />

import { describe, expect, it } from "vitest";

type CommandResult = {
  error?: NodeJS.ErrnoException | null;
  status: number | null;
  stderr?: string;
  stdout?: string;
};

type DoctorOptions = {
  commandRunner: (command: string, args: string[]) => CommandResult;
  env?: Record<string, string>;
  fileExists?: (path: string) => boolean;
  platform?: string;
  writeLine?: (line: string) => void;
};

// @ts-expect-error The doctor runner is an ESM Node script outside the TS source tree.
const { collectDoctorChecks, redactSecrets, runDoctor, TOOLCHAIN } = (await import("../../../scripts/check/doctor.mjs")) as {
  collectDoctorChecks: (options: DoctorOptions) => Array<{
    actual: string;
    id: string;
    status: string;
  }>;
  redactSecrets: (value: string, env?: Record<string, string>) => string;
  runDoctor: (options: DoctorOptions) => { ok: boolean; failures: Array<unknown> };
  TOOLCHAIN: { nodeMajor: number; pnpm: string; rust: string };
};

function commandResults(overrides: Record<string, CommandResult> = {}) {
  const defaults: Record<string, CommandResult> = {
    [process.execPath]: { status: 0, stdout: "v26.7.0\n" },
    pnpm: { status: 0, stdout: `${TOOLCHAIN.pnpm}\n` },
    rustc: { status: 0, stdout: `rustc ${TOOLCHAIN.rust} (stable)\n` },
    cargo: { status: 0, stdout: `cargo ${TOOLCHAIN.rust} (stable)\n` },
    just: { status: 0, stdout: "just 1.57.0\n" },
    git: { status: 0, stdout: "git version 2.55.0\n" },
    rustup: { status: 0, stdout: "x86_64-pc-windows-msvc\n" },
  };
  return (command: string) => overrides[command] ?? defaults[command] ?? {
    status: null,
    stdout: "",
    stderr: "",
    error: Object.assign(new Error("command not found"), { code: "ENOENT" }),
  };
}

describe("doctor", () => {
  it("passes when the declared toolchain and Windows prerequisites are available", () => {
    const result = runDoctor({
      platform: "win32",
      commandRunner: commandResults(),
      fileExists: () => true,
      writeLine: () => undefined,
    });

    expect(result.ok).toBe(true);
    expect(result.failures).toHaveLength(0);
  });

  it("distinguishes a missing command from a version mismatch", () => {
    const checks = collectDoctorChecks({
      platform: "linux",
      commandRunner: commandResults({
        pnpm: {
          status: null,
          stdout: "",
          stderr: "",
          error: Object.assign(new Error("spawn pnpm ENOENT"), { code: "ENOENT" }),
        },
        rustc: { status: 0, stdout: "rustc 1.96.0 (old)\n" },
      }),
    });

    expect(checks.find((check) => check.id === "pnpm")).toMatchObject({ status: "missing" });
    expect(checks.find((check) => check.id === "rustc")).toMatchObject({ status: "mismatch" });
  });

  it("checks Node by major and pnpm/Rust by exact version", () => {
    const checks = collectDoctorChecks({
      platform: "linux",
      commandRunner: commandResults({
        [process.execPath]: { status: 0, stdout: "v25.9.0\n" },
        pnpm: { status: 0, stdout: "11.19.0\n" },
      }),
    });

    expect(checks.find((check) => check.id === "node")).toMatchObject({ status: "mismatch", actual: "25.9.0" });
    expect(checks.find((check) => check.id === "pnpm")).toMatchObject({ status: "mismatch", actual: "11.19.0" });
    expect(checks.find((check) => check.id === "rustc")).toMatchObject({ status: "ok", actual: TOOLCHAIN.rust });
  });

  it("redacts credential-shaped output and environment values", () => {
    const secret = "github_pat_0123456789abcdefghijklmnopqrstuvwxyz";
    const output = redactSecrets(`token=${secret}; bearer ghp_abcdefghijklmnopqrstuvwxyz`, {
      GH_TOKEN: secret,
    });

    expect(output).not.toContain(secret);
    expect(output).not.toContain("ghp_abcdefghijklmnopqrstuvwxyz");
    expect(output).toContain("[REDACTED]");
  });

  it("redacts credential-shaped command errors before reporting them", () => {
    const secret = "github_pat_0123456789abcdefghijklmnopqrstuvwxyz";
    const checks = collectDoctorChecks({
      platform: "linux",
      env: { GH_TOKEN: secret },
      commandRunner: commandResults({
        pnpm: {
          status: null,
          stdout: "",
          stderr: "",
          error: Object.assign(new Error(`token=${secret}`), { code: "EACCES" }),
        },
      }),
    });

    expect(checks.find((check) => check.id === "pnpm")?.actual).not.toContain(secret);
  });
});
