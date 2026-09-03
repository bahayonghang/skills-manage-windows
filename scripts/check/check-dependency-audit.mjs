import { spawnSync } from "node:child_process";
import { appendFileSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { resolveRepoRoot } from "../lib/repo-root.mjs";

const repoRoot = resolveRepoRoot(import.meta.url);
const exceptionPath = resolve(repoRoot, "security/dependency-audit-exceptions.json");
const EXCEPTION_FIELDS = ["advisory", "ecosystem", "expires", "owner", "reason"];
const ECOSYSTEMS = new Set(["npm", "cargo"]);
const NPM_ADVISORY_PATTERN = /^GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}$/;
const CARGO_ADVISORY_PATTERN = /^RUSTSEC-\d{4}-\d{4}$/;
const BLOCKING_NPM_SEVERITIES = new Set(["high", "critical"]);
const NPM_EVIDENCE_SEVERITIES = new Set(["moderate", "low"]);
const NPM_AUDIT_COMMAND = "pnpm audit --prod --json";
const CARGO_AUDIT_COMMAND = "cargo audit --file src-tauri/Cargo.lock --json";
export const MAX_EVIDENCE_ROWS = 50;

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

function asNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0 ? value : null;
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

function npmEvidenceVersions(advisory) {
  const findings = Array.isArray(advisory?.findings) ? advisory.findings : [];
  const versions = findings
    .map((finding) => asNonEmptyString(finding?.version))
    .filter((version) => version !== null);
  return versions.length > 0 ? versions : ["unknown"];
}

function normalizeNpmEvidence(report) {
  if (!report || typeof report !== "object" || !report.advisories || typeof report.advisories !== "object") {
    return [];
  }

  const findings = [];
  for (const advisory of Object.values(report.advisories)) {
    if (!NPM_EVIDENCE_SEVERITIES.has(advisory?.severity)) continue;
    const packageName = asNonEmptyString(advisory?.module_name) ?? "unknown";
    const advisoryId = asNonEmptyString(advisory?.github_advisory_id) ?? "unknown";
    for (const version of npmEvidenceVersions(advisory)) {
      findings.push({
        ecosystem: "npm",
        category: advisory.severity,
        severity: advisory.severity,
        advisory: advisoryId,
        package: packageName,
        version,
      });
    }
  }
  return findings;
}

function normalizeCargoEvidence(report) {
  const warnings = report?.warnings;
  if (!warnings || typeof warnings !== "object" || Array.isArray(warnings)) {
    return [];
  }

  const findings = [];
  for (const [warningKind, list] of Object.entries(warnings)) {
    if (!Array.isArray(list)) continue;
    for (const warning of list) {
      const category = asNonEmptyString(warning?.kind) ?? warningKind;
      findings.push({
        ecosystem: "cargo",
        category,
        advisory: asNonEmptyString(warning?.advisory?.id) ?? "unknown",
        package: asNonEmptyString(warning?.package?.name) ?? "unknown",
        version: asNonEmptyString(warning?.package?.version) ?? "unknown",
      });
    }
  }
  return findings;
}

function evidenceKey(finding) {
  return `${finding.ecosystem}:${finding.category}:${finding.advisory}:${finding.package}:${finding.version}`;
}

function compareEvidence(left, right) {
  return (
    left.ecosystem.localeCompare(right.ecosystem) ||
    left.category.localeCompare(right.category) ||
    left.advisory.localeCompare(right.advisory) ||
    left.package.localeCompare(right.package) ||
    left.version.localeCompare(right.version)
  );
}

function summarizeEvidence(findings) {
  const counts = new Map();
  for (const finding of findings) {
    const key = evidenceKey(finding);
    const existing = counts.get(key);
    if (existing) {
      existing.count += 1;
      continue;
    }
    counts.set(key, { ...finding, count: 1 });
  }

  const rows = [...counts.values()].sort(compareEvidence);
  const shownRows = rows.slice(0, MAX_EVIDENCE_ROWS);
  return {
    total: rows.length,
    shown: shownRows.length,
    truncated: Math.max(0, rows.length - shownRows.length),
    rows: shownRows,
  };
}

function buildProvenance() {
  return {
    currentness: "unverified",
    sources: [NPM_AUDIT_COMMAND, CARGO_AUDIT_COMMAND],
  };
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
    evidence: summarizeEvidence([...normalizeNpmEvidence(npmAudit), ...normalizeCargoEvidence(cargoAudit)]),
    provenance: buildProvenance(),
  };
}

export function formatAuditEvidence(result) {
  const evidence = result.evidence ?? { total: 0, shown: 0, truncated: 0, rows: [] };
  const provenance = result.provenance ?? buildProvenance();
  const consoleLines = [
    `[audit] currentness: ${provenance.currentness}`,
    `[audit] sources: ${provenance.sources.join("; ")}`,
    `[audit] evidence total=${evidence.total} shown=${evidence.shown} truncated=${evidence.truncated}`,
    ...evidence.rows.map(
      (row) =>
        `[audit] ${row.ecosystem} ${row.category} ${row.advisory} ${row.package}@${row.version} count=${row.count}`,
    ),
  ];
  const markdownLines = [
    "## Dependency audit evidence",
    "",
    `- currentness: ${provenance.currentness}`,
    `- sources: ${provenance.sources.map((source) => `\`${source}\``).join("; ")}`,
    `- total: ${evidence.total} shown: ${evidence.shown} truncated: ${evidence.truncated}`,
    "",
    "| Ecosystem | Category | Advisory | Package | Version | Count |",
    "| --- | --- | --- | --- | --- | ---: |",
    ...evidence.rows.map(
      (row) =>
        `| ${row.ecosystem} | ${row.category} | ${row.advisory} | ${row.package} | ${row.version} | ${row.count} |`,
    ),
    "",
  ];
  return {
    consoleText: consoleLines.join("\n"),
    markdown: markdownLines.join("\n"),
  };
}

function resolveSummaryPath(summaryPath) {
  if (typeof summaryPath !== "string") return null;
  const trimmed = summaryPath.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function runJsonCommand(command, args, label) {
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

export function executeDependencyAuditCli({
  evaluate = runDependencyAudit,
  summaryPath = process.env.GITHUB_STEP_SUMMARY ?? null,
  writeLine = console.log,
  writeError = console.error,
  exitProcess = (code) => process.exit(code),
} = {}) {
  try {
    const result = evaluate();
    const output = formatAuditEvidence(result);
    writeLine(output.consoleText);
    const resolvedSummaryPath = resolveSummaryPath(summaryPath);
    if (resolvedSummaryPath) {
      try {
        appendFileSync(resolvedSummaryPath, output.markdown);
      } catch (error) {
        throw new Error(`Failed to write GitHub Step Summary: ${error.message}`);
      }
    }
    if (result.errors.length > 0) {
      throw new Error(result.errors.join("\n"));
    }
    writeLine(
      `[audit] Passed. ${result.blockers.length} blocking advisories, ${result.approved.length} approved exceptions.`,
    );
  } catch (error) {
    writeError(`[audit] ${error.message}`);
    exitProcess(1);
  }
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  executeDependencyAuditCli();
}
