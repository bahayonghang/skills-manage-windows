/// <reference types="node" />

import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { delimiter, dirname, join, relative } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

type CommandResult = {
  error?: NodeJS.ErrnoException | null;
  status: number | null;
  stderr?: string;
  stdout?: string;
};

type CommandOptions = {
  cwd?: string;
  env?: NodeJS.ProcessEnv;
};

type CommandRunner = (
  command: string,
  args?: string[],
  options?: CommandOptions,
) => CommandResult;

type DoctorCheck = {
  actual: string;
  expected?: string;
  id: string;
  status: string;
};

type DoctorOptions = {
  commandRunner?: CommandRunner;
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  fileExists?: (path: string) => boolean;
  platform?: string;
  writeLine?: (line: string) => void;
};

// @ts-expect-error The doctor runner is an ESM Node script outside the TS source tree.
const {
  collectDoctorChecks,
  redactSecrets,
  runCommand,
  runDoctor,
  TOOLCHAIN,
  withPnpmReadonlyProbeEnv,
} = (await import("../../../scripts/check/doctor.mjs")) as {
  collectDoctorChecks: (options?: DoctorOptions) => DoctorCheck[];
  redactSecrets: (value: string, env?: NodeJS.ProcessEnv) => string;
  runCommand: CommandRunner;
  runDoctor: (options?: DoctorOptions) => { ok: boolean; failures: Array<unknown> };
  TOOLCHAIN: { nodeMajor: number; pnpm: string; rust: string };
  withPnpmReadonlyProbeEnv: (env?: NodeJS.ProcessEnv) => NodeJS.ProcessEnv;
};

const tempDirs: string[] = [];
const repoPackageJsonPath = join(process.cwd(), "package.json");

afterEach(() => {
  for (const tempDir of tempDirs.splice(0)) {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

function commandResults(overrides: Record<string, CommandResult> = {}): CommandRunner {
  const defaults: Record<string, CommandResult> = {
    [process.execPath]: { status: 0, stdout: "v26.7.0\n" },
    pnpm: { status: 0, stdout: `${TOOLCHAIN.pnpm}\n` },
    rustc: { status: 0, stdout: `rustc ${TOOLCHAIN.rust} (stable)\n` },
    cargo: { status: 0, stdout: `cargo ${TOOLCHAIN.rust} (stable)\n` },
    just: { status: 0, stdout: "just 1.57.0\n" },
    git: { status: 0, stdout: "git version 2.55.0\n" },
    rustup: { status: 0, stdout: "x86_64-pc-windows-msvc\n" },
  };
  return (command) => overrides[command] ?? defaults[command] ?? {
    status: null,
    stdout: "",
    stderr: "",
    error: Object.assign(new Error("command not found"), { code: "ENOENT" }),
  };
}

function makeTempDir(prefix: string) {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  tempDirs.push(dir);
  return dir;
}

function listRelativePaths(root: string) {
  const paths: string[] = [];
  const visit = (dir: string) => {
    if (!existsSync(dir)) return;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const fullPath = join(dir, entry.name);
      paths.push(relative(root, fullPath).replaceAll("\\", "/"));
      if (entry.isDirectory()) visit(fullPath);
    }
  };
  visit(root);
  return paths.sort();
}

function userHome() {
  return process.env.USERPROFILE ?? homedir();
}

function pathKey(env: NodeJS.ProcessEnv) {
  return Object.keys(env).find((key) => key.toLowerCase() === "path") ?? "PATH";
}

function prependPath(env: NodeJS.ProcessEnv, directory: string) {
  const next = { ...env };
  const key = pathKey(next);
  next[key] = `${directory}${delimiter}${next[key] ?? ""}`;
  return next;
}

function isolatedProbeEnv(root: string) {
  const cacheRoot = join(root, "cache");
  const pnpmHome = join(root, "pnpm-home");
  mkdirSync(cacheRoot, { recursive: true });
  mkdirSync(pnpmHome, { recursive: true });
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    PNPM_HOME: pnpmHome,
    XDG_CACHE_HOME: cacheRoot,
    npm_config_cache: join(cacheRoot, "npm"),
    pnpm_config_cache_dir: join(cacheRoot, "pnpm-cache"),
    pnpm_config_store_dir: join(cacheRoot, "pnpm-store"),
    pnpm_config_state_dir: join(cacheRoot, "pnpm-state"),
  };
  delete env.pnpm_config_pm_on_fail;
  delete env.PNPM_CONFIG_PM_ON_FAIL;
  return { cacheRoot, env, pnpmHome };
}

function writeManifest(cwd: string, packageManager: string) {
  mkdirSync(cwd, { recursive: true });
  writeFileSync(join(cwd, "package.json"), `${JSON.stringify({
    name: "doctor-pnpm-probe",
    private: true,
    packageManager,
  }, null, 2)}\n`);
}

function findPinnedPnpm10345() {
  const storeRoot = join(
    userHome(),
    "scoop",
    "apps",
    "pnpm",
    "current",
    "package-manager-store",
    "v11",
    "links",
    "@pnpm",
    "exe",
    "10.34.5",
  );
  if (!existsSync(storeRoot)) return undefined;
  for (const hash of readdirSync(storeRoot)) {
    const exe = join(storeRoot, hash, "node_modules", "@pnpm", "exe", "pnpm.exe");
    if (existsSync(exe)) return exe;
  }
  return undefined;
}

function scoopPnpm1234Dir() {
  const dir = join(userHome(), "scoop", "apps", "pnpm", "12.3.4");
  return existsSync(join(dir, "pnpm.exe")) ? dir : undefined;
}

function scoopPnpmEngineStoreRoot() {
  return join(
    userHome(),
    "scoop",
    "apps",
    "pnpm",
    "current",
    "package-manager-store",
  );
}

function scoopPnpmEngineVersions() {
  const dir = join(scoopPnpmEngineStoreRoot(), "v11", "links", "@pnpm", "exe");
  return existsSync(dir) ? readdirSync(dir).sort() : [];
}

function scoopPnpmEngineStoreMarkerNames() {
  const v11 = join(scoopPnpmEngineStoreRoot(), "v11");
  if (!existsSync(v11)) return [];
  return readdirSync(v11)
    .filter((name) => !name.startsWith("index.db"))
    .sort();
}

function capturingRunner(inner: CommandRunner = runCommand) {
  const calls: Array<{ args: string[]; command: string; options?: CommandOptions }> = [];
  const commandRunner: CommandRunner = (command, args = [], options) => {
    calls.push({ command, args, options });
    return inner(command, args, options);
  };
  return { calls, commandRunner };
}

function capturingPnpmProbeRunner() {
  const otherCommands = commandResults();
  return capturingRunner((command, args, options) => {
    if (command === "pnpm") {
      return runCommand(command, args, options);
    }
    return otherCommands(command, args, options);
  });
}

function packageFetchArtifacts(paths: string[]) {
  return paths.filter((path) => (
    path.includes("registry.npmjs.org")
    || path.endsWith("pnpm.jsonl")
    || path.includes("package-manager-store")
    || path.includes("pnpm-engine-")
    || (path.includes("@pnpm") && path.includes("exe"))
  ));
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
    expect(TOOLCHAIN.pnpm).toBe("10.34.5");
    const checks = collectDoctorChecks({
      platform: "linux",
      commandRunner: commandResults({
        [process.execPath]: { status: 0, stdout: "v25.9.0\n" },
        pnpm: { status: 0, stdout: "12.3.4\n" },
      }),
    });

    expect(checks.find((check) => check.id === "node")).toMatchObject({ status: "mismatch", actual: "25.9.0" });
    expect(checks.find((check) => check.id === "pnpm")).toMatchObject({
      status: "mismatch",
      actual: "12.3.4",
      expected: "10.34.5",
    });
    expect(checks.find((check) => check.id === "rustc")).toMatchObject({ status: "ok", actual: TOOLCHAIN.rust });
  });

  it("injects ignore into the pnpm probe only and leaves the parent environment unchanged", () => {
    const env: NodeJS.ProcessEnv = {
      GH_TOKEN: "not-a-secret-for-this-test",
      pnpm_config_pm_on_fail: "download",
      PNPM_CONFIG_PM_ON_FAIL: "download",
    };
    const parentBefore = process.env.pnpm_config_pm_on_fail;
    const { calls, commandRunner } = capturingRunner(commandResults());

    collectDoctorChecks({
      platform: "linux",
      env,
      commandRunner,
    });

    const pnpmCall = calls.find((call) => call.command === "pnpm");
    const nodeCall = calls.find((call) => call.command === process.execPath);
    const pnpmEnvKeys = Object.keys(pnpmCall?.options?.env ?? {})
      .filter((key) => key.toLowerCase() === "pnpm_config_pm_on_fail");
    expect(pnpmCall?.args).toEqual(["--version"]);
    expect(pnpmCall?.options?.env?.pnpm_config_pm_on_fail).toBe("ignore");
    expect(pnpmCall?.options?.env?.PNPM_CONFIG_PM_ON_FAIL).toBeUndefined();
    expect(pnpmEnvKeys).toEqual(["pnpm_config_pm_on_fail"]);
    expect(nodeCall?.options?.env?.pnpm_config_pm_on_fail).toBe("download");
    expect(nodeCall?.options?.env?.PNPM_CONFIG_PM_ON_FAIL).toBe("download");
    expect(env.pnpm_config_pm_on_fail).toBe("download");
    expect(env.PNPM_CONFIG_PM_ON_FAIL).toBe("download");
    expect(process.env.pnpm_config_pm_on_fail).toBe(parentBefore);
    expect(withPnpmReadonlyProbeEnv(env).PNPM_CONFIG_PM_ON_FAIL).toBeUndefined();
    expect(withPnpmReadonlyProbeEnv(env).pnpm_config_pm_on_fail).toBe("ignore");
  });

  it("classifies an owned subprocess timeout as a mismatch without raising the probe deadline", { timeout: 15_000 }, () => {
    const hang = runCommand(process.execPath, ["-e", "setTimeout(() => {}, 60_000)"]);
    expect(hang.error?.code).toBe("ETIMEDOUT");

    const checks = collectDoctorChecks({
      platform: "linux",
      commandRunner: commandResults({
        pnpm: hang,
      }),
    });

    const pnpm = checks.find((check) => check.id === "pnpm");
    expect(pnpm).toMatchObject({ status: "mismatch", expected: "10.34.5" });
    expect(pnpm?.actual).toContain("ETIMEDOUT");
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

  it("returns a pnpm 12 mismatch promptly without writing an isolated cache or repo bytes", () => {
    const pnpm12Dir = scoopPnpm1234Dir();
    expect(pnpm12Dir, "local pnpm 12.3.4 is required for the mismatch probe").toBeTruthy();

    const root = makeTempDir("doctor-pnpm-mismatch-");
    const cwd = join(root, "cwd");
    writeManifest(cwd, "pnpm@10.34.5");
    const { cacheRoot, env, pnpmHome } = isolatedProbeEnv(root);
    const probeEnv = prependPath(env, pnpm12Dir!);
    const repoBytesBefore = readFileSync(repoPackageJsonPath);
    const parentPmOnFail = process.env.pnpm_config_pm_on_fail;
    const parentPnpmHome = process.env.PNPM_HOME;
    const cacheBefore = listRelativePaths(cacheRoot);
    const homeBefore = listRelativePaths(pnpmHome);
    const engineVersionsBefore = scoopPnpmEngineVersions();
    const storeMarkersBefore = scoopPnpmEngineStoreMarkerNames();
    const { calls, commandRunner } = capturingPnpmProbeRunner();

    const started = Date.now();
    const checks = collectDoctorChecks({
      platform: "linux",
      cwd,
      env: probeEnv,
      commandRunner,
    });
    const elapsedMs = Date.now() - started;

    const pnpmCall = calls.find((call) => call.command === "pnpm");
    expect(pnpmCall?.args).toEqual(["--version"]);
    expect(pnpmCall?.options?.cwd).toBe(cwd);
    expect(pnpmCall?.options?.env?.pnpm_config_pm_on_fail).toBe("ignore");
    expect(pnpmCall?.options?.env).not.toBe(process.env);

    const pnpm = checks.find((check) => check.id === "pnpm");
    expect(pnpm).toMatchObject({
      status: "mismatch",
      actual: "12.3.4",
      expected: "10.34.5",
    });
    expect(elapsedMs).toBeLessThan(3_000);

    const cacheNew = listRelativePaths(cacheRoot).filter((path) => !cacheBefore.includes(path));
    const homeNew = listRelativePaths(pnpmHome).filter((path) => !homeBefore.includes(path));
    expect(packageFetchArtifacts(cacheNew)).toEqual([]);
    expect(packageFetchArtifacts(homeNew)).toEqual([]);
    expect(cacheNew).toEqual([]);
    expect(homeNew).toEqual([]);
    expect(scoopPnpmEngineVersions()).toEqual(engineVersionsBefore);
    expect(scoopPnpmEngineStoreMarkerNames()).toEqual(storeMarkersBefore);
    expect(readFileSync(repoPackageJsonPath)).toEqual(repoBytesBefore);
    expect(process.env.pnpm_config_pm_on_fail).toBe(parentPmOnFail);
    expect(process.env.PNPM_HOME).toBe(parentPnpmHome);
  });

  it("passes a matching pnpm 10.34.5 probe without writing an isolated cache", () => {
    const pinned = findPinnedPnpm10345();
    expect(pinned, "pnpm 10.34.5 must be available for the matching probe").toBeTruthy();

    const root = makeTempDir("doctor-pnpm-match-");
    const cwd = join(root, "cwd");
    writeManifest(cwd, "pnpm@10.34.5");
    const { cacheRoot, env, pnpmHome } = isolatedProbeEnv(root);
    const probeEnv = prependPath(env, dirname(pinned!));
    const repoBytesBefore = readFileSync(repoPackageJsonPath);
    const parentPmOnFail = process.env.pnpm_config_pm_on_fail;
    const cacheBefore = listRelativePaths(cacheRoot);
    const homeBefore = listRelativePaths(pnpmHome);
    const engineVersionsBefore = scoopPnpmEngineVersions();
    const storeMarkersBefore = scoopPnpmEngineStoreMarkerNames();
    const { calls, commandRunner } = capturingPnpmProbeRunner();

    const checks = collectDoctorChecks({
      platform: "linux",
      cwd,
      env: probeEnv,
      commandRunner,
    });

    const pnpmCall = calls.find((call) => call.command === "pnpm");
    expect(pnpmCall?.args).toEqual(["--version"]);
    expect(pnpmCall?.options?.env?.pnpm_config_pm_on_fail).toBe("ignore");
    expect(checks.find((check) => check.id === "pnpm")).toMatchObject({
      status: "ok",
      actual: "10.34.5",
      expected: "10.34.5",
    });
    expect(listRelativePaths(cacheRoot).filter((path) => !cacheBefore.includes(path))).toEqual([]);
    expect(listRelativePaths(pnpmHome).filter((path) => !homeBefore.includes(path))).toEqual([]);
    expect(scoopPnpmEngineVersions()).toEqual(engineVersionsBefore);
    expect(scoopPnpmEngineStoreMarkerNames()).toEqual(storeMarkersBefore);
    expect(readFileSync(repoPackageJsonPath)).toEqual(repoBytesBefore);
    expect(process.env.pnpm_config_pm_on_fail).toBe(parentPmOnFail);
  });
});
