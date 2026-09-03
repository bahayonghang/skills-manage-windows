/// <reference types="node" />

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { extname, join, relative, resolve } from "node:path";

import { describe, expect, it } from "vitest";

const SRC_ROOT = resolve(process.cwd(), "src");
const TEST_ROOT = resolve(SRC_ROOT, "test");
const TYPES_ROOT = resolve(SRC_ROOT, "types");

/**
 * Step 0 production `@/types` barrel importer count.
 * Measured with:
 * `rg -l "from\\s+[\`\"']@/types[\`\"']" src --glob '*.ts' --glob '*.tsx' --glob '!src/test/**' --glob '!src/types/**' | Sort-Object -Unique`
 * Do not replace this with the audit's 193.
 */
const ROOT_TYPES_BARREL_BASELINE = 199;

const TAURI_EVENT_ALLOWLIST = ["src/lib/ipc/invoke.ts"] as const;

const DEAD_PRODUCTION_PATHS = [
  "src/pages/CollectionView.tsx",
  "src/components/marketplace/SkillPreviewDialog.tsx",
  "src/components/platform/DuplicatePlatformSkillsDialog.tsx",
  "src/components/skill/SkillDetailPanelShell.tsx",
] as const;

const DEAD_ISOLATED_TEST_PATHS = [
  "src/test/pages/CollectionView.test.tsx",
  "src/test/components/marketplace/SkillPreviewDialog.test.tsx",
] as const;

const DEAD_MODULE_SPECIFIER =
  /(?:from|import\()\s*['"][^'"]*(?:CollectionView|SkillPreviewDialog|DuplicatePlatformSkillsDialog|SkillDetailPanelShell)['"]/;

const ROOT_TYPES_BARREL_IMPORT = /from\s+['"]@\/types['"]/;
const TAURI_EVENT_IMPORT = /@tauri-apps\/api\/event/;
const PAGE_IMPORT = /from\s+['"]@\/pages\/([^'"]+)['"]/g;
const UPDATE_CENTER_PAGE = /(?:^|\/)(?:centralUpdateCheckMode|updateCheckMode|UpdateCenter)/i;
const PAGES_OR_STORES_IMPORT = /from\s+['"]@\/(?:pages|stores)\//;

const FINDING_HARDCODED_FIXTURE_COPY = [
  "AI connection is unavailable in the browser fixture.",
  "Preview unavailable in browser mode",
  "Open this flow in the desktop app to browse and install repository skills.",
  "浏览器模式下暂不支持预览",
  "请在桌面应用中打开此流程，以浏览并安装仓库里的技能。",
] as const;

function walkTsFiles(dir: string): string[] {
  const entries: string[] = [];
  const stack = [dir];
  while (stack.length > 0) {
    const current = stack.pop()!;
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const full = join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
        continue;
      }
      if (!entry.isFile()) continue;
      const ext = extname(entry.name);
      if (ext === ".ts" || ext === ".tsx") {
        entries.push(full);
      }
    }
  }
  return entries.sort();
}

function toRepoPath(file: string): string {
  return relative(process.cwd(), file).replace(/\\/g, "/");
}

const ALL_SRC_TS = walkTsFiles(SRC_ROOT);
const PRODUCTION_FILES = ALL_SRC_TS.filter(
  (file) => !resolve(file).startsWith(TEST_ROOT),
);
const BARREL_SCAN_FILES = PRODUCTION_FILES.filter(
  (file) => !resolve(file).startsWith(TYPES_ROOT),
);

function rootTypesBarrelImporters(): string[] {
  return BARREL_SCAN_FILES.filter((file) =>
    ROOT_TYPES_BARREL_IMPORT.test(readFileSync(file, "utf8")),
  ).map(toRepoPath);
}

describe("frontend architecture contract", () => {
  it("scans production TypeScript files", () => {
    expect(PRODUCTION_FILES.length).toBeGreaterThan(0);
    expect(BARREL_SCAN_FILES.length).toBeGreaterThan(0);
  });

  it("keeps update-check mode in a page-free lib module", () => {
    const owner = resolve(SRC_ROOT, "lib", "updateCheckMode.ts");
    expect(existsSync(owner)).toBe(true);
    const content = readFileSync(owner, "utf8");
    expect(content).toMatch(/export type UpdateCheckMode/);
    expect(content).toMatch(/CENTRAL_UPDATE_CHECK_MODE_SETTING_KEY/);
    expect(content).toMatch(/DEFAULT_UPDATE_CHECK_MODE/);
    expect(content).toMatch(/function normalizeUpdateCheckMode/);
    expect(content).not.toMatch(PAGES_OR_STORES_IMPORT);
  });

  it("forbids update-center lib/store imports of pages", () => {
    const libAndStores = PRODUCTION_FILES.filter((file) => {
      const path = toRepoPath(file);
      return path.startsWith("src/lib/") || path.startsWith("src/stores/");
    });
    const offenders: string[] = [];
    for (const file of libAndStores) {
      const content = readFileSync(file, "utf8");
      for (const match of content.matchAll(PAGE_IMPORT)) {
        const specifier = match[1];
        if (UPDATE_CENTER_PAGE.test(specifier)) {
          offenders.push(`${toRepoPath(file)} -> @/pages/${specifier}`);
        }
      }
    }
    expect(offenders, offenders.join("\n")).toEqual([]);
  });

  it("allows @tauri-apps/api/event only in the IPC adapter", () => {
    const matches = PRODUCTION_FILES.filter((file) =>
      TAURI_EVENT_IMPORT.test(readFileSync(file, "utf8")),
    ).map(toRepoPath);
    expect(matches).toEqual([...TAURI_EVENT_ALLOWLIST]);
  });

  it("removes unreachable production modules and their isolated tests", () => {
    for (const path of [...DEAD_PRODUCTION_PATHS, ...DEAD_ISOLATED_TEST_PATHS]) {
      expect(existsSync(resolve(process.cwd(), path)), path).toBe(false);
    }
  });

  it("keeps production module specifiers off deleted modules", () => {
    const offenders = PRODUCTION_FILES.filter((file) =>
      DEAD_MODULE_SPECIFIER.test(readFileSync(file, "utf8")),
    ).map(toRepoPath);
    expect(offenders, offenders.join("\n")).toEqual([]);
  });

  it("does not grow the production @/types barrel importer set", () => {
    const importers = rootTypesBarrelImporters();
    expect(
      importers.length,
      `baseline ${ROOT_TYPES_BARREL_BASELINE}; current ${importers.length}\n${importers.join("\n")}`,
    ).toBeLessThanOrEqual(ROOT_TYPES_BARREL_BASELINE);
  });

  it("does not keep finding hardcoded browser-fixture copy in production TS", () => {
    const offenders: string[] = [];
    for (const file of PRODUCTION_FILES) {
      const content = readFileSync(file, "utf8");
      for (const sentence of FINDING_HARDCODED_FIXTURE_COPY) {
        if (content.includes(sentence)) {
          offenders.push(`${toRepoPath(file)}: ${sentence}`);
        }
      }
    }
    expect(offenders, offenders.join("\n")).toEqual([]);
  });
});
