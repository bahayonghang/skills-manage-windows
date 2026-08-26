import type { SkillsCliGlobalSkill, SkillsCliInstallTarget } from "@/types";

import {
  buildSkillsCliExportV1,
  selectedSkillsInStoreOrder,
  skillsCliExportFileName,
  stringifySkillsCliExportV1,
  type SkillsCliExportScope,
} from "@/pages/skillsCliBatchModel";

export type SkillsCliExportInventoryWriter = (input: {
  path: string;
  json: string;
}) => Promise<void>;

export type SkillsCliSaveDialog = (input: {
  defaultPath: string;
  filters: Array<{ name: string; extensions: string[] }>;
}) => Promise<string | null>;

export interface ExportSkillsCliInventoryInput {
  scope: SkillsCliExportScope;
  skills: readonly SkillsCliGlobalSkill[];
  targets: readonly SkillsCliInstallTarget[];
  selectedNames?: ReadonlySet<string>;
  exportInventory: SkillsCliExportInventoryWriter;
  now?: Date;
  save?: SkillsCliSaveDialog;
}

async function defaultSaveDialog(): Promise<SkillsCliSaveDialog> {
  const { save } = await import("@tauri-apps/plugin-dialog");
  return save;
}

export async function exportSkillsCliInventory(
  input: ExportSkillsCliInventoryInput,
): Promise<"saved" | "cancelled"> {
  const now = input.now ?? new Date();
  const targetIds = input.targets.map((target) => target.id);
  const skills =
    input.scope === "all"
      ? [...input.skills]
      : selectedSkillsInStoreOrder(
          input.skills,
          input.selectedNames ?? new Set(),
        );
  const envelope = buildSkillsCliExportV1(
    skills,
    input.scope,
    now,
    targetIds,
  );
  const json = stringifySkillsCliExportV1(envelope);
  const defaultPath = skillsCliExportFileName(input.scope, now);
  const save = input.save ?? (await defaultSaveDialog());
  const path = await save({
    defaultPath,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (path == null) {
    return "cancelled";
  }
  await input.exportInventory({ path, json });
  return "saved";
}
