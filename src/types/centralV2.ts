/**
 * Central Skills v2 Information Architecture (M0~M6) 的契约类型。
 *
 * 这些类型在 M0 阶段引入，给 v2 sidebar / saved view / tag group 等后续 milestone 复用。
 * 本文件仅定义纯 TS 类型，未引入运行时改动；后端尚未落地的实体（saved view / tag group）
 * 在 M2 / M3 才创建对应表与 IPC。
 *
 * 由 `@/types` (index.ts) 重新导出，保持外部 import 路径稳定。
 */

/** 列表的视觉模式：grid 或 list。 */
export type ViewMode = "grid" | "list";

/** 列表的分组方式。 */
export type GroupByMode = "none" | "repository" | "owner" | "tag" | "status";

/**
 * 用户保存的视图。`query` 字段是整段 `CentralViewState` 的 URL 序列化（无前导 `?`），
 * 含搜索框、多选筛选、排序、视图模式、分组方式。前端用
 * `serializeCentralViewState` / `parseCentralViewStateFromUrl` 双向转换。
 *
 * 后端表 `skill_saved_views` 在 M2 创建；字段命名沿用项目 snake_case 风格（与 Collection 一致）。
 */
export interface SavedView {
  id: string;
  name: string;
  /** `CentralViewState` 的 URL 序列化（不含前导 `?`）。 */
  query: string;
  sort_order: number;
  icon: string | null;
  pinned: boolean;
  created_at: string;
  updated_at: string;
}

/**
 * 标签分组。仅一级，不允许嵌套（D4）。后端表 `skill_tag_groups` 与
 * `skill_tags.group_id` 列在 M3 创建。字段命名 snake_case，与后端 contract 一致。
 */
export interface TagGroup {
  id: string;
  name: string;
  color: string | null;
  sort_order: number;
  is_builtin: boolean;
  created_at: string;
  updated_at: string;
}

/**
 * 结构化搜索 AST。运行时实现见 `src/lib/centralSearchQuery.ts`。
 * 这里以接口形式导出，仅用于跨模块的类型契约（避免 import 循环）。
 */
export interface CentralQueryAst {
  freeText: string;
  filters: CentralQueryFilter[];
  invalid: string[];
}

export type CentralQueryFilter =
  | { kind: "tag"; value: string; negated: boolean }
  | { kind: "repo"; value: string; negated: boolean }
  | { kind: "owner"; value: string; negated: boolean }
  | { kind: "source"; value: "github" | "local" | "manual"; negated: boolean }
  | { kind: "has"; value: "update" | "no-tag" | "ai-review" | "uncategorized"; negated: boolean }
  | { kind: "platform"; value: string; negated: boolean }
  | {
      kind: "created" | "updated";
      op: "<" | ">" | "<=" | ">=" | "=";
      value: string;
      negated: boolean;
    };
