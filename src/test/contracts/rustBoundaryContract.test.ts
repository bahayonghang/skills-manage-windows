/// <reference types="node" />

import { describe, expect, it } from "vitest";

import {
  FACADE_AND_OWNER_EXCLUSIONS,
  MIGRATION_TREES,
  SCAN_ROOTS,
  TEST_FILE_ALLOWLIST,
  formatHits,
  scanRustBoundaries,
} from "./rustBoundaryScan";

/**
 * Historical production `crate::db::<repo_fn>` / `db::<repo_fn>` hits outside
 * the three migration trees. Measured 2026-09-03; do not replace with the
 * audit's file-count "87".
 */
const HISTORICAL_WIDE_REPO_FN_BASELINE = 388;

describe("rust boundary contract", () => {
  const scan = scanRustBoundaries();

  it("records fixed scan roots, exclusions, and per-file hits", () => {
    expect(SCAN_ROOTS).toEqual({
      servicesCommands: "src-tauri/src/services",
      wideRepoFns: "src-tauri/src",
      repoFunctionOwners: "src-tauri/src/db/repos/*_repo.rs",
    });
    expect([...MIGRATION_TREES]).toEqual([
      "src-tauri/src/services/central_updates",
      "src-tauri/src/services/skills_cli",
      "src-tauri/src/services/installation",
    ]);
    expect(TEST_FILE_ALLOWLIST.directorySegments).toEqual(["tests"]);
    expect(TEST_FILE_ALLOWLIST.exactBasenames).toEqual(["tests.rs"]);
    expect(TEST_FILE_ALLOWLIST.basenameSuffixes).toEqual(["_tests.rs"]);
    expect(TEST_FILE_ALLOWLIST.basenamePrefixes).toEqual(["test_"]);
    expect([...FACADE_AND_OWNER_EXCLUSIONS]).toEqual([
      "src-tauri/src/db/mod.rs",
      "src-tauri/src/db/repos/**",
    ]);
    expect(scan.repoFunctionCount).toBeGreaterThan(0);
  });

  it("keeps production services off crate::commands", () => {
    expect(
      scan.servicesCommandsHits,
      formatHits(scan.servicesCommandsHits),
    ).toEqual([]);
  });

  it("routes the three service trees through db::repos owners", () => {
    expect(scan.migrationHits, formatHits(scan.migrationHits)).toEqual([]);
  });

  it("does not grow historical wide repository function calls", () => {
    expect(
      scan.historicalHits.length,
      `baseline ${HISTORICAL_WIDE_REPO_FN_BASELINE}; current ${scan.historicalHits.length}\n${formatHits(scan.historicalHits)}`,
    ).toBeLessThanOrEqual(HISTORICAL_WIDE_REPO_FN_BASELINE);
  });
});
