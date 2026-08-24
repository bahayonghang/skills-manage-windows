import { registerIpcFixtures } from "@/lib/ipc";
import type {
  SkillsCliDoctorReport,
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliSourcePreview,
} from "@/types";

const FIXTURE_SKILLS: SkillsCliGlobalSkill[] = [
  {
    name: "demo-skill",
    path: "/Users/fixture/.agents/skills/demo-skill",
    scope: null,
    agents: ["cursor"],
    source: "owner/repo",
    sourceUrl: null,
    sourceType: null,
  },
];

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
    skills_cli_list_global: () => FIXTURE_SKILLS,
    skills_cli_install_targets: () => FIXTURE_TARGETS,
    skills_cli_preview_source: ({ source }) => ({
      ...FIXTURE_PREVIEW,
      source,
    }),
    skills_cli_add_global: () => ({
      installedSkills: 1,
      targetedPlatforms: 1,
    }),
    skills_cli_remove_global: () => null,
    cancel_skills_cli_job: () => true,
  });
}
