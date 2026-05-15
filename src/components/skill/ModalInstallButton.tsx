import { CheckCircle2, Download, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useSkillDetailStore } from "@/stores/skillDetailStore";
import { useTargetStore } from "@/stores/targetStore";
import { usePlatformStore } from "@/stores/platformStore";

export interface ModalInstallButtonProps {
  skillId: string;
  onInstallClick?: (skillId: string) => void;
}

export function ModalInstallButton({ skillId, onInstallClick }: ModalInstallButtonProps) {
  const { t } = useTranslation();
  const detail = useSkillDetailStore((s) => s.detail);
  const installingAgentId = useSkillDetailStore((s) => s.installingAgentId);

  // Read activeTarget to re-render when deployment target changes (local vs SSH)
  const _activeTarget = useTargetStore((s) => s.activeTarget);
  void _activeTarget;
  const agents = usePlatformStore((s) => s.agents);

  const skillName = detail?.name ?? skillId;
  const installations = detail?.installations ?? [];
  const isReadOnly = detail?.is_read_only ?? false;

  // Filter to enabled agents (excluding "central" which is a virtual agent)
  const enabledAgents = agents.filter(
    (agent) => agent.is_enabled && agent.id !== "central"
  );

  // Compute whether all enabled agents already have this skill installed
  const installedAgentIds = new Set(installations.map((inst) => inst.agent_id));
  const isAllInstalled =
    enabledAgents.length > 0 &&
    enabledAgents.every((agent) => installedAgentIds.has(agent.id));

  const isInstalling = installingAgentId !== null;
  const isDisabled =
    isAllInstalled || isInstalling || enabledAgents.length === 0 || !onInstallClick;
  const installAriaLabel = t("detail.installSkillAriaLabel", { name: skillName });

  // Do not render if skill is read-only
  if (isReadOnly) {
    return null;
  }

  function handleClick() {
    if (isDisabled) {
      return;
    }
    onInstallClick?.(skillId);
  }

  // Render installed state
  if (isAllInstalled) {
    return (
      <Button
        variant="outline"
        size="sm"
        disabled
        aria-label={installAriaLabel}
      >
        <CheckCircle2 className="size-3.5" />
        <span>{t("detail.installed")}</span>
      </Button>
    );
  }

  // Render installing state
  if (isInstalling) {
    return (
      <Button
        variant="outline"
        size="sm"
        disabled
        aria-label={installAriaLabel}
      >
        <Loader2 className="size-3.5 animate-spin" />
      </Button>
    );
  }

  // Render normal (clickable) state
  return (
    <Button
      variant="default"
      size="sm"
      disabled={isDisabled}
      aria-label={installAriaLabel}
      onClick={handleClick}
    >
      <Download className="size-3.5" />
      <span>{t("common.install")}</span>
    </Button>
  );
}
