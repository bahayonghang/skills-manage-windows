import { AlertTriangle, Check, Link2, Unlink } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { formatBackendError } from "@/lib/backendError";
import type { RuleTargetStatus, RuleTool } from "@/types/rules";

export function RuleTargets({ targets, pending, disabled, onToggle }: {
  targets: RuleTargetStatus[];
  pending: Partial<Record<RuleTool, boolean>>;
  disabled: boolean;
  onToggle: (target: RuleTargetStatus) => void;
}) {
  const { t } = useTranslation();
  return (
    <section aria-label={t("rules.targetsTitle")} className="shrink-0 border-t border-border">
      <div className="px-4 pt-3 text-xs font-medium text-muted-foreground">
        {t("rules.targetsTitle")}
      </div>
      {targets.map((target) => {
        const linked = target.state === "linked";
        const canEnable = target.state === "absent" || target.state === "native_equivalent";
        const Icon = linked ? Check : canEnable ? Unlink : AlertTriangle;
        return (
          <div key={target.tool} className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 px-4 py-2.5">
            <div className="min-w-0 flex-1 basis-40">
              <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                <span className="text-sm font-medium">
                  {t(`rules.tools.${target.tool}`)}
                </span>
                <span className={linked ? "flex items-center gap-1 text-xs text-success" : "flex items-center gap-1 text-xs text-muted-foreground"}>
                  <Icon aria-hidden className="size-3.5" />
                  {t(`rules.states.${target.state}`)}
                </span>
              </div>
              <p className="mt-1 break-all font-mono text-ui-meta text-muted-foreground">{target.path}
              </p>
              {target.errorCode && <p className="mt-1 text-xs text-destructive-text">{formatBackendError(`${target.errorCode}:`, t)}
              </p>}
            </div>
            <Button variant="outline" disabled={disabled || !!pending[target.tool] || (!linked && !canEnable)} onClick={() => onToggle(target)}>
              {linked ? <Unlink aria-hidden /> : <Link2 aria-hidden />}
              {pending[target.tool] ? t("rules.applying") : linked ? t("rules.disable") : target.state === "native_equivalent" ? t("rules.takeOver") : t("rules.enable")}
            </Button>
          </div>
        );
      })}
      <p className="px-4 pb-3 text-xs text-muted-foreground">
        {t("rules.reloadHint")}
      </p>
    </section>
  );
}
