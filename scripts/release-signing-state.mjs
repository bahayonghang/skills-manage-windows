#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const REQUIRED_AUTHENTICODE_FILES = ["skillport.exe", "nsis", "msi"];
const AUTHENTICODE_STATES = new Set(["valid", "not-configured", "invalid"]);

export function azureSigningConfiguration(values) {
  const required = [
    "AZURE_TENANT_ID",
    "AZURE_CLIENT_ID",
    "AZURE_SUBSCRIPTION_ID",
    "AZURE_ARTIFACT_SIGNING_ENDPOINT",
    "AZURE_ARTIFACT_SIGNING_ACCOUNT",
    "AZURE_ARTIFACT_SIGNING_PROFILE",
  ];
  const missing = required.filter((name) => !String(values[name] ?? "").trim());
  return { configured: missing.length === 0, missing };
}

export function validateSigningState(state, { mode }) {
  if (!state || typeof state !== "object" || Array.isArray(state)) {
    throw new Error("Windows signing state must be a JSON object.");
  }
  if (!AUTHENTICODE_STATES.has(state.authenticode)) {
    throw new Error(`Unknown Authenticode state: ${state.authenticode ?? "<missing>"}.`);
  }
  if (state.updaterSignature !== "valid") {
    throw new Error("Final Windows NSIS updater signature is not valid.");
  }
  if (!state.files || typeof state.files !== "object" || Array.isArray(state.files)) {
    throw new Error("Windows signing state is missing per-file Authenticode results.");
  }

  for (const file of REQUIRED_AUTHENTICODE_FILES) {
    const result = state.files[file];
    if (!result || typeof result !== "object") {
      throw new Error(`Windows signing state is missing ${file}.`);
    }
    if (state.authenticode === "valid") {
      if (result.status !== "Valid" || !result.signer || !result.timestamp) {
        throw new Error(`Authenticode validation failed for ${file}.`);
      }
    } else if (state.authenticode === "not-configured" && result.status !== "NotSigned") {
      throw new Error(`Expected ${file} to be NotSigned when Authenticode is ${state.authenticode}.`);
    } else if (state.authenticode === "invalid") {
      throw new Error(`Authenticode validation failed for ${file}.`);
    }
  }

  if (mode === "publish" && state.authenticode !== "valid") {
    throw new Error("Publish requires valid Azure Artifact Signing Authenticode for every Windows release file.");
  }
  return state;
}

export function readAndValidateSigningState(filePath, options) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Windows signing state not found: ${filePath}`);
  }
  return validateSigningState(JSON.parse(fs.readFileSync(filePath, "utf8")), options);
}

const entryPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const stateIndex = process.argv.indexOf("--state");
  const modeIndex = process.argv.indexOf("--mode");
  const statePath = stateIndex >= 0 ? process.argv[stateIndex + 1] : "windows-signing.json";
  const mode = modeIndex >= 0 ? process.argv[modeIndex + 1] : "rehearsal";
  if (!new Set(["rehearsal", "publish"]).has(mode)) throw new Error("Mode must be rehearsal or publish.");
  const state = readAndValidateSigningState(statePath, { mode });
  console.log(`Windows signing state verified: Authenticode=${state.authenticode}, updater-signature=${state.updaterSignature}.`);
}
