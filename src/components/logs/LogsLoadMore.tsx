import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";

interface LogsLoadMoreProps {
  loaded: number;
  total: number;
  isLoading: boolean;
  onLoadMore: () => void;
}

export function LogsLoadMore({
  loaded,
  total,
  isLoading,
  onLoadMore,
}: LogsLoadMoreProps) {
  const { t } = useTranslation();
  const triggerRef = useRef<HTMLDivElement | null>(null);
  const hasMore = total > loaded;

  useEffect(() => {
    if (!hasMore || isLoading) return;
    const node = triggerRef.current;
    if (!node || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          onLoadMore();
        }
      },
      { rootMargin: "120px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [hasMore, isLoading, onLoadMore]);

  if (total === 0) return null;

  if (!hasMore) {
    return (
      <div className="border-t border-border bg-muted/20 px-3 py-3 text-center text-xs text-muted-foreground">
        {t("logs.allLoaded", { total })}
      </div>
    );
  }

  return (
    <div ref={triggerRef} className="border-t border-border bg-muted/10">
      <Button
        type="button"
        variant="ghost"
        size="sm"
        className="w-full justify-center"
        onClick={onLoadMore}
        disabled={isLoading}
      >
        {isLoading ? <Loader2 className="size-3.5 animate-spin" /> : null}
        {t("logs.loadMore", { loaded, total })}
      </Button>
    </div>
  );
}
