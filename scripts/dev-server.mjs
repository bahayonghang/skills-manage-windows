import { spawn, spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import net from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const DEV_PORT = 24200;

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const viteEntry = join(repoRoot, "node_modules", "vite", "bin", "vite.js");
const PORT_RELEASE_TIMEOUT_MS = 5_000;
const DEV_SERVER_PROBE_TIMEOUT_MS = 1_000;

export function normalizeForProcessMatch(value) {
  return String(value ?? "")
    .replace(/\\/g, "/")
    .toLowerCase();
}

export function parseProcessListJson(rawJson) {
  const text = String(rawJson ?? "").trim();

  if (!text) {
    return [];
  }

  const parsed = JSON.parse(text);
  const entries = Array.isArray(parsed) ? parsed : [parsed];

  return entries
    .filter(Boolean)
    .map((entry) => ({
      pid: Number(entry.pid),
      localAddress: entry.localAddress ? String(entry.localAddress) : "",
      processName: entry.processName ? String(entry.processName) : "",
      executablePath: entry.executablePath ? String(entry.executablePath) : "",
      commandLine: entry.commandLine ? String(entry.commandLine) : "",
    }))
    .filter((entry) => Number.isInteger(entry.pid) && entry.pid > 0);
}

export function isRepoOwnedDevProcess(processInfo, projectRoot = repoRoot) {
  if (!processInfo || processInfo.pid === process.pid) {
    return false;
  }

  const processText = normalizeForProcessMatch(
    [
      processInfo.processName,
      processInfo.executablePath,
      processInfo.commandLine,
    ].join(" ")
  );
  const normalizedRoot = normalizeForProcessMatch(projectRoot);

  return (
    processText.includes(normalizedRoot)
    && ["vite", "pnpm", "node", "tauri"].some((token) => processText.includes(token))
  );
}

export function formatProcessInfo(processInfo) {
  const pieces = [
    `pid=${processInfo.pid}`,
    processInfo.processName ? `name=${processInfo.processName}` : "",
    processInfo.executablePath ? `path=${processInfo.executablePath}` : "",
    processInfo.commandLine ? `cmd=${processInfo.commandLine}` : "",
  ].filter(Boolean);

  return pieces.join(" ");
}

export function isSkillPortIndexHtml(html) {
  const content = String(html);

  return (
    content.includes("<title>SkillPort</title>")
    && /src="\/src\/main\.tsx(?:\?[^"]*)?"/.test(content)
  );
}

async function probeExistingDevServer(port) {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/`, {
      signal: AbortSignal.timeout(DEV_SERVER_PROBE_TIMEOUT_MS),
    });
    const html = await response.text();
    return response.ok && isSkillPortIndexHtml(html);
  } catch {
    return false;
  }
}

function queryWindowsPortOwners(port) {
  const script = `
$connections = @(Get-NetTCPConnection -LocalPort ${port} -State Listen -ErrorAction SilentlyContinue)
$result = foreach ($connection in $connections) {
  $process = Get-CimInstance Win32_Process -Filter "ProcessId = $($connection.OwningProcess)" -ErrorAction SilentlyContinue
  [pscustomobject]@{
    pid = [int]$connection.OwningProcess
    localAddress = [string]$connection.LocalAddress
    processName = if ($process) { [string]$process.Name } else { "" }
    executablePath = if ($process) { [string]$process.ExecutablePath } else { "" }
    commandLine = if ($process) { [string]$process.CommandLine } else { "" }
  }
}
$result | ConvertTo-Json -Compress
`;

  const result = spawnSync("powershell.exe", ["-NoProfile", "-Command", script], {
    encoding: "utf8",
  });

  if (result.error) {
    return { owners: [], error: result.error.message };
  }

  if (result.status !== 0) {
    return {
      owners: [],
      error: result.stderr.trim() || `powershell exited with ${result.status}`,
    };
  }

  return { owners: parseProcessListJson(result.stdout), error: "" };
}

function queryUnixPortOwners(port) {
  const lsof = spawnSync("lsof", ["-nP", `-iTCP:${port}`, "-sTCP:LISTEN"], {
    encoding: "utf8",
  });

  if (lsof.error || lsof.status !== 0) {
    return { owners: [], error: lsof.error?.message ?? "" };
  }

  const owners = lsof.stdout
    .trim()
    .split(/\r?\n/)
    .slice(1)
    .map((line) => {
      const columns = line.trim().split(/\s+/);
      const pid = Number(columns[1]);
      return {
        pid,
        localAddress: "",
        processName: columns[0] ?? "",
        executablePath: "",
        commandLine: line,
      };
    })
    .filter((entry) => Number.isInteger(entry.pid) && entry.pid > 0);

  return { owners, error: "" };
}

function queryPortOwners(port) {
  return process.platform === "win32"
    ? queryWindowsPortOwners(port)
    : queryUnixPortOwners(port);
}

function canBindPort(port) {
  return new Promise((resolve) => {
    const server = net.createServer();

    server.once("error", () => resolve(false));
    server.once("listening", () => {
      server.close(() => resolve(true));
    });
    server.listen(port, "127.0.0.1");
  });
}

async function waitForPortRelease(port, timeoutMs) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    if (await canBindPort(port)) {
      return true;
    }

    await new Promise((resolve) => setTimeout(resolve, 200));
  }

  return false;
}

export async function ensureDevPortAvailable({
  port = DEV_PORT,
  projectRoot = repoRoot,
  getOwners = queryPortOwners,
} = {}) {
  if (await canBindPort(port)) {
    return;
  }

  const { owners, error } = getOwners(port);
  const safeOwners = owners.filter((owner) => isRepoOwnedDevProcess(owner, projectRoot));

  if (owners.length === 0 || safeOwners.length !== owners.length) {
    const ownerDetails = owners.length > 0
      ? owners.map(formatProcessInfo).join("\n  ")
      : error || "owner lookup returned no listener details";

    throw new Error(
      `Port ${port} is already in use by a process that this script will not stop automatically.\n`
      + `  ${ownerDetails}`
    );
  }

  console.log(
    `[dev] port ${port} is held by stale SkillPort dev process(es); stopping ${safeOwners
      .map((owner) => owner.pid)
      .join(", ")}`
  );

  for (const owner of safeOwners) {
    try {
      process.kill(owner.pid, "SIGTERM");
    } catch (error) {
      if (error?.code !== "ESRCH") {
        throw error;
      }
    }
  }

  if (!(await waitForPortRelease(port, PORT_RELEASE_TIMEOUT_MS))) {
    throw new Error(`Port ${port} did not become available after stopping stale dev processes.`);
  }
}

function holdForExistingDevServer() {
  console.log(`[dev] reusing existing SkillPort dev server on port ${DEV_PORT}`);
  process.stdin.resume();
}

function startVite(args) {
  if (!existsSync(viteEntry)) {
    throw new Error(`Vite entrypoint not found: ${viteEntry}`);
  }

  const child = spawn(process.execPath, [viteEntry, ...args], {
    cwd: repoRoot,
    env: process.env,
    stdio: "inherit",
  });

  for (const signal of ["SIGINT", "SIGTERM"]) {
    process.once(signal, () => {
      if (!child.killed) {
        child.kill(signal);
      }
    });
  }

  child.on("error", (error) => {
    throw error;
  });

  child.on("exit", (code) => {
    process.exit(code ?? 0);
  });
}

async function main() {
  const shouldReuseExistingServer = !(await canBindPort(DEV_PORT))
    && (await probeExistingDevServer(DEV_PORT));

  if (shouldReuseExistingServer) {
    holdForExistingDevServer();
    return;
  }

  await ensureDevPortAvailable();
  startVite(process.argv.slice(2));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(`[dev] ${error.message}`);
    process.exit(1);
  });
}
