import { RefObject, useEffect, useId, useRef } from "react";
import { SkillDetailView, type DiscoverMetadata } from "@/components/skill/SkillDetailView";
import { SkillDetailPanelShell } from "@/components/skill/SkillDetailPanelShell";

export interface SkillDetailDrawerProps {
  open: boolean;
  skillId: string | null;
  agentId?: string | null;
  rowId?: string | null;
  onOpenChange: (open: boolean) => void;
  returnFocusRef?: RefObject<HTMLElement | null>;
  /** Direct file path for discover non-central skills. */
  filePath?: string | null;
  /** Metadata for discover non-central skills. */
  discoverMetadata?: DiscoverMetadata | null;
}

export function SkillDetailDrawer({
  open,
  skillId,
  agentId,
  rowId,
  onOpenChange,
  returnFocusRef,
  filePath,
  discoverMetadata,
}: SkillDetailDrawerProps) {
  const titleId = useId();
  const showContent = open && (skillId !== null || filePath != null);
  const lastReturnFocusRef = useRef<RefObject<HTMLElement | null> | null>(null);

  useEffect(() => {
    if (returnFocusRef) {
      lastReturnFocusRef.current = returnFocusRef;
    }
  }, [returnFocusRef]);

  return (
    <SkillDetailPanelShell
      open={open}
      onOpenChange={onOpenChange}
      returnFocusRef={{
        current:
          returnFocusRef?.current ??
          lastReturnFocusRef.current?.current ??
          null,
      }}
      titleId={showContent ? titleId : undefined}
    >
      {showContent
        ? (
          <SkillDetailView
            skillId={skillId ?? undefined}
            agentId={agentId ?? undefined}
            rowId={rowId ?? undefined}
            filePath={filePath ?? undefined}
            discoverMetadata={discoverMetadata ?? undefined}
            variant="drawer"
            leading={null}
            onRequestClose={() => onOpenChange(false)}
            titleId={titleId}
          />
        )
        : null}
    </SkillDetailPanelShell>
  );
}
