/**
 * Central Skills 视图状态序列化（URL-as-state）。
 *
 * 把搜索 / 筛选 / 排序 / 视图模式 / 分组方式都编码到 URLSearchParams，
 * 供 URL 分享、刷新保留上下文、Saved View 恢复使用。
 *
 * D2：不引入第三方依赖，全部纯函数 + URLSearchParams。
 */

import type {
  CentralSortDirection,
  CentralSortField,
} from "@/pages/centralSkillsViewModel";

// 注：这里直接复用 view-model 中已有的排序类型字面量。
// 为避免循环依赖，在 ts-only 类型层面引用。

export type ViewMode = "grid" | "list";

export type ViewDensity = "comfortable" | "compact";

export type GroupByMode = "none" | "repository" | "owner" | "tag" | "status";

export interface CentralViewState {
  /** 搜索框原文（含结构化语法） */
  q: string;
  /** 单选仓库 id（数组形状用于兼容旧 URL / Saved View，保留特殊值 "unassigned"） */
  repos: string[];
  /** 多选 tag id（保留特殊值 "uncategorized"） */
  tags: string[];
  /** 视图模式：grid 双列卡片 / list 单列列表 */
  view: ViewMode;
  /** 视图密度：comfortable 宽松 / compact 紧凑 */
  density: ViewDensity;
  /** 分组方式 */
  group: GroupByMode;
  /** 排序字段 */
  sortField: CentralSortField;
  /** 排序方向 */
  sortDir: CentralSortDirection;
  /** 当前应用的 Saved View id（可选） */
  savedView?: string;
}

const DEFAULT_STATE: CentralViewState = {
  q: "",
  repos: [],
  tags: [],
  view: "grid",
  density: "comfortable",
  group: "none",
  sortField: "name",
  sortDir: "asc",
};

const VIEW_MODES: ReadonlySet<ViewMode> = new Set(["grid", "list"]);
const VIEW_DENSITIES: ReadonlySet<ViewDensity> = new Set([
  "comfortable",
  "compact",
]);
const GROUP_BY_MODES: ReadonlySet<GroupByMode> = new Set([
  "none",
  "repository",
  "owner",
  "tag",
  "status",
]);
const SORT_FIELDS: ReadonlySet<CentralSortField> = new Set([
  "name",
  "createdAt",
  "updatedAt",
  "installedPlatformCount",
]);
const SORT_DIRECTIONS: ReadonlySet<CentralSortDirection> = new Set([
  "asc",
  "desc",
]);

/** 返回视图状态的合理默认值。每次都是新引用，安全用作 useState 初值。 */
export function defaultCentralViewState(): CentralViewState {
  return { ...DEFAULT_STATE, repos: [], tags: [] };
}

function pickEnum<T extends string>(
  raw: string | null,
  allowed: ReadonlySet<T>,
  fallback: T,
): T {
  if (raw && (allowed as ReadonlySet<string>).has(raw)) return raw as T;
  return fallback;
}

function splitCsv(raw: string | null): string[] {
  if (!raw) return [];
  return raw
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
}

function joinCsv(values: readonly string[]): string {
  return values
    .map((v) => v.trim())
    .filter((v) => v.length > 0)
    .join(",");
}

export function normalizeSingleRepoSelection(
  values: readonly string[],
): string[] {
  const first = values
    .map((value) => value.trim())
    .find((value) => value.length > 0);
  return first ? [first] : [];
}

export function normalizeCentralViewState(
  state: CentralViewState,
): CentralViewState {
  return {
    ...state,
    repos: normalizeSingleRepoSelection(state.repos),
    tags: state.tags.map((tag) => tag.trim()).filter((tag) => tag.length > 0),
  };
}

/**
 * 把 URLSearchParams 解析为 CentralViewState。
 * 任何字段缺失 / 非法都回退到默认值，不抛错。
 */
export function parseCentralViewState(
  params: URLSearchParams,
): CentralViewState {
  const sortRaw = params.get("sort");
  let sortField: CentralSortField = DEFAULT_STATE.sortField;
  let sortDir: CentralSortDirection = DEFAULT_STATE.sortDir;
  if (sortRaw) {
    const [fieldPart, dirPart] = sortRaw.split(":");
    sortField = pickEnum(
      fieldPart ?? null,
      SORT_FIELDS,
      DEFAULT_STATE.sortField,
    );
    sortDir = pickEnum(dirPart ?? null, SORT_DIRECTIONS, DEFAULT_STATE.sortDir);
  }

  const savedView = params.get("savedView")?.trim();

  return {
    q: params.get("q") ?? "",
    repos: normalizeSingleRepoSelection(splitCsv(params.get("repos"))),
    tags: splitCsv(params.get("tags")),
    view: pickEnum(params.get("view"), VIEW_MODES, DEFAULT_STATE.view),
    density: pickEnum(
      params.get("density"),
      VIEW_DENSITIES,
      DEFAULT_STATE.density,
    ),
    group: pickEnum(params.get("group"), GROUP_BY_MODES, DEFAULT_STATE.group),
    sortField,
    sortDir,
    ...(savedView ? { savedView } : {}),
  };
}

/**
 * 把 CentralViewState 序列化为 URLSearchParams。
 * 仅写入与默认值不同的字段，避免 URL 噪音。
 */
export function serializeCentralViewState(
  state: CentralViewState,
): URLSearchParams {
  const normalized = normalizeCentralViewState(state);
  const params = new URLSearchParams();
  if (normalized.q) params.set("q", normalized.q);
  if (normalized.repos.length > 0)
    params.set("repos", joinCsv(normalized.repos));
  if (normalized.tags.length > 0) params.set("tags", joinCsv(normalized.tags));
  if (normalized.view !== DEFAULT_STATE.view)
    params.set("view", normalized.view);
  if (normalized.density !== DEFAULT_STATE.density)
    params.set("density", normalized.density);
  if (normalized.group !== DEFAULT_STATE.group)
    params.set("group", normalized.group);
  if (
    normalized.sortField !== DEFAULT_STATE.sortField ||
    normalized.sortDir !== DEFAULT_STATE.sortDir
  ) {
    params.set("sort", `${normalized.sortField}:${normalized.sortDir}`);
  }
  if (normalized.savedView) params.set("savedView", normalized.savedView);
  return params;
}

/** 便捷：直接对 URL string 编码与解码。 */
export function parseCentralViewStateFromUrl(url: string): CentralViewState {
  try {
    const u = new URL(url, "http://localhost");
    return parseCentralViewState(u.searchParams);
  } catch {
    return defaultCentralViewState();
  }
}

export function toQueryString(state: CentralViewState): string {
  const params = serializeCentralViewState(state);
  const str = params.toString();
  return str ? `?${str}` : "";
}

/**
 * 返回新 state，仅替换给定字段。空数组也算"主动清空"（语义清晰）。
 */
export function updateCentralViewState(
  state: CentralViewState,
  patch: Partial<CentralViewState>,
): CentralViewState {
  return normalizeCentralViewState({ ...state, ...patch });
}
