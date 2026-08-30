#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

export function parseArgs(argv) {
  const args = {
    version: process.env.RELEASE_VERSION || process.env.GITHUB_REF_NAME?.replace(/^v/, "") || "",
    output: "release-body.md",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--version" && next) {
      args.version = next.replace(/^v/, "");
      index += 1;
    } else if (arg === "--output" && next) {
      args.output = next;
      index += 1;
    } else if (arg === "--help" || arg === "-h") {
      console.log("Usage: node scripts/release/prepare-release-body.mjs --version <version> [--output release-body.md]");
      process.exit(0);
    }
  }

  if (!args.version) {
    throw new Error("Missing release version. Pass --version or set RELEASE_VERSION/GITHUB_REF_NAME.");
  }

  return args;
}

export function resolveReleaseBody(version) {
  const majorMinor = version.split(".").slice(0, 2).join(".");
  const notesPath = path.join("release-notes", `${version}.md`);
  const seriesNotesPath = path.join("release-notes", `${majorMinor}.md`);

  if (fs.existsSync(notesPath)) {
    return {
      body: fs.readFileSync(notesPath, "utf8"),
      source: notesPath,
    };
  }

  if (fs.existsSync(seriesNotesPath)) {
    return {
      body: fs.readFileSync(seriesNotesPath, "utf8"),
      source: seriesNotesPath,
    };
  }

  return {
    body: [
      `# SkillPort ${version}`,
      "",
      `Desktop packages for SkillPort ${version}.`,
      "",
      `No release notes file was found at ${notesPath} or ${seriesNotesPath}.`,
      "",
    ].join("\n"),
    source: "generated fallback",
  };
}

export function writeReleaseBody(args) {
  const releaseBody = resolveReleaseBody(args.version);
  fs.writeFileSync(args.output, releaseBody.body);
  return releaseBody.source;
}

const entryPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const args = parseArgs(process.argv.slice(2));
  const source = writeReleaseBody(args);
  console.log(`Wrote ${args.output} from ${source}.`);
}
