import type { PlacementMutationOutcome } from "@/pages/skillsCliBatchModel";
import type { SkillsCliDocState } from "@/pages/skillsCliDetailModel";
import type {
  SkillsCliAddResult,
  SkillsCliApplyRecoveryResult,
  SkillsCliApplyResult,
  SkillsCliDoctorReport,
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliRemovePlan,
  SkillsCliSkillDoc,
  SkillsCliSourcePreview,
  SkillsCliUpdateInventory,
  SkillsCliUpdateJobPhase,
  SkillsCliUpdateProgress,
} from "@/types";

export type { SkillsCliDocState };

export interface SkillsCliExportInventoryInput {
  path: string;
  json: string;
}

export interface SkillsCliAddInput {
  source: string;
  skillNames: string[];
  skillportAgentIds: string[];
}

export type SkillsCliUpdateJob = {
  jobId: string | null;
  phase: SkillsCliUpdateJobPhase;
};

export interface SkillsCliState {
  skills: SkillsCliGlobalSkill[];
  targets: SkillsCliInstallTarget[];
  preview: SkillsCliSourcePreview | null;
  doctor: SkillsCliDoctorReport | null;
  canonicalRoot: string | null;
  lockPath: string | null;
  isLoading: boolean;
  isRefreshing: boolean;
  isPreviewing: boolean;
  isMutating: boolean;
  isCancelling: boolean;
  jobId: string | null;
  /** Doctor rejection: write paths (install/uninstall) are degraded. */
  runtimeError: string | null;
  /** list_global / install_targets read failure: stale inventory is kept. */
  inventoryError: string | null;
  /** preview/add/remove failure: toast + inline in the install section. */
  actionError: string | null;
  docState: SkillsCliDocState;
  updateInventory: SkillsCliUpdateInventory;
  isLoadingUpdateCache: boolean;
  updateJob: SkillsCliUpdateJob;
  updateError: string | null;
  updateProgress: SkillsCliUpdateProgress | null;

  loadAll: () => Promise<void>;
  loadUpdateInventory: () => Promise<void>;
  checkUpdates: () => Promise<SkillsCliUpdateInventory>;
  verifyUpdateBaseline: (skillNames: string[]) => Promise<SkillsCliUpdateInventory>;
  applyUpdates: (input: {
    repositoryKey: string;
    skillNames: string[];
  }) => Promise<SkillsCliApplyResult>;
  retryUpdateRecovery: (
    operationId: string,
  ) => Promise<SkillsCliApplyRecoveryResult>;
  cancelUpdateJob: () => Promise<void>;
  previewSource: (source: string) => Promise<SkillsCliSourcePreview | null>;
  addGlobal: (input: SkillsCliAddInput) => Promise<SkillsCliAddResult>;
  removeGlobal: (skillName: string) => Promise<boolean>;
  previewRemoveGlobal: (skillName: string) => Promise<SkillsCliRemovePlan | null>;
  readSkillMd: (skillName: string) => Promise<SkillsCliSkillDoc | null>;
  readSkillDoc: (skillName: string) => Promise<void>;
  clearSkillDoc: (skillName?: string) => void;
  revealSkillFolder: (skillName: string) => Promise<void>;
  linkPlatform: (skillName: string, agentId: string) => Promise<void>;
  unlinkPlatform: (skillName: string, agentId: string) => Promise<void>;
  linkPlatformBatch: (
    skillNames: string[],
    agentId: string,
  ) => Promise<PlacementMutationOutcome>;
  unlinkManagedBatch: (skillNames: string[]) => Promise<PlacementMutationOutcome>;
  removeGlobalBatch: (skillNames: string[]) => Promise<PlacementMutationOutcome>;
  exportInventory: (input: SkillsCliExportInventoryInput) => Promise<void>;
  cancelJob: () => Promise<void>;
  resetForTargetChange: () => void;
}
