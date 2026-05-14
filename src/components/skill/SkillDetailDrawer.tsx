import { RefObject, useEffect, useId, useRef } from "react";
import { SkillDetailView, type SourceMetadata } from "@/components/skill/SkillDetailView";
import { SkillDetailModalShell } from "@/components/skill/SkillDetailModalShell";
import { ModalInstallButton } from "@/components/skill/ModalInstallButton";
import { useSkillDetailStore } from "@/stores/skillDetailStore";

export interface SkillDetailDrawerProps {
  open: boolean;
  skillId: string | null;
  agentId?: string | null;
  rowId?: string | null;
  onOpenChange: (open: boolean) => void;
  returnFocusRef?: RefObject<HTMLElement | null>;
  /** Direct file path for non-central source skills. */
  filePath?: string | null;
  /** Metadata for non-central source skills. */
  sourceMetadata?: SourceMetadata | null;
}

export function SkillDetailDrawer({
  open,
  skillId,
  agentId,
  rowId,
  onOpenChange,
  returnFocusRef,
  filePath,
  sourceMetadata,
}: SkillDetailDrawerProps) {
  const titleId = useId();
  const showContent = open && (skillId !== null || filePath != null);
  const lastReturnFocusRef = useRef<RefObject<HTMLElement | null> | null>(null);
  const isReadOnly = useSkillDetailStore((s) => s.detail?.is_read_only ?? false);

  useEffect(() => {
    if (returnFocusRef) {
      lastReturnFocusRef.current = returnFocusRef;
    }
  }, [returnFocusRef]);

  return (
    <SkillDetailModalShell
      open={open}
      onOpenChange={onOpenChange}
      returnFocusRef={{
        current:
          returnFocusRef?.current ??
          lastReturnFocusRef.current?.current ??
          null,
      }}
      titleId={showContent ? titleId : undefined}
      headerActions={
        showContent && !isReadOnly && skillId
          ? <ModalInstallButton skillId={skillId} />
          : undefined
      }
    >
      {showContent
        ? (
          <SkillDetailView
            skillId={skillId ?? undefined}
            agentId={agentId ?? undefined}
            rowId={rowId ?? undefined}
            filePath={filePath ?? undefined}
            sourceMetadata={sourceMetadata ?? undefined}
            variant="drawer"
            leading={null}
            onRequestClose={() => onOpenChange(false)}
            titleId={titleId}
          />
        )
        : null}
    </SkillDetailModalShell>
  );
}
