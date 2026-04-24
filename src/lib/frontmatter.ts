export type FrontmatterValue =
  | string
  | number
  | boolean
  | null
  | FrontmatterValue[]
  | { [key: string]: FrontmatterValue };

export interface ParsedFrontmatter {
  frontmatterRaw: string;
  frontmatterData: Record<string, FrontmatterValue>;
  body: string;
}

const LEADING_FRONTMATTER_RE =
  /^(?:\n+)?---[ \t]*\n([\s\S]*?)\n---[ \t]*(?:\n|$)([\s\S]*)$/;

function normalizeFrontmatterInput(markdown: string): string {
  return markdown
    .replace(/^\uFEFF/, "")
    .replace(/\r\n?/g, "\n")
    .replace(/^(?:\u200B|\u200C|\u200D|\u2060)+/, "");
}

function extractLeadingFrontmatter(markdown: string) {
  const match = markdown.match(LEADING_FRONTMATTER_RE);
  if (!match) {
    return null;
  }

  return {
    frontmatterRaw: match[1],
    body: match[2].trimStart(),
  };
}

function unquoteFrontmatterValue(value: string) {
  const trimmed = value.trim();
  if (trimmed.length >= 2) {
    const quote = trimmed[0];
    if ((quote === `"` || quote === `'`) && trimmed[trimmed.length - 1] === quote) {
      return trimmed.slice(1, -1).trim();
    }
  }
  return trimmed;
}

function foldBlockScalarLines(lines: string[], style: ">" | "|") {
  if (style === "|") {
    return lines.join("\n").trim();
  }
  return lines
    .map((line) => line.trim())
    .filter(Boolean)
    .join(" ")
    .trim();
}

function getIndent(line: string) {
  let indent = 0;
  for (const char of line) {
    if (char === " ") {
      indent += 1;
      continue;
    }
    if (char === "\t") {
      indent += 2;
      continue;
    }
    break;
  }
  return indent;
}

function stripIndent(line: string, indent: number) {
  let remaining = indent;
  let index = 0;

  while (remaining > 0 && index < line.length) {
    if (line[index] === " ") {
      remaining -= 1;
      index += 1;
      continue;
    }
    if (line[index] === "\t") {
      remaining -= 2;
      index += 1;
      continue;
    }
    break;
  }

  return line.slice(index);
}

function normalizeParsedValue(value: FrontmatterValue): FrontmatterValue {
  if (Array.isArray(value)) {
    return value.map(normalizeParsedValue);
  }

  if (typeof value === "object" && value !== null) {
    const entries = Object.entries(value)
      .map(([key, entryValue]) => [key, normalizeParsedValue(entryValue)] as const)
      .filter(([, entryValue]) => {
        if (typeof entryValue === "string") {
          return entryValue.trim().length > 0;
        }
        return true;
      });
    return Object.fromEntries(entries);
  }

  return value;
}

function extractDescriptionFallback(
  data: Record<string, FrontmatterValue>,
): Record<string, FrontmatterValue> {
  if (typeof data.description === "string" && data.description.trim()) {
    return data;
  }

  const localizedDescription =
    (typeof data.description_zh === "string" && data.description_zh.trim())
    || (typeof data.description_en === "string" && data.description_en.trim())
    || null;

  if (!localizedDescription) {
    return data;
  }

  return {
    ...data,
    description: localizedDescription,
  };
}

function parseScalarValue(rawValue: string): FrontmatterValue {
  const trimmed = rawValue.trim();
  if (!trimmed) return "";

  if (trimmed === "null" || trimmed === "~") {
    return null;
  }
  if (trimmed === "true") {
    return true;
  }
  if (trimmed === "false") {
    return false;
  }

  return unquoteFrontmatterValue(trimmed);
}

function parseBlockScalar(
  lines: string[],
  startIndex: number,
  parentIndent: number,
  style: ">" | "|",
) {
  const blockLines: string[] = [];
  let cursor = startIndex + 1;
  let contentIndent: number | null = null;

  while (cursor < lines.length) {
    const nextLine = lines[cursor];
    if (!nextLine.trim()) {
      blockLines.push("");
      cursor += 1;
      continue;
    }

    const nextIndent = getIndent(nextLine);
    if (nextIndent <= parentIndent) {
      break;
    }

    if (contentIndent === null) {
      contentIndent = nextIndent;
    }

    blockLines.push(stripIndent(nextLine, contentIndent));
    cursor += 1;
  }

  return {
    value: foldBlockScalarLines(blockLines, style),
    nextIndex: cursor,
  };
}

function parseNestedBlock(
  lines: string[],
  startIndex: number,
  indent: number,
): { value: FrontmatterValue; nextIndex: number } {
  const nextLine = lines[startIndex];
  if (!nextLine) {
    return { value: "", nextIndex: startIndex };
  }

  const trimmed = stripIndent(nextLine, indent);
  if (trimmed.startsWith("- ")) {
    return parseArrayBlock(lines, startIndex, indent);
  }

  return parseObjectBlock(lines, startIndex, indent);
}

function parseArrayBlock(
  lines: string[],
  startIndex: number,
  indent: number,
): { value: FrontmatterValue[]; nextIndex: number } {
  const items: FrontmatterValue[] = [];
  let index = startIndex;

  while (index < lines.length) {
    const line = lines[index];
    if (!line.trim()) {
      index += 1;
      continue;
    }

    const lineIndent = getIndent(line);
    if (lineIndent < indent) {
      break;
    }
    if (lineIndent > indent) {
      break;
    }

    const trimmed = stripIndent(line, indent);
    if (!trimmed.startsWith("- ")) {
      break;
    }

    const itemText = trimmed.slice(2).trim();
    if (!itemText) {
      const nextIndex = index + 1;
      if (nextIndex >= lines.length || getIndent(lines[nextIndex]) <= indent) {
        items.push("");
        index += 1;
        continue;
      }

      const nested = parseNestedBlock(lines, nextIndex, getIndent(lines[nextIndex]));
      items.push(nested.value);
      index = nested.nextIndex;
      continue;
    }

    const keyMatch = itemText.match(/^([A-Za-z0-9_-]+):(?:\s*(.*))?$/);
    if (keyMatch) {
      const [, key, rawValue = ""] = keyMatch;
      const objectValue: Record<string, FrontmatterValue> = {};

      if (/^[>|][+-]?$/.test(rawValue.trim())) {
        const block = parseBlockScalar(lines, index, indent, rawValue.trim()[0] as ">" | "|");
        objectValue[key] = block.value;
        index = block.nextIndex;
      } else if (!rawValue.trim()) {
        const nextIndex = index + 1;
        if (nextIndex < lines.length && getIndent(lines[nextIndex]) > indent) {
          const nested = parseNestedBlock(lines, nextIndex, getIndent(lines[nextIndex]));
          objectValue[key] = nested.value;
          index = nested.nextIndex;
        } else {
          objectValue[key] = "";
          index += 1;
        }
      } else {
        objectValue[key] = parseScalarValue(rawValue);
        index += 1;
      }

      if (index < lines.length && lines[index].trim() && getIndent(lines[index]) > indent) {
        const nestedObject = parseObjectBlock(lines, index, getIndent(lines[index]));
        items.push({
          ...objectValue,
          ...(nestedObject.value as Record<string, FrontmatterValue>),
        });
        index = nestedObject.nextIndex;
      } else {
        items.push(objectValue);
      }
      continue;
    }

    items.push(parseScalarValue(itemText));
    index += 1;
  }

  return { value: items, nextIndex: index };
}

function parseObjectBlock(
  lines: string[],
  startIndex: number,
  indent: number,
): { value: Record<string, FrontmatterValue>; nextIndex: number } {
  const data: Record<string, FrontmatterValue> = {};
  let index = startIndex;

  while (index < lines.length) {
    const line = lines[index];
    if (!line.trim()) {
      index += 1;
      continue;
    }

    const lineIndent = getIndent(line);
    if (lineIndent < indent) {
      break;
    }
    if (lineIndent > indent) {
      break;
    }

    const trimmed = stripIndent(line, indent);
    if (trimmed.startsWith("- ")) {
      break;
    }

    const match = trimmed.match(/^([A-Za-z0-9_-]+):(?:\s*(.*))?$/);
    if (!match) {
      index += 1;
      continue;
    }

    const [, key, rawValue = ""] = match;
    const normalizedValue = rawValue.trim();

    if (/^[>|][+-]?$/.test(normalizedValue)) {
      const block = parseBlockScalar(lines, index, indent, normalizedValue[0] as ">" | "|");
      data[key] = block.value;
      index = block.nextIndex;
      continue;
    }

    if (!normalizedValue) {
      const nextIndex = index + 1;
      if (nextIndex < lines.length && getIndent(lines[nextIndex]) > indent) {
        const nested = parseNestedBlock(lines, nextIndex, getIndent(lines[nextIndex]));
        data[key] = nested.value;
        index = nested.nextIndex;
      } else {
        data[key] = "";
        index += 1;
      }
      continue;
    }

    data[key] = parseScalarValue(normalizedValue);
    index += 1;
  }

  return { value: data, nextIndex: index };
}

function parseStructuredFrontmatter(frontmatterRaw: string): Record<string, FrontmatterValue> {
  const lines = frontmatterRaw.split("\n");
  const parsed = parseObjectBlock(lines, 0, 0).value;
  return extractDescriptionFallback(normalizeParsedValue(parsed) as Record<string, FrontmatterValue>);
}

export function parseFrontmatter(markdown: string): ParsedFrontmatter {
  if (!markdown.trim()) {
    return {
      frontmatterRaw: "",
      frontmatterData: {},
      body: markdown,
    };
  }

  const normalizedMarkdown = normalizeFrontmatterInput(markdown);
  const extracted = extractLeadingFrontmatter(normalizedMarkdown);

  if (!extracted) {
    return {
      frontmatterRaw: "",
      frontmatterData: {},
      body: normalizedMarkdown,
    };
  }

  return {
    frontmatterRaw: extracted.frontmatterRaw,
    frontmatterData: parseStructuredFrontmatter(extracted.frontmatterRaw),
    body: extracted.body,
  };
}
