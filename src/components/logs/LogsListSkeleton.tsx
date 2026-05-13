interface LogsListSkeletonProps {
  rows?: number;
}

export function LogsListSkeleton({ rows = 6 }: LogsListSkeletonProps) {
  return (
    <div role="status" aria-busy="true" data-testid="logs-list-skeleton">
      {Array.from({ length: rows }).map((_, index) => (
        <div
          key={index}
          className="grid w-full grid-cols-[9rem_4.5rem_minmax(0,1fr)_minmax(0,1.6fr)_5rem_7rem] items-center gap-3 border-b border-border px-3 py-2.5 last:border-b-0"
        >
          <div className="space-y-1.5">
            <div className="h-3 w-20 animate-pulse rounded bg-muted/70" />
            <div className="h-2.5 w-16 animate-pulse rounded bg-muted/50" />
          </div>
          <div className="h-3 w-10 animate-pulse rounded bg-muted/60" />
          <div className="space-y-1.5">
            <div className="h-3 w-24 animate-pulse rounded bg-muted/70" />
            <div className="h-2.5 w-12 animate-pulse rounded bg-muted/50" />
          </div>
          <div className="space-y-1.5">
            <div className="h-3 w-32 animate-pulse rounded bg-muted/70" />
            <div className="h-2.5 w-2/3 animate-pulse rounded bg-muted/50" />
          </div>
          <div className="h-3 w-12 animate-pulse rounded bg-muted/60" />
          <div className="h-6 w-16 animate-pulse rounded-full bg-muted/60" />
        </div>
      ))}
    </div>
  );
}
