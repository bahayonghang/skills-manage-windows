import { registerIpcFixtures } from "@/lib/ipc";
import type {
  SkillsCliDoctorReport,
  SkillsCliGlobalSkill,
  SkillsCliGlobalSnapshot,
  SkillsCliInstallTarget,
  SkillsCliSourcePreview,
} from "@/types";

const FIXTURE_SKILLS: SkillsCliGlobalSkill[] = [
  {
    name: "demo-skill",
    path: "/Users/fixture/.agents/skills/demo-skill",
    installKind: "copy",
    scope: null,
    agents: ["cursor"],
    source: "owner/repo",
    sourceUrl: null,
    sourceType: null,
    sourceTypeBucket: "github",
    canonicalPath: null,
    folderHash: null,
    installedAt: null,
    updatedAt: null,
    placements: [
      {
        agentId: "cursor",
        displayName: "Cursor",
        targetPath: "/Users/fixture/.cursor/skills/demo-skill",
        state: "direct_copy",
        managedLinkKind: null,
        reasonCode: null,
      },
    ],
  },
];

const FIXTURE_SNAPSHOT: SkillsCliGlobalSnapshot = {
  skills: FIXTURE_SKILLS,
  canonicalRoot: "/Users/fixture/.agents",
  lockPath: "/Users/fixture/.agents/skills.lock",
};

const FIXTURE_TARGETS: SkillsCliInstallTarget[] = [
  {
    id: "cursor",
    displayName: "Cursor",
    iconName: null,
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "amp",
    displayName: "Amp",
    iconName: null,
    cliAgent: "amp",
    isEnabled: false,
    defaultSelected: false,
  },
];

const FIXTURE_PREVIEW: SkillsCliSourcePreview = {
  source: "owner/repo",
  skills: ["demo-skill", "helper-skill"],
};

const FIXTURE_DOCTOR: SkillsCliDoctorReport = {
  nodeVersion: "v22.20.0",
  npmSpec: "skills@1.5.23",
};

export function registerSkillsCliFixtures(): void {
  registerIpcFixtures({
    skills_cli_doctor: () => FIXTURE_DOCTOR,
    skills_cli_list_global: () => FIXTURE_SNAPSHOT,
    skills_cli_install_targets: () => FIXTURE_TARGETS,
    skills_cli_preview_source: ({ source }) => ({
      ...FIXTURE_PREVIEW,
      source,
    }),
    skills_cli_add_global: () => ({
      installedSkills: 1,
      targetedPlatforms: 1,
    }),
    skills_cli_remove_global: () => ({
      removedCanonical: true,
      removedManagedAgentIds: ["cursor"],
      retainedDirectCopyAgentIds: [],
    }),
    skills_cli_read_skill_md: ({ skillName }) => ({
      skillName,
      content: "# Demo",
      byteSize: 6,
    }),
    skills_cli_link_platform: ({ skillportAgentId }) => ({
      agentId: skillportAgentId,
      displayName: "Cursor",
      targetPath: "/Users/fixture/.cursor/skills/demo-skill",
      state: "managed_link",
      managedLinkKind: "windows_junction",
      reasonCode: null,
    }),
    skills_cli_unlink_platform: ({ skillportAgentId }) => ({
      agentId: skillportAgentId,
      displayName: "Cursor",
      targetPath: "/Users/fixture/.cursor/skills/demo-skill",
      state: "missing",
      managedLinkKind: null,
      reasonCode: null,
    }),
    skills_cli_reveal_skill_folder: () => null,
    skills_cli_export_inventory: () => null,
    skills_cli_preview_remove_global: ({ skillName }) => ({
      skillName,
      ownedCanonical: true,
      managedPlacements: [{ agentId: "cursor", displayName: "Cursor" }],
      retainedDirectCopies: [],
      conflicts: [],
      confirmable: true,
    }),
    cancel_skills_cli_job: () => true,
  });
}
