/// <reference types="node" />

import { describe, expect, it } from "vitest";

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

type AuditResult = {
  errors: string[];
  blockers: Array<{ key: string }>;
  approved: Array<{ key: string }>;
};

type AuditModule = {
  evaluateDependencyAudit: (input: AuditInput) => AuditResult;
};

// @ts-expect-error The dependency audit helper is an ESM Node script outside the TS source tree.
const auditModule = await import("../../../scripts/check/check-dependency-audit.mjs");
const { evaluateDependencyAudit } = auditModule as AuditModule;

function npmAudit(advisories: unknown[] = []) {
  return { advisories: Object.fromEntries(advisories.map((entry, index) => [index, entry])) };
}

function cargoAudit(findings: unknown[] = []) {
  return { vulnerabilities: { list: findings }, warnings: {} };
}

function npmFinding(id = "GHSA-1111-2222-3333", severity = "high") {
  return { github_advisory_id: id, module_name: "example", severity };
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

describe("dependency audit policy", () => {
  it("rejects an unapproved npm high advisory", () => {
    const result = evaluate({ npmAudit: npmAudit([npmFinding()]) });
    expect(result.errors).toContainEqual(expect.stringContaining("Unapproved high advisory"));
  });

  it("allows only an exact, current exception", () => {
    const result = evaluate({
      npmAudit: npmAudit([npmFinding()]),
      exceptions: [exception()],
    });
    expect(result.errors).toEqual([]);
    expect(result.approved.map((entry) => entry.key)).toEqual(["npm:GHSA-1111-2222-3333"]);
  });

  it("rejects expired and unused exceptions", () => {
    const expired = evaluate({
      npmAudit: npmAudit([npmFinding()]),
      exceptions: [exception({ expires: "2026-07-27" })],
    });
    expect(expired.errors).toContainEqual(expect.stringContaining("expired"));

    const unused = evaluate({ exceptions: [exception()] });
    expect(unused.errors).toContain("Exception npm:GHSA-1111-2222-3333 is unused.");
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
    expect(result.errors).toContainEqual(
      expect.stringContaining("Unapproved vulnerability advisory cargo:RUSTSEC-2026-0001"),
    );
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
      cargoAudit: { vulnerabilities: { list: [] }, warnings: { unmaintained: [{}] } },
    });
    expect(result.errors).toEqual([]);
    expect(result.blockers).toEqual([]);
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
