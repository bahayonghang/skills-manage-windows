import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ChevronDown, Loader2, Server, Settings2 } from "lucide-react";

import { useTargetStore } from "@/stores/targetStore";
import { isRemoteLikeTarget, isWslTarget } from "@/lib/targetKind";
import { cn } from "@/lib/utils";
import type { TargetSummary } from "@/types";

export function TargetQuickSwitcher() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const targets = useTargetStore((s) => s.targets);
  const activeTarget = useTargetStore((s) => s.activeTarget);
  const switchingTargetId = useTargetStore((s) => s.switchingTargetId);
  const switchTarget = useTargetStore((s) => s.switchTarget);
  const error = useTargetStore((s) => s.error);
  const clearError = useTargetStore((s) => s.clearError);

  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;

    function handlePointerDown(event: MouseEvent) {
      if (
        containerRef.current
        && !containerRef.current.contains(event.target as Node)
      ) {
        setOpen(false);
      }
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  function describeTarget(target: TargetSummary): string {
    if (target.kind === "local") return t("targets.localDescription");
    if (isWslTarget(target)) {
      return `${t("targets.kind.wsl")} - ${target.distribution || t("common.unknown")}`;
    }
    const auth =
      target.authMethod === "password"
        ? t("targets.authPassword")
        : t("targets.authKey");
    const host = `${target.username ?? ""}@${target.host ?? ""}:${target.port ?? 22}`;
    return `${auth} - ${host}`;
  }

  function triggerLabel(): string {
    if (isRemoteLikeTarget(activeTarget)) return activeTarget.label;
    return t("targets.local");
  }

  async function handleSelect(targetId: string) {
    if (targetId === activeTarget.id) {
      setOpen(false);
      return;
    }
    if (switchingTargetId) return;
    if (error) clearError();
    try {
      await switchTarget(targetId);
      setOpen(false);
    } catch {
      // store keeps the error; menu stays open so user can retry
    }
  }

  function handleManage() {
    setOpen(false);
    navigate("/settings");
  }

  const isActiveRemote = isRemoteLikeTarget(activeTarget);

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t("targets.quickSwitch.trigger")}
        title={
          activeTarget.kind === "ssh"
            ? `${activeTarget.username ?? ""}@${activeTarget.host ?? ""}`
            : isWslTarget(activeTarget)
              ? activeTarget.distribution ?? activeTarget.label
              : t("targets.local")
        }
        className={cn(
          "z-10 mr-2 hidden max-w-44 items-center gap-1.5 rounded-md px-2 py-1.5 text-xs transition-colors sm:flex",
          isActiveRemote
            ? "bg-primary/10 text-primary hover:bg-primary/15"
            : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
          open && !isActiveRemote && "bg-muted/60 text-foreground",
        )}
      >
        <Server className="size-3.5 shrink-0" />
        <span className="truncate">{triggerLabel()}</span>
        <ChevronDown
          className={cn(
            "size-3 shrink-0 opacity-70 transition-transform",
            open && "rotate-180",
          )}
        />
      </button>

      {open && (
        <div
          role="menu"
          className="absolute right-0 top-full z-50 mt-1 w-72 overflow-hidden rounded-md border border-border bg-popover text-popover-foreground shadow-md"
        >
          <div className="py-1">
            {targets.map((target) => {
              const isActive = target.id === activeTarget.id || target.isActive;
              const isSwitching = switchingTargetId === target.id;
              const otherSwitching = Boolean(switchingTargetId) && !isSwitching;
              return (
                <button
                  key={target.id}
                  type="button"
                  role="menuitem"
                  disabled={isSwitching || otherSwitching}
                  onClick={() => handleSelect(target.id)}
                  className={cn(
                    "flex w-full items-start gap-2 px-3 py-2 text-left text-xs transition-colors",
                    "hover:bg-muted/60 focus:bg-muted/60 focus:outline-none",
                    isActive && "bg-primary/5",
                    "disabled:cursor-not-allowed disabled:opacity-60",
                  )}
                >
                  <Server
                    className={cn(
                      "mt-0.5 size-3.5 shrink-0",
                      isActive ? "text-primary" : "text-muted-foreground",
                    )}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span
                        className={cn(
                          "truncate text-sm font-medium",
                          isActive ? "text-foreground" : "text-foreground/90",
                        )}
                      >
                        {target.kind === "local"
                          ? t("targets.local")
                          : target.label}
                      </span>
                      {isActive && (
                        <span className="rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] text-primary">
                          {t("targets.active")}
                        </span>
                      )}
                    </div>
                    <div className="mt-0.5 truncate text-[11px] text-muted-foreground">
                      {describeTarget(target)}
                    </div>
                  </div>
                  <div className="flex items-center pl-1 pt-0.5">
                    {isSwitching ? (
                      <Loader2 className="size-3.5 animate-spin text-primary" />
                    ) : !isActive ? (
                      <span className="text-[10px] text-muted-foreground">
                        {t("targets.quickSwitch.useThis")}
                      </span>
                    ) : null}
                  </div>
                </button>
              );
            })}

            {targets.length <= 1 && (
              <div className="px-3 py-2 text-[11px] text-muted-foreground">
                {t("targets.quickSwitch.empty")}
              </div>
            )}

            {error && (
              <div className="px-3 pb-2 text-[11px] text-destructive">
                {error}
              </div>
            )}
          </div>

          <div className="border-t border-border">
            <button
              type="button"
              role="menuitem"
              onClick={handleManage}
              className="flex w-full items-center gap-2 px-3 py-2 text-left text-xs text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground focus:bg-muted/60 focus:outline-none"
            >
              <Settings2 className="size-3.5" />
              {t("targets.quickSwitch.manage")}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
