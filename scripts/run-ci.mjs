import { spawn, spawnSync } from "node:child_process";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const isWindows = process.platform === "win32";
const activeChildren = new Set();

let abortReason = null;
let firstFailure = null;

const chains = [
  {
    name: "web",
    steps: [
      { name: "typecheck", command: "pnpm", args: ["typecheck"] },
      { name: "lint", command: "pnpm", args: ["lint"] },
      { name: "sizecheck", command: "pnpm", args: ["sizecheck"] },
      { name: "test", command: "pnpm", args: ["test"] },
    ],
  },
  {
    name: "rust",
    steps: [
      { name: "entrypoints", command: "pnpm", args: ["entrypointcheck"] },
      {
        name: "clippy",
        command: "cargo",
        args: [
          "clippy",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "--",
          "-D",
          "warnings",
        ],
      },
      {
        name: "test",
        command: "cargo",
        args: ["test", "--manifest-path", "src-tauri/Cargo.toml"],
      },
    ],
  },
];

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
        candidates.find((candidate) => /\.(cmd|bat)$/i.test(candidate)) ??
        candidates[0] ??
        command
      );
    }
  }

  return command;
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

  child.kill("SIGTERM");
  setTimeout(() => {
    if (child.exitCode === null && child.signalCode === null) {
      child.kill("SIGKILL");
    }
  }, 5_000).unref();
}

function abortAll(reason) {
  if (abortReason) {
    return;
  }

  abortReason = reason;
  console.error(`[ci] ${reason.message}`);
  console.error("[ci] Stopping sibling checks...");

  for (const child of activeChildren) {
    terminateChild(child);
  }
}

async function runProcess(chainName, step) {
  if (abortReason) {
    throw new Error(`[${chainName}:${step.name}] skipped after sibling failure.`);
  }

  const label = `${chainName}:${step.name}`;
  const spawnConfig = createSpawnConfig(step.command, step.args);
  const startedAt = Date.now();

  console.log(`[${label}] $ ${formatCommand(step.command, step.args)}`);

  return new Promise((resolve, reject) => {
    const child = spawn(spawnConfig.command, spawnConfig.args, {
      cwd: repoRoot,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
      ...spawnConfig.options,
    });
    const stdout = createLineWriter(label, console.log);
    const stderr = createLineWriter(label, console.error);
    let settled = false;

    activeChildren.add(child);

    const finish = (error) => {
      if (settled) {
        return;
      }

      settled = true;
      stdout.flush();
      stderr.flush();
      activeChildren.delete(child);

      if (error) {
        reject(error);
        return;
      }

      const elapsedSeconds = ((Date.now() - startedAt) / 1_000).toFixed(1);
      console.log(`[${label}] completed in ${elapsedSeconds}s`);
      resolve();
    };

    child.stdout.on("data", (chunk) => stdout.push(chunk));
    child.stderr.on("data", (chunk) => stderr.push(chunk));
    child.on("error", (error) => {
      finish(new Error(`[${label}] failed to start: ${error.message}`));
    });
    child.on("close", (code, signal) => {
      if (code === 0) {
        finish();
        return;
      }

      if (abortReason) {
        finish(new Error(`[${label}] stopped after sibling failure.`));
        return;
      }

      const suffix = signal ? `signal ${signal}` : `exit code ${code ?? 1}`;
      finish(new Error(`[${label}] failed with ${suffix}.`));
    });
  });
}

async function runChain(chain) {
  const startedAt = Date.now();

  for (const step of chain.steps) {
    await runProcess(chain.name, step);
  }

  const elapsedSeconds = ((Date.now() - startedAt) / 1_000).toFixed(1);
  console.log(`[${chain.name}] chain completed in ${elapsedSeconds}s`);
}

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    abortAll(new Error(`Received ${signal}.`));
  });
}

console.log("[ci] Running web and Rust chains in parallel.");

const results = await Promise.allSettled(
  chains.map((chain) =>
    runChain(chain).catch((error) => {
      if (!firstFailure) {
        firstFailure = error;
        abortAll(error);
      }

      throw error;
    }),
  ),
);

const rejected = results.filter((result) => result.status === "rejected");

if (rejected.length > 0) {
  console.error(`[ci] Failed. First failure: ${firstFailure.message}`);
  process.exit(1);
}

console.log("[ci] All checks passed.");
