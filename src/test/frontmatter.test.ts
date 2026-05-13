import { describe, expect, it } from "vitest";
import { parseFrontmatter } from "@/lib/frontmatter";

describe("parseFrontmatter", () => {
  it("parses common summary fields and strips the fenced block from the body", () => {
    const parsed = parseFrontmatter(
      "---\nname: baoyu-comic\ndescription: Knowledge comic creator\nversion: 1.56.1\n---\n\n# Heading\n\nBody."
    );

    expect(parsed.frontmatterData.name).toBe("baoyu-comic");
    expect(parsed.frontmatterData.description).toBe("Knowledge comic creator");
    expect(parsed.frontmatterData.version).toBe("1.56.1");
    expect(parsed.body).toBe("# Heading\n\nBody.");
  });

  it("still strips malformed frontmatter from the markdown body", () => {
    const parsed = parseFrontmatter(
      "---\nname: broken-skill\nmetadata: [oops\n---\n\n# Broken Skill\n\nBody."
    );

    expect(parsed.frontmatterData.name).toBe("broken-skill");
    expect(parsed.frontmatterRaw).toContain("name: broken-skill");
    expect(parsed.body).toBe("# Broken Skill\n\nBody.");
  });

  it("extracts block-scalar descriptions when yaml parsing falls back", () => {
    const parsed = parseFrontmatter(
      "---\nname: autoglm-search-image\ndescription: >\n  使用 AutoGLM 搜图接口。\n  Token 通过本地服务自动获取。\ncompatibility:\n  requires:\n    - Python 3.x\n---\n\n# AutoGLM\n"
    );

    expect(parsed.frontmatterData.name).toBe("autoglm-search-image");
    expect(String(parsed.frontmatterData.description).trim()).toBe(
      "使用 AutoGLM 搜图接口。 Token 通过本地服务自动获取。"
    );
  });

  it("extracts quoted summary fields even when inner quotes break yaml", () => {
    const parsed = parseFrontmatter(
      '---\nname: andonq\ndescription_zh: "AndonQ 腾讯云智能客服"小龙虾"（工单查询、智能问答、云API调用）"\nversion: 1.1.9\n---\n\n# AndonQ\n'
    );

    expect(parsed.frontmatterData.name).toBe("andonq");
    expect(parsed.frontmatterData.description).toBe(
      'AndonQ 腾讯云智能客服"小龙虾"（工单查询、智能问答、云API调用）'
    );
    expect(parsed.frontmatterData.version).toBe("1.1.9");
  });

  it("matches backend summary semantics for folded descriptions and nested display data", () => {
    const parsed = parseFrontmatter(
      "---\nname: rich-skill\ndescription: >\n  first line\n  second line\nversion: 2\nmetadata:\n  runtimes:\n    - bun\n    - npx\n  support:\n    windows: true\n    linux: false\nempty_label:\n---\n\n# Rich Skill\n"
    );

    expect(parsed.frontmatterData.name).toBe("rich-skill");
    expect(parsed.frontmatterData.description).toBe("first line second line");
    expect(parsed.frontmatterData.version).toBe("2");
    expect(parsed.frontmatterData.metadata).toEqual({
      runtimes: ["bun", "npx"],
      support: {
        windows: true,
        linux: false,
      },
    });
    expect(parsed.frontmatterData).not.toHaveProperty("empty_label");
    expect(parsed.body).toBe("# Rich Skill\n");
  });

  it("falls back to localized descriptions like backend import summaries", () => {
    const parsed = parseFrontmatter(
      "---\nname: localized-skill\ndescription_zh: 中文摘要\ndescription_en: English summary\n---\n\n# Localized Skill\n"
    );

    expect(parsed.frontmatterData.name).toBe("localized-skill");
    expect(parsed.frontmatterData.description).toBe("中文摘要");
    expect(parsed.frontmatterData.description_zh).toBe("中文摘要");
    expect(parsed.frontmatterData.description_en).toBe("English summary");
  });

  it("preserves literal block scalars and arrays of objects for the detail card", () => {
    const parsed = parseFrontmatter(
      "---\nname: tool-skill\ndescription: |\n  line one\n  line two\ntools:\n  - name: bun\n    command: bun install\n  - name: npx\n    command: npx shadcn add button\n---\n\n# Tool Skill\n"
    );

    expect(parsed.frontmatterData.description).toBe("line one\nline two");
    expect(parsed.frontmatterData.tools).toEqual([
      { name: "bun", command: "bun install" },
      { name: "npx", command: "npx shadcn add button" },
    ]);
  });

  it("supports BOM, zero-width prefixes, CRLF fences, nulls, and booleans", () => {
    const parsed = parseFrontmatter(
      "\uFEFF\u200B---\r\nname: windows-skill\r\ndescription: Windows path helper\r\noptional: null\r\nenabled: false\r\n---\r\n\r\n# Windows Skill\r\n"
    );

    expect(parsed.frontmatterData).toMatchObject({
      name: "windows-skill",
      description: "Windows path helper",
      optional: null,
      enabled: false,
    });
    expect(parsed.body).toBe("# Windows Skill\n");
  });

});
