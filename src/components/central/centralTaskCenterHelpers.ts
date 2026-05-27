import type {
  AiTagJob,
  CentralSkillUpdateJob,
  SkillportStatePortabilityJob,
} from "@/types";

/** 统计活跃任务数（running / cancelling）。 */
export function countActiveCentralTasks(
  aiTagJob: AiTagJob,
  updateJob: CentralSkillUpdateJob,
  portabilityJob: SkillportStatePortabilityJob
): number {
  let count = 0;
  if (isJobActive(aiTagJob.status)) count++;
  if (isJobActive(updateJob.status)) count++;
  if (isJobActive(portabilityJob.status)) count++;
  return count;
}

function isJobActive(status: string): boolean {
  return status === "running" || status === "cancelling";
}
