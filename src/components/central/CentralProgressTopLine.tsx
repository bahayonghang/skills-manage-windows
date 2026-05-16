import { cn } from "@/lib/utils";
import type {
  AiTagJob,
  CentralSkillUpdateJob,
  SkillportStatePortabilityJob,
} from "@/types";

/**
 * CentralProgressTopLine：贴在容器顶部的 1px 进度线。
 *
 * 基于活跃任务（AI 标签 / 更新检查 / portability）的最高进度。
 * 任意任务 failed → 染 destructive 色。
 * 所有任务 idle → 不渲染。
 */
export interface CentralProgressTopLineProps {
  aiTagJob: AiTagJob;
  updateJob: CentralSkillUpdateJob;
  portabilityJob: SkillportStatePortabilityJob;
}

function jobActive(status: string): boolean {
  return status === "running" || status === "cancelling";
}

function jobFailed(status: string): boolean {
  return status === "failed";
}

function jobRatio(completed: number, total: number): number {
  if (total <= 0) return 0;
  return Math.min(1, completed / total);
}

export function CentralProgressTopLine({
  aiTagJob,
  updateJob,
  portabilityJob,
}: CentralProgressTopLineProps) {
  const aiActive = jobActive(aiTagJob.status);
  const updateActive = jobActive(updateJob.status);
  const portabilityActive = jobActive(portabilityJob.status);
  const isActive = aiActive || updateActive || portabilityActive;
  if (!isActive) return null;

  const ratios: number[] = [];
  if (aiActive) ratios.push(jobRatio(aiTagJob.completed, aiTagJob.total));
  if (updateActive) ratios.push(jobRatio(updateJob.completed, updateJob.total));
  if (portabilityActive)
    ratios.push(jobRatio(portabilityJob.completed, portabilityJob.total));
  const ratio = ratios.length > 0 ? Math.max(...ratios) : 0;
  const hasFailed =
    jobFailed(aiTagJob.status) ||
    jobFailed(updateJob.status) ||
    jobFailed(portabilityJob.status);

  return (
    <div
      role="progressbar"
      aria-valuenow={Math.round(ratio * 100)}
      aria-valuemin={0}
      aria-valuemax={100}
      data-testid="central-progress-top-line"
      className="pointer-events-none absolute inset-x-0 top-0 z-40 h-px overflow-hidden"
    >
      <div
        className={cn(
          "h-full w-full origin-left transition-transform duration-300 ease-out",
          hasFailed ? "bg-destructive" : "bg-primary"
        )}
        style={{ transform: `scaleX(${ratio})` }}
      />
    </div>
  );
}
