/// <reference types="node" />

import { afterEach, describe, expect, it } from "vitest";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

type Exception = {
  ecosystem: "npm" | "cargo";
  advisory: string;
  owner: string;
  reason: string;
  expires: string;
};

type AuditInput = {
  npmAudit: unknown;
  cargoAudit: unknown;
  exceptions: Exception[];
  now?: Date;
};

type Blocker = {
  key: string;
  ecosystem: "npm" | "cargo";
  advisory: string;
  package: string;
  severity: string;
};

type EvidenceRow = {
  ecosystem: "npm" | "cargo";
  category: string;
  advisory: string;
  package: string;
  version: string;
  count: number;
  severity?: string;
};

type EvidenceSummary = {
  total: number;
  shown: number;
  truncated: number;
  rows: EvidenceRow[];
};

type Provenance = {
  currentness: "unverified";
  sources: string[];
};

type AuditResult = {
  errors: string[];
  blockers: Blocker[];
  approved: Blocker[];
  evidence: EvidenceSummary;
  provenance: Provenance;
};

type AuditModule = {
  MAX_EVIDENCE_ROWS: number;
  evaluateDependencyAudit: (input: AuditInput) => AuditResult;
  executeDependencyAuditCli: (options?: {
    evaluate?: () => AuditResult;
    exitProcess?: (code: number) => void;
    summaryPath?: string | null;
    writeError?: (message: string) => void;
    writeLine?: (message: string) => void;
  }) => void;
  formatAuditEvidence: (result: AuditResult) => { consoleText: string; markdown: string };
  runJsonCommand: (command: string, args: string[], label: string) => unknown;
};

// @ts-expect-error The dependency audit helper is an ESM Node script outside the TS source tree.
const auditModule = await import("../../../scripts/check/check-dependency-audit.mjs");
const {
  MAX_EVIDENCE_ROWS,
  evaluateDependencyAudit,
  executeDependencyAuditCli,
  formatAuditEvidence,
  runJsonCommand,
} = auditModule as AuditModule;

const tempDirs: string[] = [];

afterEach(() => {
  for (const tempDir of tempDirs.splice(0)) {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

function makeTempDir() {
  const tempDir = mkdtempSync(join(tmpdir(), "dependency-audit-"));
  tempDirs.push(tempDir);
  return tempDir;
}

function npmAudit(advisories: unknown[] = []) {
  return { advisories: Object.fromEntries(advisories.map((entry, index) => [index, entry])) };
}

function cargoAudit(findings: unknown[] = [], warnings: Record<string, unknown[]> = {}) {
  return { vulnerabilities: { list: findings }, warnings };
}

function npmFinding(id = "GHSA-1111-2222-3333", severity = "high", version = "1.0.0") {
  return {
    github_advisory_id: id,
    module_name: "example",
    severity,
    findings: [{ version }],
  };
}

function cargoFinding(id = "RUSTSEC-2026-0001") {
  return {
    advisory: { id },
    package: {
      name: "example-rust-package",
      version: "1.0.0",
      source: "registry+https://github.com/rust-lang/crates.io-index",
    },
  };
}

function cargoWarning(
  kind: string,
  overrides: {
    advisoryId?: string | null;
    name?: string;
    version?: string;
  } = {},
) {
  const advisory =
    overrides.advisoryId === null
      ? null
      : { id: overrides.advisoryId ?? "RUSTSEC-2021-0145" };
  return {
    kind,
    package: {
      name: overrides.name ?? "example-unmaintained",
      version: overrides.version ?? "0.2.14",
    },
    advisory,
  };
}

function exception(overrides: Partial<Exception> = {}): Exception {
  return {
    ecosystem: "npm",
    advisory: "GHSA-1111-2222-3333",
    owner: "maintainer",
    reason: "No patched stable release is available.",
    expires: "2026-08-11",
    ...overrides,
  };
}

function evaluate(overrides: Partial<AuditInput> = {}) {
  return evaluateDependencyAudit({
    npmAudit: npmAudit(),
    cargoAudit: cargoAudit(),
    exceptions: [],
    now: new Date("2026-07-28T00:00:00.000Z"),
    ...overrides,
  });
}

function captureCli(evaluateResult: () => AuditResult, summaryPath: string | null = null) {
  const lines: string[] = [];
  const errors: string[] = [];
  let exitCode: number | undefined;
  executeDependencyAuditCli({
    evaluate: evaluateResult,
    summaryPath,
    writeLine: (message) => lines.push(message),
    writeError: (message) => errors.push(message),
    exitProcess: (code) => {
      exitCode = code;
    },
  });
  return {
    exitCode,
    lines,
    errors,
    output: [...lines, ...errors].join("\n"),
  };
}

function evidenceCounts(text: string) {
  const match = text.match(/total[=:]?\s*(\d+).*shown[=:]?\s*(\d+).*truncated[=:]?\s*(\d+)/i);
  expect(match).not.toBeNull();
  return {
    total: Number(match?.[1]),
    shown: Number(match?.[2]),
    truncated: Number(match?.[3]),
  };
}

describe("dependency audit policy", () => {
  it("rejects an unapproved npm high advisory", () => {
    const result = evaluate({ npmAudit: npmAudit([npmFinding()]) });
    expect(result.errors).toEqual([
      "Unapproved high advisory npm:GHSA-1111-2222-3333 affects example.",
    ]);
    expect(result.blockers).toEqual([
      {
        key: "npm:GHSA-1111-2222-3333",
        ecosystem: "npm",
        advisory: "GHSA-1111-2222-3333",
        package: "example",
        severity: "high",
      },
    ]);
    expect(result.approved).toEqual([]);
  });

  it("rejects an unapproved npm critical advisory", () => {
    const result = evaluate({
      npmAudit: npmAudit([npmFinding("GHSA-aaaa-bbbb-cccc", "critical")]),
    });
    expect(result.errors).toEqual([
      "Unapproved critical advisory npm:GHSA-aaaa-bbbb-cccc affects example.",
    ]);
    expect(result.blockers).toEqual([
      {
        key: "npm:GHSA-aaaa-bbbb-cccc",
        ecosystem: "npm",
        advisory: "GHSA-aaaa-bbbb-cccc",
        package: "example",
        severity: "critical",
      },
    ]);
  });

  it("allows only an exact, current exception", () => {
    const result = evaluate({
      npmAudit: npmAudit([npmFinding()]),
      exceptions: [exception()],
    });
    expect(result.errors).toEqual([]);
    expect(result.approved.map((entry) => entry.key)).toEqual(["npm:GHSA-1111-2222-3333"]);
    expect(captureCli(() => result).exitCode).toBeUndefined();
  });

  it("rejects expired and unused exceptions", () => {
    const expired = evaluate({
      npmAudit: npmAudit([npmFinding()]),
      exceptions: [exception({ expires: "2026-07-27" })],
    });
    expect(expired.errors).toContainEqual(expect.stringContaining("expired"));
    expect(captureCli(() => expired).exitCode).toBe(1);

    const unused = evaluate({ exceptions: [exception()] });
    expect(unused.errors).toContain("Exception npm:GHSA-1111-2222-3333 is unused.");
    expect(captureCli(() => unused).exitCode).toBe(1);
  });

  it("rejects duplicate, malformed, and cross-ecosystem exceptions", () => {
    const duplicate = evaluate({ exceptions: [exception(), exception()] });
    expect(duplicate.errors).toContainEqual(expect.stringContaining("duplicates"));

    const malformed = evaluate({
      exceptions: [{ ...exception(), extra: true } as unknown as Exception],
    });
    expect(malformed.errors).toContainEqual(expect.stringContaining("fields must be exactly"));

    const crossEcosystem = evaluate({
      exceptions: [exception({ ecosystem: "cargo" })],
    });
    expect(crossEcosystem.errors).toContainEqual(expect.stringContaining("does not match ecosystem"));
  });

  it("rejects an unapproved Cargo vulnerability", () => {
    const result = evaluate({ cargoAudit: cargoAudit([cargoFinding()]) });
    expect(result.errors).toEqual([
      "Unapproved vulnerability advisory cargo:RUSTSEC-2026-0001 affects example-rust-package.",
    ]);
    expect(result.blockers).toEqual([
      {
        key: "cargo:RUSTSEC-2026-0001",
        ecosystem: "cargo",
        advisory: "RUSTSEC-2026-0001",
        package: "example-rust-package",
        severity: "vulnerability",
      },
    ]);
  });

  it("allows only an exact Cargo exception", () => {
    const result = evaluate({
      cargoAudit: cargoAudit([cargoFinding()]),
      exceptions: [
        exception({
          ecosystem: "cargo",
          advisory: "RUSTSEC-2026-0001",
        }),
      ],
    });
    expect(result.errors).toEqual([]);
    expect(result.approved.map((entry) => entry.key)).toEqual(["cargo:RUSTSEC-2026-0001"]);
  });

  it("does not block lower npm severities or Cargo informational warnings", () => {
    const result = evaluate({
      npmAudit: npmAudit([
        npmFinding("GHSA-1111-2222-3333", "moderate"),
        npmFinding("GHSA-4444-5555-6666", "low"),
      ]),
      cargoAudit: cargoAudit([], {
        unmaintained: [cargoWarning("unmaintained")],
        unsound: [cargoWarning("unsound", { advisoryId: "RUSTSEC-2022-0001", name: "example-unsound" })],
      }),
    });
    expect(result.errors).toEqual([]);
    expect(result.blockers).toEqual([]);
    expect(captureCli(() => result).exitCode).toBeUndefined();
  });

  it("does not match exceptions against evidence findings", () => {
    const result = evaluate({
      npmAudit: npmAudit([npmFinding("GHSA-1111-2222-3333", "moderate")]),
      exceptions: [exception()],
    });
    expect(result.blockers).toEqual([]);
    expect(result.errors).toContain("Exception npm:GHSA-1111-2222-3333 is unused.");
  });

  it("fails closed when either audit format is malformed", () => {
    expect(evaluate({ npmAudit: {} }).errors).toContainEqual(
      expect.stringContaining("missing the advisories object"),
    );
    expect(evaluate({ cargoAudit: {} }).errors).toContainEqual(
      expect.stringContaining("missing vulnerabilities.list"),
    );
  });
});

describe("dependency audit evidence", () => {
  it("normalizes npm moderate and low findings with occurrence counts", () => {
    const result = evaluate({
      npmAudit: npmAudit([
        npmFinding("GHSA-1111-2222-3333", "moderate", "1.2.3"),
        npmFinding("GHSA-4444-5555-6666", "low", "0.4.0"),
      ]),
    });
    expect(result.errors).toEqual([]);
    expect(result.evidence.rows).toEqual([
      {
        ecosystem: "npm",
        category: "low",
        severity: "low",
        advisory: "GHSA-4444-5555-6666",
        package: "example",
        version: "0.4.0",
        count: 1,
      },
      {
        ecosystem: "npm",
        category: "moderate",
        severity: "moderate",
        advisory: "GHSA-1111-2222-3333",
        package: "example",
        version: "1.2.3",
        count: 1,
      },
    ]);
  });

  it("normalizes Cargo unmaintained and unsound warnings, using unknown when advisory is missing", () => {
    const result = evaluate({
      cargoAudit: cargoAudit([], {
        unsound: [
          cargoWarning("unsound", {
            advisoryId: null,
            name: "example-unsound",
            version: "0.9.0",
          }),
        ],
        unmaintained: [cargoWarning("unmaintained")],
      }),
    });
    expect(result.errors).toEqual([]);
    expect(result.evidence.rows).toEqual([
      {
        ecosystem: "cargo",
        category: "unmaintained",
        advisory: "RUSTSEC-2021-0145",
        package: "example-unmaintained",
        version: "0.2.14",
        count: 1,
      },
      {
        ecosystem: "cargo",
        category: "unsound",
        advisory: "unknown",
        package: "example-unsound",
        version: "0.9.0",
        count: 1,
      },
    ]);
  });

  it("keeps high and critical npm findings on the policy plane only", () => {
    const result = evaluate({
      npmAudit: npmAudit([
        npmFinding("GHSA-1111-2222-3333", "high"),
        npmFinding("GHSA-aaaa-bbbb-cccc", "moderate", "2.0.0"),
      ]),
    });
    expect(result.blockers.map((blocker) => blocker.key)).toEqual(["npm:GHSA-1111-2222-3333"]);
    expect(result.evidence.rows.map((row) => row.advisory)).toEqual(["GHSA-aaaa-bbbb-cccc"]);
  });

  it("deduplicates shuffled duplicate findings into a byte-stable sorted list", () => {
    const duplicateModerate = npmFinding("GHSA-zzzz-yyyy-xxxx", "moderate", "3.1.0");
    const low = npmFinding("GHSA-1111-2222-3333", "low", "1.0.0");
    const unsound = cargoWarning("unsound", { advisoryId: "RUSTSEC-2020-0002", name: "beta" });
    const unmaintained = cargoWarning("unmaintained", { advisoryId: "RUSTSEC-2020-0001", name: "alpha" });
    const shuffled = evaluate({
      npmAudit: npmAudit([duplicateModerate, low, duplicateModerate]),
      cargoAudit: cargoAudit([], {
        unsound: [unsound],
        unmaintained: [unmaintained, unmaintained],
      }),
    });
    const ordered = evaluate({
      npmAudit: npmAudit([low, duplicateModerate, duplicateModerate]),
      cargoAudit: cargoAudit([], {
        unmaintained: [unmaintained, unmaintained],
        unsound: [unsound],
      }),
    });

    expect(JSON.stringify(shuffled.evidence)).toBe(JSON.stringify(ordered.evidence));
    expect(shuffled.evidence.rows).toEqual([
      {
        ecosystem: "cargo",
        category: "unmaintained",
        advisory: "RUSTSEC-2020-0001",
        package: "alpha",
        version: "0.2.14",
        count: 2,
      },
      {
        ecosystem: "cargo",
        category: "unsound",
        advisory: "RUSTSEC-2020-0002",
        package: "beta",
        version: "0.2.14",
        count: 1,
      },
      {
        ecosystem: "npm",
        category: "low",
        severity: "low",
        advisory: "GHSA-1111-2222-3333",
        package: "example",
        version: "1.0.0",
        count: 1,
      },
      {
        ecosystem: "npm",
        category: "moderate",
        severity: "moderate",
        advisory: "GHSA-zzzz-yyyy-xxxx",
        package: "example",
        version: "3.1.0",
        count: 2,
      },
    ]);
  });

  it("truncates overflow evidence to the fixed row cap", () => {
    const overflow = MAX_EVIDENCE_ROWS + 5;
    const warnings = Array.from({ length: overflow }, (_, index) =>
      cargoWarning("unmaintained", {
        advisoryId: `RUSTSEC-2026-${String(index).padStart(4, "0")}`,
        name: `crate-${String(index).padStart(3, "0")}`,
      }),
    );
    const reversed = evaluate({
      cargoAudit: cargoAudit([], { unmaintained: [...warnings].reverse() }),
    });
    expect(reversed.evidence.total).toBe(overflow);
    expect(reversed.evidence.shown).toBe(MAX_EVIDENCE_ROWS);
    expect(reversed.evidence.truncated).toBe(5);
    expect(reversed.evidence.rows).toHaveLength(MAX_EVIDENCE_ROWS);
    expect(reversed.evidence.rows[0]?.advisory).toBe("RUSTSEC-2026-0000");
    expect(reversed.evidence.rows[MAX_EVIDENCE_ROWS - 1]?.advisory).toBe(
      `RUSTSEC-2026-${String(MAX_EVIDENCE_ROWS - 1).padStart(4, "0")}`,
    );
    expect(reversed.errors).toEqual([]);
  });

  it("labels provenance as unverified when reports omit a verifiable update time", () => {
    const result = evaluate({
      npmAudit: npmAudit([npmFinding("GHSA-1111-2222-3333", "moderate")]),
    });
    expect(result.provenance).toEqual({
      currentness: "unverified",
      sources: ["pnpm audit --prod --json", "cargo audit --file src-tauri/Cargo.lock --json"],
    });
  });
});

describe("dependency audit rendering", () => {
  it("writes only the console when GITHUB_STEP_SUMMARY is absent", () => {
    const summaryPath = join(makeTempDir(), "step-summary.md");
    const previous = process.env.GITHUB_STEP_SUMMARY;
    delete process.env.GITHUB_STEP_SUMMARY;
    try {
      const result = evaluate({
        npmAudit: npmAudit([npmFinding("GHSA-1111-2222-3333", "moderate")]),
      });
      const captured = captureCli(() => result);
      expect(captured.exitCode).toBeUndefined();
      expect(captured.output).toContain("currentness: unverified");
      expect(captured.output).toContain("total=1 shown=1 truncated=0");
      expect(captured.output).not.toMatch(/latest/i);
      expect(existsSync(summaryPath)).toBe(false);
    } finally {
      if (previous === undefined) delete process.env.GITHUB_STEP_SUMMARY;
      else process.env.GITHUB_STEP_SUMMARY = previous;
    }
  });

  it("appends the same evidence counts to a declared Step Summary file", () => {
    const summaryPath = join(makeTempDir(), "step-summary.md");
    writeFileSync(summaryPath, "existing\n");
    const result = evaluate({
      npmAudit: npmAudit([npmFinding("GHSA-1111-2222-3333", "moderate", "1.2.3")]),
      cargoAudit: cargoAudit([], {
        unmaintained: [cargoWarning("unmaintained")],
      }),
    });
    const formatted = formatAuditEvidence(result);
    const captured = captureCli(() => result, summaryPath);
    const markdown = readFileSync(summaryPath, "utf8");
    expect(captured.exitCode).toBeUndefined();
    expect(captured.lines[0]).toBe(formatted.consoleText);
    expect(markdown).toBe(`existing\n${formatted.markdown}`);
    expect(evidenceCounts(captured.output)).toEqual(evidenceCounts(markdown));
    expect(markdown).toContain("currentness: unverified");
    expect(markdown).not.toMatch(/latest/i);
    expect(captured.output).not.toMatch(/latest/i);
  });

  it("keeps warning findings from changing the exit code while still showing truncation", () => {
    const overflow = MAX_EVIDENCE_ROWS + 3;
    const warnings = Array.from({ length: overflow }, (_, index) =>
      cargoWarning("unmaintained", {
        advisoryId: `RUSTSEC-2025-${String(index).padStart(4, "0")}`,
        name: `warn-${index}`,
      }),
    );
    const result = evaluate({ cargoAudit: cargoAudit([], { unmaintained: warnings }) });
    const captured = captureCli(() => result);
    const findingLines = captured.output
      .split("\n")
      .filter((line) => line.includes(" count="));
    expect(captured.exitCode).toBeUndefined();
    expect(captured.output).toContain(`[audit] Passed.`);
    expect(captured.output).toContain(`total=${overflow} shown=${MAX_EVIDENCE_ROWS} truncated=3`);
    expect(findingLines).toHaveLength(MAX_EVIDENCE_ROWS);
  });
});

describe("dependency audit infrastructure failures", () => {
  it("fails closed for command start, empty JSON, and invalid JSON", () => {
    const cases = [
      [
        "command start failure",
        () => runJsonCommand("this-audit-command-does-not-exist", [], "pnpm audit"),
        /Failed to start pnpm audit/,
      ],
      [
        "empty JSON",
        () => runJsonCommand(process.execPath, ["-e", "process.exit(0)"], "pnpm audit"),
        /produced no JSON/,
      ],
      [
        "invalid JSON",
        () => runJsonCommand(process.execPath, ["-e", "process.stdout.write('{')"], "pnpm audit"),
        /produced invalid JSON/,
      ],
    ] as const;

    for (const [, run, pattern] of cases) {
      expect(run).toThrowError(pattern);
      const captured = captureCli(() => {
        run();
        return evaluate();
      });
      expect(captured.exitCode).toBe(1);
      expect(captured.output).not.toContain("[audit] Passed");
      expect(captured.errors.join("\n")).toMatch(pattern);
    }
  });

  it("fails closed for a malformed blocker report and does not print Passed", () => {
    const result = evaluate({ npmAudit: {}, cargoAudit: {} });
    const captured = captureCli(() => result);
    expect(result.errors).toEqual(expect.arrayContaining([
      expect.stringContaining("missing the advisories object"),
      expect.stringContaining("missing vulnerabilities.list"),
    ]));
    expect(captured.exitCode).toBe(1);
    expect(captured.output).not.toContain("[audit] Passed");
  });

  it("fails closed when a declared Step Summary path cannot be written", () => {
    const summaryPath = makeTempDir();
    const result = evaluate({
      npmAudit: npmAudit([npmFinding("GHSA-1111-2222-3333", "moderate")]),
    });
    const captured = captureCli(() => result, summaryPath);
    expect(captured.exitCode).toBe(1);
    expect(captured.output).not.toContain("[audit] Passed");
    expect(captured.errors.join("\n")).toContain("Failed to write GitHub Step Summary");
  });
});
