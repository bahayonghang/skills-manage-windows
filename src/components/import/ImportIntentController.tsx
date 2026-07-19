import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { Inbox, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { isTauriRuntime } from "@/lib/ipc";
import {
  ensureImportIntentListener,
  setImportIntentHandler,
} from "@/lib/importIntentListener";
import {
  openImportIntent,
  parseImportIntentEvent,
  useImportIntentStore,
} from "@/stores/importIntentStore";

export function ImportIntentController() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const githubWizardOpen = useImportIntentStore(
    (state) => state.githubWizardOpen,
  );
  const pendingCount = useImportIntentStore(
    (state) => state.pendingSources.length,
  );
  const consumePendingIntent = useImportIntentStore(
    (state) => state.consumePendingIntent,
  );
  const discardPendingIntent = useImportIntentStore(
    (state) => state.discardPendingIntent,
  );

  useEffect(() => {
    if (!isTauriRuntime()) return;

    const handler = (payload: unknown) => {
      const intent = parseImportIntentEvent(payload);
      if (!intent) return;
      openImportIntent(intent);
      navigate("/central");
    };
    setImportIntentHandler(handler);
    void ensureImportIntentListener();

    return () => {
      setImportIntentHandler(null);
    };
  }, [navigate]);

  if (pendingCount === 0) return null;

  return (
    <aside
      role="status"
      aria-live="polite"
      className="fixed bottom-4 right-4 z-[70] w-[min(24rem,calc(100vw-2rem))] rounded-lg border border-border bg-popover p-3 text-popover-foreground shadow-lg"
    >
      <p className="text-sm font-medium">
        {t("importIntent.pendingTitle", { count: pendingCount })}
      </p>
      <p className="mt-1 text-xs text-muted-foreground">
        {githubWizardOpen
          ? t("importIntent.closeCurrentFirst")
          : t("importIntent.pendingDescription")}
      </p>
      <div className="mt-3 flex justify-end gap-2">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={discardPendingIntent}
        >
          <Trash2 className="size-4" aria-hidden="true" />
          {t("importIntent.discard")}
        </Button>
        <Button
          type="button"
          size="sm"
          disabled={githubWizardOpen}
          onClick={consumePendingIntent}
        >
          <Inbox className="size-4" aria-hidden="true" />
          {t("importIntent.openPending")}
        </Button>
      </div>
    </aside>
  );
}
