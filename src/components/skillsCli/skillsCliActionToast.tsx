import {
  CheckCircle2,
  CircleAlert,
  ShieldAlert,
  Trash2,
  type LucideIcon,
} from "lucide-react";
import { toast } from "sonner";

import { statusTextClass, type StatusTone } from "@/lib/statusTone";

export const SKILLS_CLI_ACTION_TOAST_ID = "skills-cli-action";
export const SKILLS_CLI_ACTION_TOAST_DURATION_MS = 2_800;

export type SkillsCliToastSemantic =
  | "success"
  | "error"
  | "destructiveSuccess"
  | "destructiveError";

export const SKILLS_CLI_TOAST_ICONS = {
  success: CheckCircle2,
  error: CircleAlert,
  destructiveSuccess: Trash2,
  destructiveError: ShieldAlert,
} as const satisfies Record<SkillsCliToastSemantic, LucideIcon>;

function toastPresentation(semantic: SkillsCliToastSemantic): {
  Icon: LucideIcon;
  tone: StatusTone;
} {
  switch (semantic) {
    case "success":
      return { Icon: SKILLS_CLI_TOAST_ICONS.success, tone: "success" };
    case "error":
      return { Icon: SKILLS_CLI_TOAST_ICONS.error, tone: "error" };
    case "destructiveSuccess":
      return { Icon: SKILLS_CLI_TOAST_ICONS.destructiveSuccess, tone: "success" };
    case "destructiveError":
      return { Icon: SKILLS_CLI_TOAST_ICONS.destructiveError, tone: "error" };
    default: {
      const _exhaustive: never = semantic;
      return _exhaustive;
    }
  }
}

export function showSkillsCliActionToast(input: {
  semantic: SkillsCliToastSemantic;
  message: string;
}): void {
  const { Icon, tone } = toastPresentation(input.semantic);
  toast(input.message, {
    id: SKILLS_CLI_ACTION_TOAST_ID,
    duration: SKILLS_CLI_ACTION_TOAST_DURATION_MS,
    icon: <Icon aria-hidden className={statusTextClass[tone]} />,
  });
}
