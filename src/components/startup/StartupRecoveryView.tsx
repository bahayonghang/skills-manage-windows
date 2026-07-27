import {
  AlertTriangle,
  CheckCircle2,
  DatabaseBackup,
  LoaderCircle,
  Power,
  RefreshCw,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useStartupStore } from "@/stores/startupStore";
import type { StartupDiagnostic, StartupIssue, StartupStatus } from "@/types";

const ISSUE_KEYS: Record<StartupIssue, string> = {
  data_directory_unavailable: "startup.issues.dataDirectoryUnavailable",
  database_open_failed: "startup.issues.databaseOpenFailed",
  schema_initialization_failed: "startup.issues.schemaInitializationFailed",
  database_recovery_failed: "startup.issues.databaseRecoveryFailed",
  startup_state_unavailable: "startup.issues.stateUnavailable",
};

const DIAGNOSTIC_KEYS: Record<StartupDiagnostic, string> = {
  not_run: "startup.diagnostics.notRun",
  healthy: "startup.diagnostics.healthy",
  corrupt: "startup.diagnostics.corrupt",
  unavailable: "startup.diagnostics.unavailable",
};

type FailedStartupStatus = Exclude<StartupStatus, { phase: "checking" | "ready" }>;

export function StartupRecoveryView({ status }: { status: FailedStartupStatus }) {
  const { t } = useTranslation();
  const activeAction = useStartupStore((state) => state.activeAction);
  const actionError = useStartupStore((state) => state.actionError);
  const retry = useStartupStore((state) => state.retry);
  const rebuild = useStartupStore((state) => state.rebuild);
  const exit = useStartupStore((state) => state.exit);
  const isBusy = activeAction !== null;
  const canRebuild = status.phase === "recovery_required" && status.canRebuild;

  return (
    <main className="flex min-h-screen items-center bg-background px-6 py-10 text-foreground sm:px-10">
      <section aria-labelledby="startup-error-title" className="mx-auto w-full max-w-2xl">
        <div className="mb-6 flex size-11 items-center justify-center rounded-md border border-destructive/30 bg-destructive/10 text-destructive-text">
          <AlertTriangle className="size-5" aria-hidden="true" />
        </div>

        <p className="text-xs font-semibold uppercase text-muted-foreground">{t("app.name")}</p>
        <h1 id="startup-error-title" className="mt-2 text-2xl font-semibold">
          {t(status.phase === "fatal" ? "startup.fatalTitle" : "startup.recoveryTitle")}
        </h1>
        <p className="mt-3 max-w-xl text-sm leading-6 text-muted-foreground">
          {t(ISSUE_KEYS[status.issue])}
        </p>

        {status.phase === "recovery_required" && (
          <div className="mt-6 border-l-2 border-border pl-4 text-sm text-muted-foreground">
            {t(DIAGNOSTIC_KEYS[status.diagnostic])}
          </div>
        )}

        {status.phase === "recovery_required" && status.backupCreated && (
          <p className="mt-5 flex items-start gap-2 text-sm text-primary">
            <CheckCircle2 className="mt-0.5 size-4" aria-hidden="true" />
            <span>{t("startup.backupCreated")}</span>
          </p>
        )}

        {actionError !== null && (
          <p role="alert" className="mt-5 text-sm text-destructive-text">
            {t("startup.actionError")}
          </p>
        )}

        <div className="mt-8 flex flex-wrap gap-2 border-t border-border pt-5">
          <Button onClick={() => void retry()} disabled={isBusy}>
            {activeAction === "retry" ? (
              <LoaderCircle className="size-4 animate-spin" aria-hidden="true" />
            ) : (
              <RefreshCw className="size-4" aria-hidden="true" />
            )}
            {t(activeAction === "retry" ? "startup.retrying" : "startup.retry")}
          </Button>

          {canRebuild && (
            <Button variant="destructive" onClick={() => void rebuild()} disabled={isBusy}>
              {activeAction === "rebuild" ? (
                <LoaderCircle className="size-4 animate-spin" aria-hidden="true" />
              ) : (
                <DatabaseBackup className="size-4" aria-hidden="true" />
              )}
              {t(activeAction === "rebuild" ? "startup.rebuilding" : "startup.rebuild")}
            </Button>
          )}

          <Button variant="outline" onClick={() => void exit()} disabled={isBusy}>
            {activeAction === "exit" ? (
              <LoaderCircle className="size-4 animate-spin" aria-hidden="true" />
            ) : (
              <Power className="size-4" aria-hidden="true" />
            )}
            {t("startup.exit")}
          </Button>
        </div>
      </section>
    </main>
  );
}
