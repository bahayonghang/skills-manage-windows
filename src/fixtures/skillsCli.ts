import { registerIpcFixtures } from "@/lib/ipc";
import {
  EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  type SkillsCliDoctorReport,
  type SkillsCliGlobalSkill,
  type SkillsCliGlobalSnapshot,
  type SkillsCliInstallTarget,
  type SkillsCliSourcePreview,
  type SkillsCliUpdateInventory,
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
        installOrigin: null,
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

const FIXTURE_UPDATE_INVENTORY: SkillsCliUpdateInventory = {
  ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  lastSuccessAt: "2026-08-26T00:00:00.000Z",
  repositories: [
    {
      repositoryKey: "owner/repo@main",
      normalizedSource: "https://github.com/owner/repo",
      branch: "main",
      observedRevisionSha: "bbbbbbb222222222222222222222222222222222",
      status: "ok",
      lastCheckedAt: "2026-08-26T00:00:00.000Z",
      lastErrorCode: null,
      rateLimitResetAt: null,
      pendingCount: 1,
    },
  ],
  skills: [
    {
      skillName: "demo-skill",
      repositoryKey: "owner/repo@main",
      normalizedSource: "https://github.com/owner/repo",
      skillPath: "demo-skill",
      status: "update_available",
      installedRevisionSha: "aaaaaaa111111111111111111111111111111111",
      observedRevisionSha: "bbbbbbb222222222222222222222222222222222",
      pendingRevisionSha: "bbbbbbb222222222222222222222222222222222",
      installedLocalDigest: "sha256-v1:installed",
      observedUpstreamDigest: "sha256-v1:upstream",
      pendingUpstreamDigest: "sha256-v1:upstream",
      isStale: false,
      lastErrorCode: null,
      changeSummary: ["SKILL.md"],
      blockers: [],
      argvPreview: [
        "refresh",
        "owned-canonical",
        "from-pinned-github-snapshot",
        "demo-skill",
      ],
    },
  ],
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
      installOrigin: null,
    }),
    skills_cli_link_platform_batch: ({ items }) => ({
      succeeded: items.map((item) => ({
        skillName: item.skillName,
        agentId: item.skillportAgentId,
      })),
      failed: [],
      skipped: [],
    }),
    skills_cli_unlink_platform: ({ skillportAgentId }) => ({
      agentId: skillportAgentId,
      displayName: "Cursor",
      targetPath: "/Users/fixture/.cursor/skills/demo-skill",
      state: "missing",
      managedLinkKind: null,
      reasonCode: null,
      installOrigin: null,
    }),
    skills_cli_unlink_platform_batch: ({ items }) => ({
      succeeded: items.map((item) => ({
        skillName: item.skillName,
        agentId: item.skillportAgentId,
      })),
      failed: [],
      skipped: [],
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
    skills_cli_update_inventory: () => FIXTURE_UPDATE_INVENTORY,
    skills_cli_check_updates: () => FIXTURE_UPDATE_INVENTORY,
    skills_cli_verify_update_baseline: () => ({
      ...FIXTURE_UPDATE_INVENTORY,
      skills: FIXTURE_UPDATE_INVENTORY.skills.map((row) => ({
        ...row,
        status: "current" as const,
        pendingRevisionSha: null,
        pendingUpstreamDigest: null,
      })),
    }),
    skills_cli_apply_updates: () => ({
      appliedSkillNames: ["demo-skill"],
      installedRevisionSha: "bbbbbbb222222222222222222222222222222222",
    }),
    skills_cli_retry_update_recovery: () => ({
      operationId: "fixture-recovery",
      phase: "db_committed",
    }),
  });
}
