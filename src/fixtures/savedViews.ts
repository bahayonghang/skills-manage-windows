import { ipcFixtureError, registerIpcFixtures } from "@/lib/ipc";
import type { SavedView } from "@/types";

/** 浏览器演示态的内存数据集：CRUD 全走 fixture 命令，store 只按真实路径 refetch。 */
let views: SavedView[] = [];
let counter = 0;

function nextId(): string {
  counter += 1;
  return `fixture-saved-view-${counter}`;
}

function nowIso(): string {
  return new Date().toISOString();
}

export function registerSavedViewFixtures(): void {
  registerIpcFixtures({
    list_saved_views: () => [...views],
    create_saved_view: ({ input }) => {
      const next: SavedView = {
        id: nextId(),
        name: input.name,
        query: input.query,
        sort_order: views.length,
        icon: input.icon ?? null,
        pinned: input.pinned ?? false,
        created_at: nowIso(),
        updated_at: nowIso(),
      };
      views = [...views, next];
      return next;
    },
    update_saved_view: ({ id, input }) => {
      views = views.map((view) =>
        view.id === id
          ? {
              ...view,
              ...(input.name !== undefined ? { name: input.name } : {}),
              ...(input.query !== undefined ? { query: input.query } : {}),
              ...(input.icon !== undefined ? { icon: input.icon } : {}),
              ...(input.pinned !== undefined ? { pinned: input.pinned } : {}),
              updated_at: nowIso(),
            }
          : view,
      );
      const found = views.find((view) => view.id === id);
      if (!found) {
        throw ipcFixtureError("resource.not_found", "Saved view not found");
      }
      return found;
    },
    delete_saved_view: ({ id }) => {
      views = views.filter((view) => view.id !== id);
    },
    reorder_saved_views: ({ ids }) => {
      const map = new Map(views.map((view) => [view.id, view]));
      const ordered: SavedView[] = [];
      ids.forEach((id, index) => {
        const view = map.get(id);
        if (view) {
          ordered.push({ ...view, sort_order: index, updated_at: nowIso() });
          map.delete(id);
        }
      });
      const trailing = Array.from(map.values()).map((view, idx) => ({
        ...view,
        sort_order: ids.length + idx,
      }));
      views = [...ordered, ...trailing];
    },
  });
}

/** 测试隔离用：清空内存数据集。 */
export function resetSavedViewFixturesForTest(): void {
  views = [];
  counter = 0;
}
