import { registerIpcFixtures } from "@/lib/ipc";
import type { TagGroup } from "@/types";

/** 浏览器演示态的内存数据集：CRUD 全走 fixture 命令，store 只按真实路径 refetch。 */
let groups: TagGroup[] = [];
let counter = 0;

function nextId(): string {
  counter += 1;
  return `fixture-tag-group-${counter}`;
}

function nowIso(): string {
  return new Date().toISOString();
}

export function registerTagGroupFixtures(): void {
  registerIpcFixtures({
    list_tag_groups: () => [...groups],
    create_tag_group: ({ input }) => {
      const next: TagGroup = {
        id: nextId(),
        name: input.name,
        color: input.color ?? null,
        sort_order: groups.length,
        is_builtin: false,
        created_at: nowIso(),
        updated_at: nowIso(),
      };
      groups = [...groups, next];
      return next;
    },
    update_tag_group: ({ id, input }) => {
      groups = groups.map((g) =>
        g.id === id
          ? {
              ...g,
              ...(input.name !== undefined ? { name: input.name } : {}),
              ...(input.color !== undefined ? { color: input.color } : {}),
              updated_at: nowIso(),
            }
          : g,
      );
      const found = groups.find((g) => g.id === id);
      if (!found) throw new Error(`Tag group '${id}' not found`);
      return found;
    },
    delete_tag_group: ({ id }) => {
      groups = groups.filter((g) => g.id !== id);
    },
    reorder_tag_groups: ({ ids }) => {
      const map = new Map(groups.map((g) => [g.id, g]));
      const ordered: TagGroup[] = [];
      ids.forEach((id, index) => {
        const g = map.get(id);
        if (g) {
          ordered.push({ ...g, sort_order: index, updated_at: nowIso() });
          map.delete(id);
        }
      });
      const trailing = Array.from(map.values()).map((g, idx) => ({
        ...g,
        sort_order: ids.length + idx,
      }));
      groups = [...ordered, ...trailing];
    },
    set_tag_group: () => undefined,
  });
}

/** 测试隔离用：清空内存数据集。 */
export function resetTagGroupFixturesForTest(): void {
  groups = [];
  counter = 0;
}
