import type { ReactNode, RefObject } from "react";

export interface SkillsCliInstallMountProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  returnFocusRef: RefObject<HTMLElement | null>;
  contentWidthPx: number | null;
}

/**
 * page-shell ships the adapter unavailable. install-wizard replaces the body
 * of `renderSkillsCliInstallMount` and flips this flag; it must not edit
 * SkillsCliView, SkillsCliHeader, or the page surface controller.
 * Constants stay co-located with the mount component by design.
 */
export const SKILLS_CLI_INSTALL_SURFACE_AVAILABLE = false;

export function SkillsCliInstallMount(
  props: SkillsCliInstallMountProps,
): ReactNode {
  return renderSkillsCliInstallMount(
    props,
    SKILLS_CLI_INSTALL_SURFACE_AVAILABLE,
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export function renderSkillsCliInstallMount(
  props: SkillsCliInstallMountProps,
  available: boolean,
): ReactNode {
  if (!available) {
    return null;
  }
  return (
    <div
      data-testid="skills-cli-install-mount"
      data-open={String(props.open)}
      data-content-width={
        props.contentWidthPx == null ? "" : String(props.contentWidthPx)
      }
      data-has-return-focus={String(props.returnFocusRef != null)}
    />
  );
}
