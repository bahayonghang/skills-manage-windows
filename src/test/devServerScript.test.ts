import { describe, expect, it } from "vitest";

type ProcessInfo = {
  pid: number;
  localAddress?: string;
  processName?: string;
  executablePath?: string;
  commandLine?: string;
};

type DevServerHelpers = {
  formatProcessInfo: (processInfo: ProcessInfo) => string;
  isRepoOwnedDevProcess: (processInfo: ProcessInfo, projectRoot?: string) => boolean;
  isSkillPortIndexHtml: (html: string) => boolean;
  normalizeForProcessMatch: (value: string) => string;
  parseProcessListJson: (rawJson: string) => ProcessInfo[];
};

// @ts-expect-error The dev server wrapper is an ESM Node script outside the TS source tree.
const helpers = (await import("../../scripts/dev-server.mjs")) as DevServerHelpers;

const {
  formatProcessInfo,
  isRepoOwnedDevProcess,
  isSkillPortIndexHtml,
  normalizeForProcessMatch,
  parseProcessListJson,
} = helpers;

describe("dev-server script helpers", () => {
  it("normalizes Windows paths for process matching", () => {
    expect(
      normalizeForProcessMatch("D:\\Documents\\Code\\Agents\\skills-manage-windows")
    ).toBe("d:/documents/code/agents/skills-manage-windows");
  });

  it("parses PowerShell JSON for one or many port owners", () => {
    expect(parseProcessListJson("")).toEqual([]);
    expect(parseProcessListJson('{"pid":42,"processName":"node.exe"}')).toEqual([
      {
        pid: 42,
        localAddress: "",
        processName: "node.exe",
        executablePath: "",
        commandLine: "",
      },
    ]);
    expect(parseProcessListJson('[{"pid":42},{"pid":43,"commandLine":"vite"}]')).toHaveLength(2);
  });

  it("only treats same-repository dev processes as safe to stop", () => {
    const repoRoot = "D:/Documents/Code/Agents/skills-manage-windows";

    expect(
      isRepoOwnedDevProcess(
        {
          pid: 42,
          processName: "node.exe",
          executablePath: "C:/Program Files/nodejs/node.exe",
          commandLine:
            "node D:/Documents/Code/Agents/skills-manage-windows/node_modules/vite/bin/vite.js",
        },
        repoRoot
      )
    ).toBe(true);

    expect(
      isRepoOwnedDevProcess(
        {
          pid: 43,
          processName: "node.exe",
          executablePath: "C:/Program Files/nodejs/node.exe",
          commandLine: "node D:/Other/project/node_modules/vite/bin/vite.js",
        },
        repoRoot
      )
    ).toBe(false);
  });

  it("recognizes this app's Vite index before reusing an existing server", () => {
    expect(
      isSkillPortIndexHtml(
        '<title>SkillPort</title><script type="module" src="/src/main.tsx"></script>'
      )
    ).toBe(true);
    expect(
      isSkillPortIndexHtml(
        '<title>SkillPort</title><script type="module" src="/src/main.tsx?t=123"></script>'
      )
    ).toBe(true);
    expect(isSkillPortIndexHtml('<title>Other App</title><script src="/src/main.tsx"></script>')).toBe(
      false
    );
  });

  it("formats process details for actionable errors", () => {
    expect(
      formatProcessInfo({
        pid: 42,
        processName: "node.exe",
        executablePath: "C:/Program Files/nodejs/node.exe",
        commandLine: "node vite.js",
      })
    ).toBe("pid=42 name=node.exe path=C:/Program Files/nodejs/node.exe cmd=node vite.js");
  });
});
