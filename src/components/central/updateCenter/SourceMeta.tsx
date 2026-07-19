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

// 字段区分靠 <dt> 标签文字，不靠色相：五类行统一中性样板（随主题换肤）。
const ROW_STYLE = {
  container: "bg-muted/35 ring-border/60",
  label: "text-muted-foreground",
  value: "text-foreground",
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
    clean(repositoryLabel) ??
    (path || url ? t("central.updateCenter.sourceRepositoryUnknown") : null);

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
    <dl className="mt-2 flex min-w-0 flex-wrap gap-1.5 text-ui-meta leading-5">
      {rows.map((row) => (
        <div
          key={row.key}
          data-source-meta-key={row.key}
          className={`grid w-full min-w-0 max-w-full grid-cols-[auto_minmax(0,1fr)] items-baseline gap-1.5 rounded-md px-2 py-1 ring-1 sm:w-auto ${ROW_STYLE.container}`}
        >
          <dt className={`font-medium ${ROW_STYLE.label}`}>{row.label}</dt>
          <dd
            className={`min-w-0 break-all font-mono ${ROW_STYLE.value}`}
            title={row.value}
          >
            {row.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}
