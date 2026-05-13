/**
 * Central Skills 结构化搜索语法。
 *
 * 输入：用户在搜索框中输入的字符串
 * 输出：可由 matcher 消费的 AST
 *
 * 支持的关键字（关键字大小写不敏感，值保留原样）：
 *
 *   tag:<name>          包含某个 tag（按 tag.name 匹配，大小写不敏感）
 *   -tag:<name>         排除某个 tag
 *   repo:<owner/repo>   仓库筛选；支持 `*` 通配（owner/* 或 *
 *                       /repo 或 *）；也接受 repo.name 直接匹配
 *   owner:<name>        所有者筛选
 *   source:<github|local|manual>
 *                       仓库来源类型
 *   has:<update|no-tag|ai-review|uncategorized>
 *                       状态筛选
 *   platform:<agent_id> 已链接到指定 agent
 *   created:<op><value> 创建时间，op 取 <, >, <=, >=, =；value 支持
 *                       7d / 2026-01-01 / 2026 等粗粒度
 *   updated:<op><value> 修改时间，同上
 *
 *   "<phrase>"          带引号的多词短语，按整体当作自由词或值
 *   <free text>         其它内容串成自由词，由 matcher 走原有的全文匹配
 *
 * 例：
 *   tag:编程 repo:anthropics/* has:update -tag:wip
 *
 * 解析失败的 token 会落入 `invalid` 数组，由调用方决定如何提示。
 */

import type {
  CentralQueryAst,
  CentralQueryFilter,
  CentralSkillUpdateState,
  SkillTag,
  SkillWithLinks,
} from "@/types";

// AST 类型在 `src/types/index.ts` 中定义为契约。这里以局部别名复用，
// 同时导出 helper 类型，便于模块外按枚举值约束。
export type { CentralQueryAst, CentralQueryFilter };

export type SourceValue = Extract<CentralQueryFilter, { kind: "source" }>["value"];
export type HasValue = Extract<CentralQueryFilter, { kind: "has" }>["value"];
export type TimeOp = Extract<CentralQueryFilter, { kind: "created" | "updated" }>["op"];

const SOURCE_VALUES: ReadonlySet<SourceValue> = new Set(["github", "local", "manual"]);
const HAS_VALUES: ReadonlySet<HasValue> = new Set([
  "update",
  "no-tag",
  "ai-review",
  "uncategorized",
]);

const KNOWN_KEYS = new Set([
  "tag",
  "repo",
  "owner",
  "source",
  "has",
  "platform",
  "created",
  "updated",
]);

// ─── Tokenizer ────────────────────────────────────────────────────────────

/**
 * 把输入串切成 token：以空格分隔，但保留引号包裹的整段（含内部空格）。
 *
 * - 双引号支持反斜杠转义
 * - 未闭合的引号回退为字面量处理（容错）
 */
export function tokenizeCentralQuery(input: string): string[] {
  const tokens: string[] = [];
  let buf = "";
  let inQuote = false;
  let i = 0;
  const n = input.length;

  while (i < n) {
    const ch = input[i];
    if (inQuote) {
      if (ch === "\\" && i + 1 < n) {
        buf += input[i + 1];
        i += 2;
        continue;
      }
      if (ch === '"') {
        inQuote = false;
        i += 1;
        continue;
      }
      buf += ch;
      i += 1;
      continue;
    }
    if (ch === '"') {
      inQuote = true;
      i += 1;
      continue;
    }
    if (/\s/.test(ch)) {
      if (buf.length > 0) {
        tokens.push(buf);
        buf = "";
      }
      i += 1;
      continue;
    }
    buf += ch;
    i += 1;
  }
  if (buf.length > 0) tokens.push(buf);
  return tokens;
}

// ─── Parser ───────────────────────────────────────────────────────────────

const TIME_OP_RE = /^(<=|>=|<|>|=)/;

interface ParseResult {
  filter?: CentralQueryFilter;
  freeText?: string;
  invalid?: string;
}

function parseTimeFilter(
  kind: "created" | "updated",
  rawValue: string,
  negated: boolean
): CentralQueryFilter | null {
  const opMatch = TIME_OP_RE.exec(rawValue);
  const op: TimeOp = (opMatch?.[1] as TimeOp | undefined) ?? "=";
  const value = opMatch ? rawValue.slice(opMatch[0].length).trim() : rawValue.trim();
  if (value.length === 0) return null;
  return { kind, op, value, negated };
}

function parseToken(token: string): ParseResult {
  if (token.length === 0) return {};

  // 处理负号：仅当 - 后紧跟 key: 形式才算负号
  let negated = false;
  let body = token;
  if (token.startsWith("-")) {
    const rest = token.slice(1);
    if (rest.includes(":")) {
      negated = true;
      body = rest;
    }
  }

  const colonIdx = body.indexOf(":");
  if (colonIdx < 0) {
    // 不含冒号的 token 一律当作自由词
    return { freeText: token };
  }

  const rawKey = body.slice(0, colonIdx);
  const rawValue = body.slice(colonIdx + 1);
  const key = rawKey.toLowerCase(); // D1: 关键字大小写不敏感

  if (!KNOWN_KEYS.has(key)) {
    // 未知关键字：作为自由词处理（保留原 token，避免静默丢失）
    return { freeText: token };
  }

  const value = rawValue.trim();
  if (value.length === 0) {
    return { invalid: token };
  }

  switch (key) {
    case "tag":
      return { filter: { kind: "tag", value, negated } };
    case "repo":
      return { filter: { kind: "repo", value, negated } };
    case "owner":
      return { filter: { kind: "owner", value, negated } };
    case "platform":
      return { filter: { kind: "platform", value, negated } };
    case "source": {
      const lc = value.toLowerCase() as SourceValue;
      if (!SOURCE_VALUES.has(lc)) return { invalid: token };
      return { filter: { kind: "source", value: lc, negated } };
    }
    case "has": {
      const lc = value.toLowerCase() as HasValue;
      if (!HAS_VALUES.has(lc)) return { invalid: token };
      return { filter: { kind: "has", value: lc, negated } };
    }
    case "created":
    case "updated": {
      const filter = parseTimeFilter(key, value, negated);
      if (!filter) return { invalid: token };
      return { filter };
    }
    default:
      return { invalid: token };
  }
}

/** 把输入字符串解析为 AST。空输入返回空 AST。 */
export function parseCentralQuery(input: string): CentralQueryAst {
  const ast: CentralQueryAst = { freeText: "", filters: [], invalid: [] };
  const tokens = tokenizeCentralQuery(input);
  const freeWords: string[] = [];

  for (const token of tokens) {
    const result = parseToken(token);
    if (result.filter) ast.filters.push(result.filter);
    if (result.freeText) freeWords.push(result.freeText);
    if (result.invalid) ast.invalid.push(result.invalid);
  }

  ast.freeText = freeWords.join(" ").trim().toLowerCase();
  return ast;
}

// ─── Matcher ──────────────────────────────────────────────────────────────

export interface CentralQueryContext {
  /** 当前 skill_id → update 状态 */
  updateStatuses: Record<string, CentralSkillUpdateState>;
  /** 处于 AI review 队列的 skill_id 集合 */
  aiReviewSkillIds: ReadonlySet<string>;
  /** 全量 tag 索引：tag.id → tag。用于按 id 快速找 name */
  tagsById?: ReadonlyMap<string, SkillTag>;
}

function caseInsensitiveIncludes(haystack: string, needle: string): boolean {
  if (!needle) return true;
  return haystack.toLowerCase().includes(needle.toLowerCase());
}

function caseInsensitiveEquals(a: string, b: string): boolean {
  return a.toLowerCase() === b.toLowerCase();
}

/**
 * 简单 glob 匹配：仅支持 `*` 通配。
 * 用于 repo:anthropics/* 或 *
 * /skills 之类的写法。
 */
export function matchesGlob(value: string, pattern: string): boolean {
  if (!pattern) return true;
  if (!pattern.includes("*")) {
    return caseInsensitiveEquals(value, pattern);
  }
  const escaped = pattern
    .toLowerCase()
    .replace(/[.+?^${}()|[\]\\]/g, "\\$&")
    .replace(/\*/g, ".*");
  const re = new RegExp(`^${escaped}$`);
  return re.test(value.toLowerCase());
}

function getRepoFullName(skill: SkillWithLinks): string | null {
  const repo = skill.repository;
  if (!repo) return null;
  if (repo.owner && repo.repo) return `${repo.owner}/${repo.repo}`;
  return repo.name ?? null;
}

function isUpdateAvailable(
  skill: SkillWithLinks,
  ctx: CentralQueryContext
): boolean {
  return ctx.updateStatuses[skill.id]?.status === "update_available";
}

function hasNoTag(skill: SkillWithLinks): boolean {
  const tags = skill.tags ?? [];
  if (tags.length === 0) return true;
  return tags.every((tag) => tag.id === "uncategorized");
}

function matchesFilter(
  skill: SkillWithLinks,
  filter: CentralQueryFilter,
  ctx: CentralQueryContext
): boolean {
  switch (filter.kind) {
    case "tag": {
      const tags = skill.tags ?? [];
      return tags.some(
        (tag) =>
          caseInsensitiveEquals(tag.name, filter.value)
          || caseInsensitiveIncludes(tag.name, filter.value)
          || caseInsensitiveEquals(tag.id, filter.value)
      );
    }
    case "repo": {
      const fullName = getRepoFullName(skill);
      if (!fullName) return false;
      return (
        matchesGlob(fullName, filter.value)
        || caseInsensitiveEquals(skill.repository?.id ?? "", filter.value)
        || caseInsensitiveIncludes(skill.repository?.name ?? "", filter.value)
      );
    }
    case "owner": {
      const owner = skill.repository?.owner ?? "";
      return caseInsensitiveEquals(owner, filter.value);
    }
    case "source": {
      const source = (skill.repository?.source_type ?? "").toLowerCase();
      // local 同时覆盖 is_unknown / 没有 repository 的本地散件
      if (filter.value === "local") {
        return source === "local" || skill.is_source_unknown === true || skill.repository === undefined;
      }
      return source === filter.value;
    }
    case "has": {
      switch (filter.value) {
        case "update":
          return isUpdateAvailable(skill, ctx);
        case "no-tag":
        case "uncategorized":
          return hasNoTag(skill);
        case "ai-review":
          return ctx.aiReviewSkillIds.has(skill.id);
        default:
          return false;
      }
    }
    case "platform": {
      const linked = skill.linked_agents ?? [];
      const shared = skill.shared_root_agents ?? [];
      return [...linked, ...shared].some((id) => caseInsensitiveEquals(id, filter.value));
    }
    case "created":
    case "updated":
      return matchesTimeFilter(skill, filter, ctx);
    default:
      return false;
  }
}

function parseRelativeDuration(value: string): number | null {
  const match = /^(\d+)\s*([dhm])$/i.exec(value.trim());
  if (!match) return null;
  const amount = Number(match[1]);
  const unit = match[2].toLowerCase();
  const ms = unit === "d" ? 86_400_000 : unit === "h" ? 3_600_000 : 60_000;
  return amount * ms;
}

function parseAbsoluteDate(value: string): number | null {
  // 支持 YYYY、YYYY-MM、YYYY-MM-DD 与 ISO 8601 字符串
  const t = Date.parse(value);
  if (Number.isFinite(t)) return t;
  return null;
}

function getSkillTimestamp(
  skill: SkillWithLinks,
  field: "created" | "updated"
): number | null {
  const raw =
    field === "created"
      ? skill.created_at ?? skill.scanned_at
      : skill.updated_at ?? skill.scanned_at;
  if (!raw) return null;
  const t = Date.parse(raw);
  return Number.isFinite(t) ? t : null;
}

function matchesTimeFilter(
  skill: SkillWithLinks,
  filter: Extract<CentralQueryFilter, { kind: "created" | "updated" }>,
  _ctx: CentralQueryContext
): boolean {
  const skillTime = getSkillTimestamp(skill, filter.kind);
  if (skillTime === null) return false;

  // 相对时间（如 7d）：表示"距今 N 单位以内"
  const relativeMs = parseRelativeDuration(filter.value);
  if (relativeMs !== null) {
    const now = Date.now();
    const threshold = now - relativeMs;
    switch (filter.op) {
      case "<":
      case "<=":
        return skillTime >= threshold;
      case ">":
      case ">=":
        return skillTime < threshold;
      case "=":
        return skillTime >= threshold;
      default:
        return false;
    }
  }

  const target = parseAbsoluteDate(filter.value);
  if (target === null) return false;
  switch (filter.op) {
    case "<":
      return skillTime < target;
    case "<=":
      return skillTime <= target;
    case ">":
      return skillTime > target;
    case ">=":
      return skillTime >= target;
    case "=":
      // 同一天即视为相等（粒度到日）
      return Math.abs(skillTime - target) < 86_400_000;
    default:
      return false;
  }
}

/**
 * 检查 skill 是否匹配 AST。
 *
 * 自由词部分由调用方自己用现有 buildSearchText 处理；本函数仅评估 filters。
 *
 * 多个 filter 之间是 AND；带 `negated: true` 的 filter 取反后参与 AND。
 */
export function matchSkillAgainstFilters(
  skill: SkillWithLinks,
  ast: CentralQueryAst,
  ctx: CentralQueryContext
): boolean {
  for (const filter of ast.filters) {
    const ok = matchesFilter(skill, filter, ctx);
    const expected = !filter.negated;
    if (ok !== expected) return false;
  }
  return true;
}

/** 把 AST 里的自由词用空格连起来返回，方便配合 buildSearchText 使用。 */
export function getFreeText(ast: CentralQueryAst): string {
  return ast.freeText;
}
