import {
  useCallback,
  useEffect,
  useReducer,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";
import { XIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { usePlatformTargetSelection } from "@/components/platform/PlatformMultiSelect";
import {
  SkillsCliInstallFooter,
  SkillsCliInstallPlatformsStep,
  SkillsCliInstallSkillsStep,
  SkillsCliInstallSourceStep,
  SkillsCliInstallStepper,
} from "@/components/skillsCli/SkillsCliInstallDialog.steps";
import { showSkillsCliActionToast } from "@/components/skillsCli/skillsCliActionToast";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { formatBackendError } from "@/lib/backendError";
import type { PlatformTarget } from "@/lib/platformTargetGroups";
import {
  buildInstallCommandPreview,
  createInitialInstallSession,
  defaultSelectedPlatformIds,
  defaultUninstalledSkillNames,
  installDialogReducer,
  mapSelectedCliAgents,
  mapSelectedSkillportAgentIds,
  orderedSelectedSkills,
} from "@/pages/skillsCliInstallViewModel";
import type {
  SkillsCliAddResult,
  SkillsCliInstallTarget,
  SkillsCliSourcePreview,
} from "@/types";

export interface SkillsCliInstallDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  canonicalRoot: string | null;
  npmSpec: string;
  targets: SkillsCliInstallTarget[];
  platformTargets: PlatformTarget[];
  installedNames: ReadonlySet<string>;
  platformInstalledCounts: Readonly<Record<string, number>>;
  recentSources: readonly string[];
  isRecentSourcesLoading: boolean;
  recentSourcesLoadFailed?: boolean;
  isPreviewing: boolean;
  isMutating: boolean;
  returnFocusRef?: RefObject<HTMLElement | null>;
  onPreview: (source: string) => Promise<SkillsCliSourcePreview>;
  onInstall: (input: {
    source: string;
    skillNames: string[];
    skillportAgentIds: string[];
  }) => Promise<SkillsCliAddResult>;
}

export function SkillsCliInstallDialog({
  open,
  onOpenChange,
  npmSpec,
  targets,
  platformTargets,
  installedNames,
  platformInstalledCounts,
  recentSources,
  isRecentSourcesLoading,
  recentSourcesLoadFailed = false,
  isPreviewing,
  isMutating,
  returnFocusRef,
  onPreview,
  onInstall,
}: SkillsCliInstallDialogProps): ReactNode {
  const { t } = useTranslation();
  const [session, dispatch] = useReducer(
    installDialogReducer,
    undefined,
    () => createInitialInstallSession(0),
  );
  const [isInstalling, setIsInstalling] = useState(false);
  const sessionRef = useRef(session);
  const openRef = useRef(open);
  const previewSeqRef = useRef(0);
  const platformsResetForRef = useRef<number | null>(null);
  const sourceInputRef = useRef<HTMLInputElement | null>(null);
  const stepHeadingRef = useRef<HTMLHeadingElement | null>(null);
  sessionRef.current = session;
  openRef.current = open;

  const selection = usePlatformTargetSelection({
    targets: platformTargets,
    isTargetDefaultSelected: (target) =>
      targets.some(
        (cliTarget) => cliTarget.id === target.id && cliTarget.defaultSelected,
      ),
  });
  const { reset: resetPlatformSelection } = selection;

  useEffect(() => {
    if (open) {
      dispatch({ type: "opened" });
      return;
    }
    dispatch({ type: "closed" });
    setIsInstalling(false);
    platformsResetForRef.current = null;
  }, [open]);

  useEffect(() => {
    if (!open) {
      return;
    }
    if (
      session.resolvedPreview &&
      platformsResetForRef.current !== session.sessionId
    ) {
      resetPlatformSelection();
      platformsResetForRef.current = session.sessionId;
    }
  }, [
    open,
    session.resolvedPreview,
    session.sessionId,
    resetPlatformSelection,
  ]);

  useEffect(() => {
    if (session.step !== "platforms") {
      return;
    }
    dispatch({ type: "setPlatformIds", ids: [...selection.selectedIds] });
  }, [selection.selectedIds, session.step]);

  useEffect(() => {
    if (!open) {
      return;
    }
    if (session.step === "source") {
      sourceInputRef.current?.focus();
      return;
    }
    stepHeadingRef.current?.focus();
  }, [open, session.sessionId, session.step]);

  const previewBusy =
    session.pendingPreviewId !== null || isPreviewing || isInstalling;
  const installLocked = isInstalling || isMutating;
  const preview = session.resolvedPreview;
  const selectedCliAgents = mapSelectedCliAgents(
    targets,
    session.selectedPlatformIds,
  );
  const commandPreview =
    preview && session.step === "platforms"
      ? buildInstallCommandPreview({
          npmSpec,
          source: preview.source,
          skillNames: orderedSelectedSkills(
            preview.skills,
            session.selectedSkillNames,
          ),
          cliAgents: selectedCliAgents,
        })
      : "";

  const handleOpenChange = useCallback(
    (nextOpen: boolean) => {
      if (!nextOpen && installLocked) {
        return;
      }
      onOpenChange(nextOpen);
    },
    [installLocked, onOpenChange],
  );

  const awaitPreviewAndAdvance = useCallback(
    async (source: string) => {
      if (
        sessionRef.current.pendingPreviewId !== null ||
        isPreviewing ||
        isInstalling ||
        isMutating
      ) {
        return;
      }
      const trimmed = source.trim();
      if (trimmed === "") {
        return;
      }
      const requestId = previewSeqRef.current + 1;
      previewSeqRef.current = requestId;
      const capturedSession = sessionRef.current.sessionId;
      dispatch({ type: "previewStarted", source: trimmed, requestId });
      try {
        const result = await onPreview(trimmed);
        if (
          !openRef.current ||
          sessionRef.current.sessionId !== capturedSession ||
          sessionRef.current.pendingPreviewId !== requestId
        ) {
          return;
        }
        if (!result || result.skills.length === 0) {
          const message = t("skillsCli.install.previewEmpty");
          dispatch({
            type: "previewFailed",
            sessionId: capturedSession,
            requestId,
            message,
          });
          showSkillsCliActionToast({ semantic: "error", message });
          return;
        }
        dispatch({
          type: "previewSucceeded",
          sessionId: capturedSession,
          requestId,
          result,
          defaultSkillNames: defaultUninstalledSkillNames(
            result.skills,
            installedNames,
          ),
          defaultPlatformIds: defaultSelectedPlatformIds(targets),
        });
      } catch (error) {
        if (
          !openRef.current ||
          sessionRef.current.sessionId !== capturedSession ||
          sessionRef.current.pendingPreviewId !== requestId
        ) {
          return;
        }
        const message = t("skillsCli.previewError", {
          error: formatBackendError(error, t),
        });
        dispatch({
          type: "previewFailed",
          sessionId: capturedSession,
          requestId,
          message,
        });
        showSkillsCliActionToast({ semantic: "error", message });
      }
    },
    [
      installedNames,
      isInstalling,
      isMutating,
      isPreviewing,
      onPreview,
      t,
      targets,
    ],
  );

  async function handleInstall(): Promise<void> {
    if (installLocked || !preview || session.selectedSkillNames.size === 0) {
      return;
    }
    const skillportAgentIds = mapSelectedSkillportAgentIds(
      targets,
      session.selectedPlatformIds,
    );
    if (skillportAgentIds.length === 0) {
      return;
    }
    const capturedSession = session.sessionId;
    dispatch({ type: "installStarted" });
    setIsInstalling(true);
    try {
      await onInstall({
        source: preview.source,
        skillNames: orderedSelectedSkills(
          preview.skills,
          session.selectedSkillNames,
        ),
        skillportAgentIds,
      });
    } catch (error) {
      if (!openRef.current || sessionRef.current.sessionId !== capturedSession) {
        return;
      }
      const message = t("skillsCli.addError", {
        error: formatBackendError(error, t),
      });
      dispatch({ type: "installFailed", message });
      showSkillsCliActionToast({ semantic: "error", message });
    } finally {
      if (sessionRef.current.sessionId === capturedSession) {
        setIsInstalling(false);
      }
    }
  }

  const continueDisabled =
    session.step === "skills" && session.selectedSkillNames.size === 0;
  const installDisabled =
    session.step === "platforms" &&
    (session.selectedPlatformIds.size === 0 || commandPreview === "");

  return (
    <Dialog
      open={open}
      onOpenChange={handleOpenChange}
      disablePointerDismissal={installLocked}
    >
      <DialogContent
        showCloseButton={false}
        initialFocus={sourceInputRef}
        finalFocus={returnFocusRef}
        className="@container/install-dialog flex max-h-[min(90vh,40rem)] w-full min-w-0 flex-col overflow-hidden sm:max-w-xl"
      >
        <DialogHeader className="min-w-0 pr-10">
          <DialogTitle>{t("skillsCli.install.title")}</DialogTitle>
          <DialogDescription>
            {t("skillsCli.install.subtitle")}
          </DialogDescription>
          <SkillsCliInstallStepper step={session.step} />
        </DialogHeader>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          disabled={installLocked}
          aria-label={t("common.close")}
          className="absolute top-2 right-2 after:absolute after:left-1/2 after:top-1/2 after:size-10 after:-translate-x-1/2 after:-translate-y-1/2 after:content-['']"
          onClick={() => handleOpenChange(false)}
        >
          <XIcon />
        </Button>
        <DialogBody className="min-w-0 overflow-x-hidden">
          {session.submitError ? (
            <p role="alert" className="mb-3 text-sm text-destructive-text">
              {session.submitError}
            </p>
          ) : null}
          {session.step === "source" ? (
            <SkillsCliInstallSourceStep
              session={session}
              sourceInputRef={sourceInputRef}
              recentSources={recentSources}
              isRecentSourcesLoading={isRecentSourcesLoading}
              recentSourcesLoadFailed={recentSourcesLoadFailed}
              previewBusy={previewBusy}
              onSourceChange={(value) =>
                dispatch({ type: "sourceChanged", value })
              }
              onPreview={(source) => {
                void awaitPreviewAndAdvance(source);
              }}
            />
          ) : null}
          {session.step === "skills" && preview ? (
            <SkillsCliInstallSkillsStep
              preview={preview}
              installedNames={installedNames}
              selectedSkillNames={session.selectedSkillNames}
              headingRef={stepHeadingRef}
              onToggle={(name) => dispatch({ type: "toggleSkill", name })}
              onSelectAll={() => dispatch({ type: "selectAllSkills" })}
              onClear={() => dispatch({ type: "clearSkills" })}
            />
          ) : null}
          {session.step === "platforms" ? (
            <SkillsCliInstallPlatformsStep
              platformTargets={platformTargets}
              selection={selection}
              platformInstalledCounts={platformInstalledCounts}
              commandPreview={commandPreview}
              headingRef={stepHeadingRef}
            />
          ) : null}
        </DialogBody>
        <DialogFooter>
          <SkillsCliInstallFooter
            step={session.step}
            installLocked={installLocked}
            continueDisabled={continueDisabled}
            installDisabled={installDisabled}
            onBack={() => dispatch({ type: "goBack" })}
            onContinue={() => dispatch({ type: "continueToPlatforms" })}
            onInstall={() => void handleInstall()}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
