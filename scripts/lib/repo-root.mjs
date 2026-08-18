import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const PACKAGE_NAME = "skillport";

export function resolveRepoRoot(fromUrl) {
  let currentDir = dirname(fileURLToPath(fromUrl));

  while (true) {
    try {
      const pkg = JSON.parse(readFileSync(join(currentDir, "package.json"), "utf8"));
      if (pkg.name === PACKAGE_NAME) {
        return currentDir;
      }
    } catch (error) {
      if (error && error.code !== "ENOENT") {
        // Invalid or unreadable package.json: keep walking.
      }
    }

    const parentDir = dirname(currentDir);
    if (parentDir === currentDir) {
      throw new Error(
        `Could not find repository root from ${fromUrl}. Expected a package.json with name "${PACKAGE_NAME}".`,
      );
    }
    currentDir = parentDir;
  }
}
