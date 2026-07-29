import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const exceptionPath = resolve(repoRoot, "security/dependency-audit-exceptions.json");
const EXCEPTION_FIELDS = ["advisory", "ecosystem", "expires", "owner", "reason"];
const ECOSYSTEMS = new Set(["npm", "cargo"]);
const NPM_ADVISORY_PATTERN = /^GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}$/;
const CARGO_ADVISORY_PATTERN = /^RUSTSEC-\d{4}-\d{4}$/;
const BLOCKING_NPM_SEVERITIES = new Set(["high", "critical"]);

function sorted(values) {
  return [...values].sort((left, right) => left.localeCompare(right));
}

function todayUtc(now) {
  return now.toISOString().slice(0, 10);
}

function isIsoDate(value) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const parsed = new Date(`${value}T00:00:00.000Z`);
  return !Number.isNaN(parsed.valueOf()) && parsed.toISOString().slice(0, 10) === value;
}

function validateExceptions(exceptions, now) {
  const errors = [];
  const valid = [];
  const keys = new Set();

  if (!Array.isArray(exceptions)) {
    return { errors: ["Exception manifest must be a JSON array."], valid };
  }

  exceptions.forEach((entry, index) => {
    const label = `Exception ${index + 1}`;
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
      errors.push(`${label} must be an object.`);
      return;
    }

    const fields = sorted(Object.keys(entry));
    if (JSON.stringify(fields) !== JSON.stringify(EXCEPTION_FIELDS)) {
      errors.push(`${label} fields must be exactly: ${EXCEPTION_FIELDS.join(", ")}.`);
      return;
    }

    if (!ECOSYSTEMS.has(entry.ecosystem)) {
      errors.push(`${label} has invalid ecosystem '${entry.ecosystem}'.`);
      return;
    }

    const pattern = entry.ecosystem === "npm" ? NPM_ADVISORY_PATTERN : CARGO_ADVISORY_PATTERN;
    if (typeof entry.advisory !== "string" || !pattern.test(entry.advisory)) {
      errors.push(`${label} advisory does not match ecosystem '${entry.ecosystem}'.`);
      return;
    }

    if (typeof entry.owner !== "string" || entry.owner.trim().length === 0) {
      errors.push(`${label} must have a non-empty owner.`);
      return;
    }
    if (typeof entry.reason !== "string" || entry.reason.trim().length === 0) {
      errors.push(`${label} must have a non-empty reason.`);
      return;
    }
    if (typeof entry.expires !== "string" || !isIsoDate(entry.expires)) {
      errors.push(`${label} must have a valid ISO date expiry.`);
      return;
    }
    if (entry.expires < todayUtc(now)) {
      errors.push(`${label} expired on ${entry.expires}.`);
      return;
    }

    const key = `${entry.ecosystem}:${entry.advisory}`;
    if (keys.has(key)) {
      errors.push(`${label} duplicates ${key}.`);
      return;
    }
    keys.add(key);
    valid.push({ ...entry, key });
  });

  return { errors, valid };
}

function normalizeNpmBlockers(report, errors) {
  if (!report || typeof report !== "object" || !report.advisories || typeof report.advisories !== "object") {
    errors.push("pnpm audit JSON is missing the advisories object.");
    return [];
  }

  const blockers = new Map();
  for (const advisory of Object.values(report.advisories)) {
    if (!BLOCKING_NPM_SEVERITIES.has(advisory?.severity)) continue;
    if (typeof advisory.github_advisory_id !== "string" || !NPM_ADVISORY_PATTERN.test(advisory.github_advisory_id)) {
      errors.push("A blocking pnpm advisory is missing a valid GitHub advisory id.");
      continue;
    }

    const key = `npm:${advisory.github_advisory_id}`;
    blockers.set(key, {
      key,
      ecosystem: "npm",
      advisory: advisory.github_advisory_id,
      package: advisory.module_name ?? "unknown",
      severity: advisory.severity,
    });
  }
  return [...blockers.values()];
}

function normalizeCargoBlockers(report, errors) {
  const findings = report?.vulnerabilities?.list;
  if (!Array.isArray(findings)) {
    errors.push("cargo audit JSON is missing vulnerabilities.list.");
    return [];
  }

  const blockers = [];
  for (const finding of findings) {
    const advisory = finding?.advisory?.id;
    if (typeof advisory !== "string" || !CARGO_ADVISORY_PATTERN.test(advisory)) {
      errors.push("A cargo vulnerability is missing a valid RUSTSEC advisory id.");
      continue;
    }

    blockers.push({
      key: `cargo:${advisory}`,
      ecosystem: "cargo",
      advisory,
      package: finding.package.name,
      severity: "vulnerability",
    });
  }
  return blockers;
}

export function evaluateDependencyAudit({ npmAudit, cargoAudit, exceptions, now = new Date() }) {
  const errors = [];
  const npmBlockers = normalizeNpmBlockers(npmAudit, errors);
  const cargoBlockers = normalizeCargoBlockers(cargoAudit, errors);
  const exceptionResult = validateExceptions(exceptions, now);
  errors.push(...exceptionResult.errors);

  const blockers = [...npmBlockers, ...cargoBlockers];
  const blockersByKey = new Map(blockers.map((blocker) => [blocker.key, blocker]));
  const exceptionKeys = new Set(exceptionResult.valid.map((entry) => entry.key));

  for (const entry of exceptionResult.valid) {
    if (!blockersByKey.has(entry.key)) errors.push(`Exception ${entry.key} is unused.`);
  }
  for (const blocker of blockers) {
    if (!exceptionKeys.has(blocker.key)) {
      errors.push(`Unapproved ${blocker.severity} advisory ${blocker.key} affects ${blocker.package}.`);
    }
  }

  return {
    errors,
    blockers,
    approved: blockers.filter((blocker) => exceptionKeys.has(blocker.key)),
  };
}

function runJsonCommand(command, args, label) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    windowsHide: true,
    maxBuffer: 32 * 1024 * 1024,
  });
  if (result.error) throw new Error(`Failed to start ${label}: ${result.error.message}`);
  if (!result.stdout.trim()) {
    throw new Error(`${label} produced no JSON (exit ${result.status ?? 1}): ${result.stderr.trim()}`);
  }
  try {
    return JSON.parse(result.stdout);
  } catch (error) {
    throw new Error(`${label} produced invalid JSON: ${error.message}`);
  }
}

export function runDependencyAudit() {
  const npmAudit = runJsonCommand("pnpm", ["audit", "--prod", "--json"], "pnpm audit");
  const cargoAudit = runJsonCommand(
    "cargo",
    ["audit", "--file", "src-tauri/Cargo.lock", "--json"],
    "cargo audit",
  );
  const exceptions = JSON.parse(readFileSync(exceptionPath, "utf8"));
  return evaluateDependencyAudit({ npmAudit, cargoAudit, exceptions });
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  try {
    const result = runDependencyAudit();
    if (result.errors.length > 0) throw new Error(result.errors.join("\n"));
    console.log(
      `[audit] Passed. ${result.blockers.length} blocking advisories, ${result.approved.length} approved exceptions.`,
    );
  } catch (error) {
    console.error(`[audit] ${error.message}`);
    process.exit(1);
  }
}
