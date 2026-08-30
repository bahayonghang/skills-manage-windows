/// <reference types="node" />

import { describe, expect, it } from "vitest";

type SigningHelpers = {
  azureSigningConfiguration: (values: Record<string, string>) => { configured: boolean; missing: string[] };
  validateSigningState: (state: Record<string, unknown>, options: { mode: string }) => unknown;
};

// @ts-expect-error The release signing helper is an ESM Node script outside the TS source tree.
const module = await import("../../../scripts/release/release-signing-state.mjs");
const { azureSigningConfiguration, validateSigningState } = module as SigningHelpers;

const files = {
  "skillport.exe": { status: "Valid", signer: "CN=SkillPort", timestamp: true },
  nsis: { status: "Valid", signer: "CN=SkillPort", timestamp: true },
  msi: { status: "Valid", signer: "CN=SkillPort", timestamp: true },
};

describe("Windows release signing state", () => {
  it("distinguishes missing Azure configuration from a valid signature", () => {
    expect(azureSigningConfiguration({}).configured).toBe(false);
    expect(azureSigningConfiguration({
      AZURE_TENANT_ID: "tenant",
      AZURE_CLIENT_ID: "client",
      AZURE_SUBSCRIPTION_ID: "subscription",
      AZURE_ARTIFACT_SIGNING_ENDPOINT: "endpoint",
      AZURE_ARTIFACT_SIGNING_ACCOUNT: "account",
      AZURE_ARTIFACT_SIGNING_PROFILE: "profile",
    }).configured).toBe(true);
  });

  it("allows explicit unsigned rehearsal but fails publish closed", () => {
    const rehearsal = {
      authenticode: "not-configured",
      updaterSignature: "valid",
      files: {
        "skillport.exe": { status: "NotSigned" },
        nsis: { status: "NotSigned" },
        msi: { status: "NotSigned" },
      },
    };
    expect(() => validateSigningState(rehearsal, { mode: "rehearsal" })).not.toThrow();
    expect(() => validateSigningState(rehearsal, { mode: "publish" })).toThrow(/Publish requires/);
  });

  it("requires timestamped Authenticode and a valid final-byte updater signature", () => {
    expect(() => validateSigningState({ authenticode: "valid", updaterSignature: "valid", files }, { mode: "publish" })).not.toThrow();
    expect(() => validateSigningState({ authenticode: "valid", updaterSignature: "invalid", files }, { mode: "publish" })).toThrow(/updater signature/);
    expect(() => validateSigningState({ authenticode: "valid", updaterSignature: "valid", files: { ...files, nsis: { status: "Valid", signer: "CN=SkillPort", timestamp: false } } }, { mode: "publish" })).toThrow(/nsis/);
  });

  it("rejects an explicitly invalid Authenticode state even during rehearsal", () => {
    expect(() => validateSigningState({
      authenticode: "invalid",
      updaterSignature: "valid",
      files: {
        "skillport.exe": { status: "NotSigned" },
        nsis: { status: "NotSigned" },
        msi: { status: "NotSigned" },
      },
    }, { mode: "rehearsal" })).toThrow(/Authenticode validation failed/);
  });
});
