import { spawn, spawnSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { resolveRepoRoot } from "../lib/repo-root.mjs";

const repoRoot = resolveRepoRoot(import.meta.url);
const isWindows = process.platform === "win32";

const quickSteps = [
  { name: "version", command: "pnpm", args: ["version:check"] },
  { name: "docs-generated", command: "pnpm", args: ["docs:gen:check"] },
  { name: "typecheck", command: "pnpm", args: ["typecheck"] },
  { name: "lint", command: "pnpm", args: ["lint"] },
  { name: "capability", command: "pnpm", args: ["capabilitycheck"] },
  { name: "size", command: "pnpm", args: ["sizecheck"] },
  { name: "entrypoints", command: "pnpm", args: ["entrypointcheck"] },
  {
    name: "fmt",
    command: "cargo",
    args: [
      "fmt",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--all",
      "--",
      "--check",
    ],
  },
];

export const CI_LANES = {
  quick: quickSteps,
  common: [
    ...quickSteps,
    { name: "ipc-codegen", command: "pnpm", args: ["ipc:codegen:check"] },
    { name: "test", command: "pnpm", args: ["test"] },
    { name: "build", command: "pnpm", args: ["build"] },
    { name: "docs", command: "pnpm", args: ["docs:site:build"] },
  ],
  "rust-platform": [
    {
      name: "clippy",
      command: "cargo",
      args: [
        "clippy",
        "--manifest-path",
        "src-tauri/Cargo.toml",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "warnings",
      ],
    },
    {
      name: "test",
      command: "cargo",
      args: ["test", "--manifest-path", "src-tauri/Cargo.toml", "--locked"],
    },
  ],
};

function resolveCommand(command) {
  if (isWindows) {
    const result = spawnSync("where.exe", [command], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
      windowsHide: true,
    });

    if (result.status === 0) {
      const candidates = result.stdout
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter(Boolean);
      return (
        candidates.find((candidate) => /\.(cmd|bat)$/i.test(candidate))
        ?? candidates[0]
        ?? command
      );
    }
  }

  return command;
}

function quoteArg(arg) {
  return /\s/.test(arg) ? JSON.stringify(arg) : arg;
}

function quoteCmdArg(arg) {
  if (!/[()\s"&^<>|]/.test(arg)) {
    return arg;
  }

  return `"${arg.replace(/"/g, '""')}"`;
}

function formatCommand(command, args) {
  return [command, ...args].map(quoteArg).join(" ");
}

function createSpawnConfig(command, args) {
  const resolvedCommand = resolveCommand(command);

  if (isWindows && /\.(cmd|bat)$/i.test(resolvedCommand)) {
    return {
      command: [resolvedCommand, ...args].map(quoteCmdArg).join(" "),
      args: [],
      options: { shell: true },
    };
  }

  return { command: resolvedCommand, args, options: {} };
}

function createLineWriter(prefix, write) {
  let buffer = "";

  return {
    push(chunk) {
      buffer += chunk.toString();
      const lines = buffer.split(/\r?\n/);
      buffer = lines.pop() ?? "";

      for (const line of lines) {
        write(`[${prefix}] ${line}`);
      }
    },
    flush() {
      if (buffer.length > 0) {
        write(`[${prefix}] ${buffer}`);
        buffer = "";
      }
    },
  };
}

function terminateChild(child) {
  if (!child.pid || child.exitCode !== null || child.signalCode !== null) {
    return;
  }

  if (isWindows) {
    spawnSync("taskkill", ["/pid", String(child.pid), "/t", "/f"], {
      stdio: "ignore",
      windowsHide: true,
    });
    return;
  }

  const killProcessGroup = (signal) => {
    try {
      process.kill(-child.pid, signal);
    } catch (error) {
      if (error?.code !== "ESRCH") {
        child.kill(signal);
      }
    }
  };

  killProcessGroup("SIGTERM");
  setTimeout(() => {
    if (child.exitCode === null && child.signalCode === null) {
      killProcessGroup("SIGKILL");
    }
  }, 5_000).unref();
}

export async function spawnStep({ lane, step, signal, writeLine, writeError }) {
  const label = `${lane}:${step.name}`;
  const spawnConfig = createSpawnConfig(step.command, step.args);

  await new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(spawnConfig.command, spawnConfig.args, {
      cwd: repoRoot,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
      detached: !isWindows,
      windowsHide: true,
      ...spawnConfig.options,
    });
    const stdout = createLineWriter(label, writeLine);
    const stderr = createLineWriter(label, writeError);
    let settled = false;

    const finish = (error) => {
      if (settled) return;
      settled = true;
      signal.removeEventListener("abort", handleAbort);
      stdout.flush();
      stderr.flush();
      if (error) rejectPromise(error);
      else resolvePromise();
    };
    const handleAbort = () => terminateChild(child);

    signal.addEventListener("abort", handleAbort, { once: true });
    if (signal.aborted) handleAbort();
    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.on("error", (error) => {
      finish(new Error(`[${label}] failed to start: ${error.message}`));
    });
    child.on("close", (code, childSignal) => {
      if (signal.aborted) {
        finish(new Error(`[${label}] stopped after sibling failure.`));
        return;
      }
      if (code === 0) {
        finish();
        return;
      }

      const suffix = childSignal ? `signal ${childSignal}` : `exit code ${code ?? 1}`;
      finish(new Error(`[${label}] failed with ${suffix}.`));
    });
  });
}

function resolveLaneNames(lane) {
  if (lane === "all") return ["common", "rust-platform"];
  if (Object.hasOwn(CI_LANES, lane)) return [lane];
  throw new Error(`Unknown CI lane: ${lane}. Expected quick, common, rust-platform, or all.`);
}

export function parseCiArgs(args) {
  if (args.length === 0) return { lane: "all" };
  if (args.length !== 2 || args[0] !== "--lane") {
    throw new Error("Usage: node scripts/check/run-ci.mjs [--lane quick|common|rust-platform|all]");
  }

  resolveLaneNames(args[1]);
  return { lane: args[1] };
}

function formatSeconds(startedAt, finishedAt) {
  return ((finishedAt - startedAt) / 1_000).toFixed(1);
}

function writeSummary(summaryPath, stepResults, laneResults) {
  if (!summaryPath) return;

  const lines = [
    "## CI lane summary",
    "",
    "| Lane | Step | Result | Seconds |",
    "| --- | --- | --- | ---: |",
    ...stepResults.map(
      (result) => `| ${result.lane} | ${result.step} | ${result.result} | ${result.seconds} |`,
    ),
    "",
    "| Lane | Result |",
    "| --- | --- |",
    ...laneResults.map((result) => `| ${result.lane} | ${result.result} |`),
    "",
  ];
  appendFileSync(summaryPath, `${lines.join("\n")}\n`);
}

export async function runCi({
  lane = "all",
  executeStep = spawnStep,
  now = Date.now,
  summaryPath = process.env.GITHUB_STEP_SUMMARY ?? null,
  writeLine = console.log,
  writeError = console.error,
  signal: externalSignal,
} = {}) {
  const laneNames = resolveLaneNames(lane);
  const controller = new AbortController();
  const stepResults = [];
  const laneResults = [];
  let firstFailure = null;
  const handleExternalAbort = () => {
    controller.abort(externalSignal.reason ?? new Error("CI execution aborted."));
  };

  if (externalSignal) {
    if (externalSignal.aborted) handleExternalAbort();
    else externalSignal.addEventListener("abort", handleExternalAbort, { once: true });
  }

  async function runLane(laneName) {
    for (const step of CI_LANES[laneName]) {
      if (controller.signal.aborted) {
        throw new Error(`[${laneName}:${step.name}] stopped after sibling failure.`);
      }

      const startedAt = now();
      writeLine(`[${laneName}:${step.name}] $ ${formatCommand(step.command, step.args)}`);
      try {
        await executeStep({
          lane: laneName,
          step,
          signal: controller.signal,
          writeLine,
          writeError,
        });
        const seconds = formatSeconds(startedAt, now());
        stepResults.push({ lane: laneName, step: step.name, result: "success", seconds });
        writeLine(`[${laneName}:${step.name}] completed in ${seconds}s`);
      } catch (error) {
        const seconds = formatSeconds(startedAt, now());
        stepResults.push({
          lane: laneName,
          step: step.name,
          result: controller.signal.aborted ? "cancelled" : "failure",
          seconds,
        });
        throw error;
      }
    }
  }

  writeLine(`[ci] Running ${laneNames.join(" and ")} lane${laneNames.length === 1 ? "" : "s"}.`);

  const results = await Promise.allSettled(laneNames.map(async (laneName) => {
    try {
      await runLane(laneName);
      laneResults.push({ lane: laneName, result: "success" });
    } catch (error) {
      const normalizedError = error instanceof Error ? error : new Error(String(error));
      const aborted = controller.signal.aborted;
      laneResults.push({ lane: laneName, result: aborted ? "cancelled" : "failure" });
      if (!firstFailure) {
        firstFailure = normalizedError;
        writeError(`[ci] ${normalizedError.message}`);
        writeError("[ci] Stopping sibling checks...");
        controller.abort(normalizedError);
      }
      throw normalizedError;
    }
  }));

  if (externalSignal) {
    externalSignal.removeEventListener("abort", handleExternalAbort);
  }
  writeSummary(summaryPath, stepResults, laneResults);

  if (results.some((result) => result.status === "rejected")) {
    const failure = firstFailure ?? new Error("CI execution failed.");
    writeError(`[ci] Failed. First failure: ${failure.message}`);
    throw failure;
  }

  writeLine("[ci] All checks passed.");
}

function isDirectExecution() {
  return process.argv[1] !== undefined
    && pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
}

if (isDirectExecution()) {
  let cliOptions = null;
  try {
    cliOptions = parseCiArgs(process.argv.slice(2));
  } catch (error) {
    console.error(`[ci] ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  }

  if (cliOptions) {
    const controller = new AbortController();
    const signalHandlers = new Map();

    for (const signalName of ["SIGINT", "SIGTERM"]) {
      const handler = () => controller.abort(new Error(`Received ${signalName}.`));
      signalHandlers.set(signalName, handler);
      process.on(signalName, handler);
    }

    try {
      await runCi({ ...cliOptions, signal: controller.signal });
    } catch {
      process.exitCode = 1;
    }

    for (const [signalName, handler] of signalHandlers) {
      process.off(signalName, handler);
    }
  }
}
