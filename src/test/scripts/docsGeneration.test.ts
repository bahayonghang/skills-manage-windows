/// <reference types="node" />

import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";

type GenerateDocs = (options: {
  check?: boolean;
  log?: (message: string) => void;
  outputFile: string;
  rootDir: string;
  sourceDir: string;
}) => string;

// @ts-expect-error The documentation generator is an ESM Node script outside the TS source tree.
const ipcModule = await import("../../../scripts/docs/build-ipc-dict.mjs");
const schemaModule =
  // @ts-expect-error The documentation generator is an ESM Node script outside the TS source tree.
  await import("../../../scripts/docs/build-schema-table.mjs");

const generators: Array<{
  createSource: (sourceDir: string) => void;
  generate: GenerateDocs;
  name: string;
  outputName: string;
}> = [
  {
    name: "IPC dictionary",
    outputName: "ipc-commands.md",
    generate: ipcModule.generateIpcDocs as GenerateDocs,
    createSource(sourceDir) {
      const commandDir = join(sourceDir, "commands");
      mkdirSync(commandDir, { recursive: true });
      writeFileSync(
        join(commandDir, "example.rs"),
        [
          "/// Returns one example value.",
          "#[tauri::command]",
          "pub async fn example_command(input: String) -> crate::ipc_error::IpcResult<()> {",
          "    let _ = input;",
          "    Ok(())",
          "}",
          "",
        ].join("\n"),
        "utf8",
      );
    },
  },
  {
    name: "schema table",
    outputName: "data-model.md",
    generate: schemaModule.generateSchemaDocs as GenerateDocs,
    createSource(sourceDir) {
      mkdirSync(sourceDir, { recursive: true });
      writeFileSync(
        join(sourceDir, "example.rs"),
        [
          'const SQL: &str = "CREATE TABLE IF NOT EXISTS examples (',
          "    id TEXT PRIMARY KEY,",
          "    name TEXT NOT NULL DEFAULT 'example',",
          "    CHECK (name != '')",
          ')";',
          "",
        ].join("\n"),
        "utf8",
      );
    },
  },
];

describe.each(generators)(
  "$name documentation generator",
  ({ createSource, generate, outputName }) => {
    it("produces stable bytes and checks without rewriting", () => {
      const rootDir = mkdtempSync(join(tmpdir(), "skillport-docs-gen-"));
      const sourceDir = join(rootDir, "source");
      const outputFile = join(rootDir, "generated", outputName);
      const silent = () => undefined;

      try {
        createSource(sourceDir);
        generate({ sourceDir, outputFile, rootDir, log: silent });
        const first = readFileSync(outputFile);
        expect(first.toString("utf8")).not.toContain("Last generated:");
        expect(first.toString("utf8")).not.toContain("| `CHECK` |");
        expect(first.toString("utf8")).not.toContain(") ->");

        generate({ sourceDir, outputFile, rootDir, log: silent });
        expect(readFileSync(outputFile).equals(first)).toBe(true);

        const fixedTime = new Date("2020-01-01T00:00:00.000Z");
        utimesSync(outputFile, fixedTime, fixedTime);
        const cleanMtime = statSync(outputFile).mtimeMs;
        generate({ sourceDir, outputFile, rootDir, check: true, log: silent });
        expect(readFileSync(outputFile).equals(first)).toBe(true);
        expect(statSync(outputFile).mtimeMs).toBe(cleanMtime);
      } finally {
        rmSync(rootDir, { recursive: true, force: true });
      }
    });

    it("fails on drift without changing the stale file", () => {
      const rootDir = mkdtempSync(join(tmpdir(), "skillport-docs-drift-"));
      const sourceDir = join(rootDir, "source");
      const outputFile = join(rootDir, "generated", outputName);
      const silent = () => undefined;

      try {
        createSource(sourceDir);
        mkdirSync(join(rootDir, "generated"), { recursive: true });
        writeFileSync(outputFile, "stale\n", "utf8");
        const fixedTime = new Date("2020-01-01T00:00:00.000Z");
        utimesSync(outputFile, fixedTime, fixedTime);
        const staleMtime = statSync(outputFile).mtimeMs;

        expect(() =>
          generate({
            sourceDir,
            outputFile,
            rootDir,
            check: true,
            log: silent,
          }),
        ).toThrow(new RegExp(`${outputName}.*pnpm docs:gen`, "s"));
        expect(readFileSync(outputFile, "utf8")).toBe("stale\n");
        expect(statSync(outputFile).mtimeMs).toBe(staleMtime);
      } finally {
        rmSync(rootDir, { recursive: true, force: true });
      }
    });
  },
);

const repositoryRoot = resolve(process.cwd());

function collectRegistryCommands(macroName: string): string[] {
  const registry = readFileSync(
    join(repositoryRoot, "src-tauri", "src", "ipc_registry.rs"),
    "utf8",
  );
  const macroStart = registry.indexOf(`macro_rules! ${macroName}`);
  const bodyStart = registry.indexOf("$callback! {", macroStart);
  const bodyEnd = registry.indexOf("\n        }\n    };", bodyStart);

  if (macroStart < 0 || bodyStart < 0 || bodyEnd < 0) {
    throw new Error(`Malformed registry macro ${macroName}`);
  }

  return [
    ...registry.slice(bodyStart, bodyEnd).matchAll(/^\s*([a-z0-9_]+)\s*=>/gm),
  ].map((match) => match[1]);
}

it("keeps the generated IPC dictionary equal to the runtime registry", () => {
  const markdown = readFileSync(
    join(
      repositoryRoot,
      "docs",
      "architecture",
      "_generated",
      "ipc-commands.md",
    ),
    "utf8",
  );
  const documented = [...markdown.matchAll(/^\| `([a-z0-9_]+)` \|/gm)].map(
    (match) => match[1],
  );
  const runtime = collectRegistryCommands("__skillport_runtime_commands");

  expect(new Set(documented).size).toBe(documented.length);
  expect([...documented].sort()).toEqual([...runtime].sort());
  expect(markdown).toContain("| Command / action | Log policy |");
  expect(markdown).toContain(
    "| `clear_operation_logs` | operation | Logs | Database | TerminalOnly |",
  );
  expect(markdown).toContain(
    "| `get_operation_log` | runtime-only | — | — | ReadOnly |",
  );
  expect(markdown).toContain(
    "| `record_frontend_runtime_log` | excluded | — | — | SelfLogging |",
  );
});

describe.each(["build-ipc-dict.mjs", "build-schema-table.mjs"])(
  "%s CLI",
  (scriptName) => {
    it("resolves repository paths independently of the current working directory", () => {
      const outsideDirectory = mkdtempSync(
        join(tmpdir(), "skillport-docs-cli-"),
      );

      try {
        const result = spawnSync(
          process.execPath,
          [join(repositoryRoot, "scripts", "docs", scriptName), "--check"],
          { cwd: outsideDirectory, encoding: "utf8" },
        );

        expect(result.stderr).toBe("");
        expect(result.status).toBe(0);
      } finally {
        rmSync(outsideDirectory, { recursive: true, force: true });
      }
    });
  },
);

function rustSqlModule(statements: string[]): string {
  return statements.map((sql) => `sqlx::query("${sql}");`).join("\n") + "\n";
}

function generateSchemaMarkdown(sourceText: string, fileName = "example.rs"): string {
  const rootDir = mkdtempSync(join(tmpdir(), "skillport-schema-parse-"));
  const sourceDir = join(rootDir, "source");
  const outputFile = join(rootDir, "generated", "data-model.md");
  try {
    mkdirSync(sourceDir, { recursive: true });
    writeFileSync(join(sourceDir, fileName), sourceText, "utf8");
    return schemaModule.generateSchemaDocs({
      sourceDir,
      outputFile,
      rootDir,
      log: () => undefined,
    });
  } finally {
    rmSync(rootDir, { recursive: true, force: true });
  }
}

describe("schema table parser", () => {
  it("keeps ALTER ADD COLUMN once using the ALTER definition", () => {
    const markdown = generateSchemaMarkdown(
      rustSqlModule([
        `CREATE TABLE IF NOT EXISTS skill_tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL
        )`,
        "ALTER TABLE skill_tags ADD COLUMN group_id TEXT NOT NULL DEFAULT 'ungrouped'",
      ]),
    );

    expect(markdown.match(/\| `group_id` \|/g)).toEqual(["| `group_id` |"]);
    expect(markdown).toContain(
      "| `group_id` | `TEXT` | no | `'ungrouped'` | — |",
    );
  });

  it("omits indexes after DROP INDEX", () => {
    const markdown = generateSchemaMarkdown(
      rustSqlModule([
        "CREATE TABLE IF NOT EXISTS items (id TEXT PRIMARY KEY)",
        "CREATE INDEX IF NOT EXISTS idx_items_id ON items(id)",
        "DROP INDEX IF EXISTS idx_items_id",
      ]),
    );

    expect(markdown).not.toContain("idx_items_id");
    expect(markdown).not.toContain("Indexes:");
  });

  it("keeps only the rebuilt index definition after DROP then CREATE", () => {
    const markdown = generateSchemaMarkdown(
      rustSqlModule([
        `CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            name TEXT
        )`,
        "CREATE INDEX IF NOT EXISTS idx_items ON items(id)",
        "DROP INDEX IF EXISTS idx_items",
        "CREATE INDEX IF NOT EXISTS idx_items ON items(id, name)",
      ]),
    );

    expect(markdown).toContain("- `idx_items` on `(id, name)` non-unique");
    expect(markdown).not.toContain("- `idx_items` on `(id)` non-unique");
    expect(markdown.match(/`idx_items`/g)).toEqual(["`idx_items`"]);
  });

  it("labels unique and non-unique indexes", () => {
    const markdown = generateSchemaMarkdown(
      rustSqlModule([
        `CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            uid TEXT,
            is_central BOOLEAN NOT NULL DEFAULT 0
        )`,
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_skills_uid ON skills(uid)",
        "CREATE INDEX IF NOT EXISTS idx_skills_is_central ON skills(is_central)",
      ]),
    );

    expect(markdown).toContain("- `idx_skills_uid` on `(uid)` unique");
    expect(markdown).toContain(
      "- `idx_skills_is_central` on `(is_central)` non-unique",
    );
  });

  it("fails closed on unsupported schema DDL with source path and category", () => {
    const rootDir = mkdtempSync(join(tmpdir(), "skillport-schema-unknown-"));
    const sourceDir = join(rootDir, "source");
    const outputFile = join(rootDir, "generated", "data-model.md");

    try {
      mkdirSync(sourceDir, { recursive: true });
      writeFileSync(
        join(sourceDir, "legacy.rs"),
        rustSqlModule([
          "CREATE TABLE IF NOT EXISTS examples (id TEXT PRIMARY KEY)",
          "ALTER TABLE examples RENAME TO renamed",
        ]),
        "utf8",
      );

      expect(() =>
        schemaModule.generateSchemaDocs({
          sourceDir,
          outputFile,
          rootDir,
          log: () => undefined,
        }),
      ).toThrow(/unsupported ALTER TABLE in source\/legacy\.rs:\d/);
    } finally {
      rmSync(rootDir, { recursive: true, force: true });
    }
  });

  it("ignores ordinary DML backfill statements", () => {
    const markdown = generateSchemaMarkdown(
      rustSqlModule([
        `CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            uid TEXT
        )`,
        "UPDATE skills SET uid = 'backfill' WHERE uid IS NULL",
        "INSERT INTO skills (id) VALUES ('seed')",
      ]),
    );

    expect(markdown).toContain("### `skills`");
    expect(markdown).toContain("| `uid` | `TEXT` | yes | — | — |");
  });

  it("documents repository unique uid index and known incremental columns", () => {
    const sourceDir = join(
      repositoryRoot,
      "src-tauri",
      "src",
      "db",
      "schema",
    );
    const rootDir = mkdtempSync(join(tmpdir(), "skillport-schema-repo-"));
    const outputFile = join(rootDir, "generated", "data-model.md");

    try {
      const markdown = schemaModule.generateSchemaDocs({
        sourceDir,
        outputFile,
        rootDir: repositoryRoot,
        log: () => undefined,
      });

      expect(markdown).toContain("- `idx_skills_uid` on `(uid)` unique");
      expect(markdown).toContain(
        "- `idx_skills_is_central` on `(is_central)` non-unique",
      );
      expect(markdown).toContain("| `group_id` | `TEXT` | yes | — | — |");
      expect(markdown).toContain("| `proposed_name` | `TEXT` | yes | — | — |");
      expect(markdown).toContain(
        "| `proposed_description` | `TEXT` | yes | — | — |",
      );
      expect(markdown).toContain("| `last_synced_at` | `TEXT` | yes | — | — |");
    } finally {
      rmSync(rootDir, { recursive: true, force: true });
    }
  });
});
