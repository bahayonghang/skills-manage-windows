import { useTranslation } from "react-i18next";
import type { SkillUpdateDiagnostic } from "@/types/skillUpdateInventory";

interface SourceMetaProps {
  repositoryLabel?: string | null;
  sourcePath?: string | null;
  sourceUrl?: string | null;
  diagnostics?: SkillUpdateDiagnostic | null;
}

type SourceMetaRow = {
  key: SourceMetaRowKey;
  label: string;
  value: string;
};

type SourceMetaRowKey = "repository" | "path" | "url" | "cache" | "hash";

type SourceMetaRowStyle = {
  container: string;
  label: string;
  value: string;
};

const ROW_STYLES: Record<SourceMetaRowKey, SourceMetaRowStyle> = {
  repository: {
    container: "bg-sky-500/10 ring-sky-500/30",
    label: "text-sky-700 dark:text-sky-300",
    value: "text-sky-950 dark:text-sky-100",
  },
  path: {
    container: "bg-emerald-500/10 ring-emerald-500/30",
    label: "text-emerald-700 dark:text-emerald-300",
    value: "text-emerald-950 dark:text-emerald-100",
  },
  url: {
    container: "bg-cyan-500/10 ring-cyan-500/30",
    label: "text-cyan-700 dark:text-cyan-300",
    value: "text-cyan-950 dark:text-cyan-100",
  },
  cache: {
    container: "bg-muted/35 ring-border/60",
    label: "text-muted-foreground",
    value: "text-foreground/90",
  },
  hash: {
    container: "bg-violet-500/10 ring-violet-500/30",
    label: "text-violet-700 dark:text-violet-300",
    value: "text-violet-950 dark:text-violet-100",
  },
};

function clean(value?: string | null): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

export function SourceMeta({
  repositoryLabel,
  sourcePath,
  sourceUrl,
  diagnostics,
}: SourceMetaProps) {
  const { t } = useTranslation();
  const path = clean(sourcePath);
  const url = clean(sourceUrl);
  const repository =
    clean(repositoryLabel)
    ?? (path || url ? t("central.updateCenter.sourceRepositoryUnknown") : null);

  const rows: SourceMetaRow[] = [];
  if (repository) {
    rows.push({
      key: "repository",
      label: t("central.updateCenter.sourceRepositoryLabel"),
      value: repository,
    });
  }
  if (path) {
    rows.push({
      key: "path",
      label: t("central.updateCenter.sourcePathLabel"),
      value: path,
    });
  }
  if (url) {
    rows.push({
      key: "url",
      label: t("central.updateCenter.sourceUrlLabel"),
      value: url,
    });
  }
  if (diagnostics) {
    rows.push({
      key: "cache",
      label: t("central.updateCenter.cacheLabel"),
      value: t(`central.updateCenter.cachePolicies.${diagnostics.cachePolicy}`),
    });
    if (diagnostics.localHash || diagnostics.remoteHash) {
      rows.push({
        key: "hash",
        label: t("central.updateCenter.hashLabel"),
        value: `${diagnostics.localHash ?? "-"} -> ${diagnostics.remoteHash ?? "-"}`,
      });
    }
  }

  if (rows.length === 0) return null;

  return (
    <dl className="mt-2 flex min-w-0 flex-wrap gap-1.5 text-[11px] leading-5">
      {rows.map((row) => {
        const style = ROW_STYLES[row.key];
        return (
          <div
            key={row.key}
            data-source-meta-key={row.key}
            className={`grid w-full min-w-0 max-w-full grid-cols-[auto_minmax(0,1fr)] items-baseline gap-1.5 rounded-md px-2 py-1 ring-1 sm:w-auto ${style.container}`}
          >
            <dt className={`font-medium ${style.label}`}>{row.label}</dt>
            <dd
              className={`min-w-0 break-all font-mono ${style.value}`}
              title={row.value}
            >
              {row.value}
            </dd>
          </div>
        );
      })}
    </dl>
  );
}
