import { useTranslation } from "react-i18next";
import { Inbox } from "lucide-react";

import { Button } from "@/components/ui/button";

interface LogsEmptyStateProps {
  hasFilters: boolean;
  onResetFilters: () => void;
}

export function LogsEmptyState({
  hasFilters,
  onResetFilters,
}: LogsEmptyStateProps) {
  const { t } = useTranslation();

  return (
    <div
      className="flex flex-col items-center justify-center gap-3 px-4 py-12 text-center"
      data-testid="logs-empty-state"
    >
      <Inbox className="size-10 text-muted-foreground/60" />
      <p className="text-sm text-muted-foreground">{t("logs.empty")}</p>
      {hasFilters && (
        <Button variant="outline" size="sm" onClick={onResetFilters}>
          {t("logs.emptyCta")}
        </Button>
      )}
    </div>
  );
}
