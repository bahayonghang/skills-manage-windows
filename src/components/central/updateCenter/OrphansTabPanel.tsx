import { useTranslation } from "react-i18next";

export function OrphansTabPanel() {
  const { t } = useTranslation();
  return (
    <p className="rounded-xl border border-dashed border-border bg-muted/20 p-4 text-sm text-muted-foreground">
      {t("central.updateCenter.orphansComingSoon")}
    </p>
  );
}
