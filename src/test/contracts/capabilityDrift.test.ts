import { describe, expect, it } from "vitest";

type CapabilityState = Record<
  | "capabilityPermissions"
  | "frontendPluginImports"
  | "frontendPluginDependencies"
  | "rustPluginDependencies"
  | "rustPluginInitializers",
  string[]
>;

type CapabilityCheckHelpers = {
  parseCapabilityContract: (document: string) => CapabilityState;
  renderCapabilityTable: (contract: CapabilityState) => string;
  validateCapabilityContract: (observed: CapabilityState, contract: CapabilityState) => string[];
  validateRenderedTable: (document: string, contract: CapabilityState) => string[];
};

// @ts-expect-error The capability checker is an ESM Node script outside the TS source tree.
const capabilityCheckModule = await import("../../../scripts/check/check-capability-drift.mjs");

const {
  parseCapabilityContract,
  renderCapabilityTable,
  validateCapabilityContract,
  validateRenderedTable,
} = capabilityCheckModule as CapabilityCheckHelpers;

const contract = {
  capabilityPermissions: ["dialog:allow-open"],
  frontendPluginImports: ["@tauri-apps/plugin-dialog"],
  frontendPluginDependencies: ["@tauri-apps/plugin-dialog"],
  rustPluginDependencies: ["tauri-plugin-dialog"],
  rustPluginInitializers: ["tauri-plugin-dialog"],
};

function documentFor(candidate = contract, table = renderCapabilityTable(candidate)) {
  return `<!-- capability-contract:start -->\n\`\`\`json\n${JSON.stringify(candidate)}\n\`\`\`\n<!-- capability-contract:end -->\n<!-- capability-table:start -->\n${table}\n<!-- capability-table:end -->`;
}

describe("capability drift contract", () => {
  it("accepts matching exact sets and rendered table", () => {
    const parsed = parseCapabilityContract(documentFor());
    expect(validateCapabilityContract(contract, parsed)).toEqual([]);
    expect(validateRenderedTable(documentFor(), parsed)).toEqual([]);
  });

  it("rejects a missing permission", () => {
    expect(
      validateCapabilityContract({ ...contract, capabilityPermissions: [] }, contract),
    ).toContainEqual(expect.stringContaining("capabilityPermissions drifted"));
  });

  it("rejects a stale JSON entry", () => {
    const stale = {
      ...contract,
      frontendPluginImports: [...contract.frontendPluginImports, "@tauri-apps/plugin-fs"],
    };
    expect(validateCapabilityContract(contract, stale)).toContainEqual(
      expect.stringContaining("frontendPluginImports drifted"),
    );
  });

  it("rejects a stale rendered table", () => {
    expect(validateRenderedTable(documentFor(contract, "| stale | table |"), contract)).toEqual([
      expect.stringContaining("table is stale"),
    ]);
  });

  it("rejects an unexpected frontend plugin import", () => {
    const observed = {
      ...contract,
      frontendPluginImports: [...contract.frontendPluginImports, "@tauri-apps/plugin-fs"],
    };
    expect(validateCapabilityContract(observed, contract).join("\n")).toContain(
      "@tauri-apps/plugin-fs",
    );
  });

  it("rejects stale plugin dependency and initializer sets", () => {
    const observed = {
      ...contract,
      rustPluginDependencies: [],
      rustPluginInitializers: [],
    };
    const errors = validateCapabilityContract(observed, contract).join("\n");
    expect(errors).toContain("rustPluginDependencies drifted");
    expect(errors).toContain("rustPluginInitializers drifted");
  });
});
