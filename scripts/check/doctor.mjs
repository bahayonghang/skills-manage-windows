import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { resolveRepoRoot } from "../lib/repo-root.mjs";

const repoRoot = resolveRepoRoot(import.meta.url);

export const TOOLCHAIN = Object.freeze({
  nodeMajor: 26,
  pnpm: "10.34.5",
  rust: "1.98.0",
});

const secretNamePattern = /(token|secret|password|passwd|api[_-]?key|private[_-]?key|credential)/i;
const secretValuePattern = /\b(?:ghp_|github_pat_|gho_|ghu_|ghs_|ghr_|sk-|xox[baprs]-)[A-Za-z0-9_./+=:-]+/g;
const assignmentSecretPattern = /((?:token|secret|password|passwd|api[_-]?key|private[_-]?key)\s*[=:]\s*)("[^"]*"|'[^']*'|[^\s,;]+)/gi;

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Keep diagnostics useful without ever echoing values from credential-shaped
 * environment variables or common token formats.
 */
export function redactSecrets(value, env = process.env) {
  let redacted = String(value ?? "");
  const secretValues = Object.entries(env ?? {})
    .filter(([name, secret]) => secretNamePattern.test(name) && typeof secret === "string")
    .map(([, secret]) => secret)
    .filter((secret) => secret.length >= 3)
    .sort((left, right) => right.length - left.length);

  for (const secret of secretValues) {
    redacted = redacted.replace(new RegExp(escapeRegExp(secret), "g"), "[REDACTED]");
  }

  return redacted
    .replace(secretValuePattern, "[REDACTED]")
    .replace(assignmentSecretPattern, "$1[REDACTED]");
}

/**
 * pnpm 12+ defaults `pmOnFail` to download. Inject ignore only into the
 * version probe so diagnostics cannot fetch, self-update, or rewrite global
 * config. This must not be used to bypass the repository pin in CI.
 */
const PNPM_PM_ON_FAIL_KEY = "pnpm_config_pm_on_fail";

export function withPnpmReadonlyProbeEnv(env = process.env) {
  const next = { ...env };
  for (const key of Object.keys(next)) {
    if (key.toLowerCase() === PNPM_PM_ON_FAIL_KEY) {
      delete next[key];
    }
  }
  next[PNPM_PM_ON_FAIL_KEY] = "ignore";
  return next;
}

export function runCommand(command, args = [], options = {}) {
  try {
    const result = spawnSync(command, args, {
      cwd: options.cwd ?? repoRoot,
      env: options.env ?? process.env,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
      timeout: 5_000,
      killSignal: "SIGTERM",
    });

    return {
      status: result.status,
      stdout: result.stdout ?? "",
      stderr: result.stderr ?? "",
      error: result.error ?? null,
    };
  } catch (error) {
    return { status: null, stdout: "", stderr: "", error };
  }
}

function extractVersion(output) {
  const match = String(output).match(/(?:^|[^\d])v?(\d+\.\d+\.\d+)(?:[^\d]|$)/);
  return match?.[1] ?? null;
}

function versionStatus(actual, expected, mode) {
  if (!actual) return "mismatch";
  if (mode === "major") return Number.parseInt(actual.split(".")[0], 10) === expected ? "ok" : "mismatch";
  return actual === expected ? "ok" : "mismatch";
}

function commandDetail(result, env) {
  const combined = redactSecrets(`${result.stdout}\n${result.stderr}`, env).trim();
  if (result.error?.code === "ENOENT") return { status: "missing", detail: "command not found" };
  if (result.error) {
    const errorMessage = redactSecrets(result.error.message ?? String(result.error), env);
    return { status: "mismatch", detail: `could not run command (${errorMessage})` };
  }
  if (result.status !== 0) {
    return { status: "mismatch", detail: `command exited with ${result.status ?? 1}` };
  }
  return { status: "ok", detail: combined };
}

function checkVersion({ id, label, command, args, expected, mode, hint }, options) {
  const result = options.commandRunner(command, args, { cwd: options.cwd, env: options.env });
  const commandResult = commandDetail(result, options.env);
  if (commandResult.status === "missing" || commandResult.status === "mismatch") {
    return {
      id,
      label,
      status: commandResult.status,
      expected: mode === "major" ? `major ${expected}` : expected,
      actual: commandResult.detail,
      hint,
    };
  }

  const actual = extractVersion(commandResult.detail);
  const status = versionStatus(actual, expected, mode);
  return {
    id,
    label,
    status,
    expected: mode === "major" ? `major ${expected}` : expected,
    actual: actual ?? commandResult.detail,
    hint: status === "ok" ? undefined : hint,
  };
}

function checkPresence({ id, label, command, args, hint }, options) {
  const result = options.commandRunner(command, args, { cwd: options.cwd, env: options.env });
  const commandResult = commandDetail(result, options.env);
  return {
    id,
    label,
    status: commandResult.status === "ok" ? "ok" : commandResult.status,
    expected: "available",
    actual: commandResult.status === "ok"
      ? extractVersion(commandResult.detail) ?? commandResult.detail.split(/\r?\n/, 1)[0]
      : commandResult.detail,
    hint: commandResult.status === "ok" ? undefined : hint,
  };
}

function checkTarget(options) {
  const result = options.commandRunner("rustup", ["target", "list", "--installed"], {
    cwd: options.cwd,
    env: options.env,
  });
  const commandResult = commandDetail(result, options.env);
  const target = "x86_64-pc-windows-msvc";
  if (commandResult.status !== "ok") {
    return {
      id: "windows-msvc-target",
      label: "Windows MSVC Rust target",
      status: commandResult.status,
      expected: target,
      actual: commandResult.detail,
      hint: "install the target with rustup target add x86_64-pc-windows-msvc",
    };
  }

  const installedTargets = commandResult.detail.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const present = installedTargets.includes(target);
  return {
    id: "windows-msvc-target",
    label: "Windows MSVC Rust target",
    status: present ? "ok" : "mismatch",
    expected: target,
    actual: present ? target : "not installed",
    hint: present ? undefined : "install the target with rustup target add x86_64-pc-windows-msvc",
  };
}

function checkTauri(options) {
  const binaryName = options.platform === "win32" ? "tauri.cmd" : "tauri";
  const path = join(options.cwd, "node_modules", ".bin", binaryName);
  const present = options.fileExists(path);
  return {
    id: "tauri-cli",
    label: "Tauri CLI",
    status: present ? "ok" : "missing",
    expected: "installed",
    actual: present ? "installed" : "not installed",
    hint: present ? undefined : "run pnpm install",
  };
}

export function collectDoctorChecks({
  platform = process.platform,
  cwd = repoRoot,
  env = process.env,
  commandRunner = runCommand,
  fileExists = existsSync,
} = {}) {
  const options = { platform, cwd, env, commandRunner, fileExists };
  const checks = [
    checkVersion({
      id: "node",
      label: "Node.js",
      command: process.execPath,
      args: ["--version"],
      expected: TOOLCHAIN.nodeMajor,
      mode: "major",
      hint: "use Node.js 26 for this repository",
    }, options),
    checkVersion({
      id: "pnpm",
      label: "pnpm",
      command: "pnpm",
      args: ["--version"],
      expected: TOOLCHAIN.pnpm,
      mode: "exact",
      hint: "activate pnpm 10.34.5 without changing the repository",
    }, { ...options, env: withPnpmReadonlyProbeEnv(options.env) }),
    checkVersion({
      id: "rustc",
      label: "rustc",
      command: "rustc",
      args: ["--version"],
      expected: TOOLCHAIN.rust,
      mode: "exact",
      hint: "install or select Rust 1.98.0 with rustup",
    }, options),
    checkVersion({
      id: "cargo",
      label: "Cargo",
      command: "cargo",
      args: ["--version"],
      expected: TOOLCHAIN.rust,
      mode: "exact",
      hint: "install or select Rust 1.98.0 with rustup",
    }, options),
    checkPresence({
      id: "just",
      label: "just",
      command: "just",
      args: ["--version"],
      hint: "install just from https://just.systems",
    }, options),
    checkPresence({
      id: "git",
      label: "Git",
      command: "git",
      args: ["--version"],
      hint: "install Git and reopen the terminal",
    }, options),
  ];

  if (platform === "win32") {
    checks.push(checkTarget(options), checkTauri(options));
  }

  return checks;
}

function formatCheck(check) {
  const expected = `expected ${check.expected}`;
  const hint = check.hint ? `; ${check.hint}` : "";
  return `[${check.status}] ${check.label}: ${check.actual} (${expected}${hint})`;
}

export function runDoctor({
  ...options
} = {}) {
  const checks = collectDoctorChecks(options);
  const writeLine = options.writeLine ?? console.log;
  writeLine("[doctor] Read-only development environment check");
  for (const check of checks) writeLine(formatCheck(check));

  const failures = checks.filter((check) => check.status !== "ok");
  if (failures.length === 0) {
    writeLine("[doctor] All required tools are ready.");
  } else {
    writeLine(`[doctor] ${failures.length} check(s) need attention; no changes were made.`);
  }

  return { ok: failures.length === 0, checks, failures };
}

function isDirectExecution() {
  return process.argv[1] !== undefined
    && pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
}

if (isDirectExecution()) {
  const result = runDoctor();
  if (!result.ok) process.exitCode = 1;
}
