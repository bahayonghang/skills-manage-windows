import { Fragment } from "react";
import { useTranslation } from "react-i18next";

import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useHotkey } from "@/hooks/useHotkey";
import { formatShortcut } from "@/lib/keyboardShortcuts";
import { SHORTCUT_GROUPS } from "@/lib/shortcutRegistry";

interface ShortcutsSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function KeyCombo({ combo }: { combo: string }) {
  const descriptor = formatShortcut(combo);

  return (
    <kbd
      aria-label={descriptor.ariaLabel}
      title={descriptor.ariaLabel}
      className="inline-flex items-center gap-1"
    >
      {descriptor.tokens.map((token, index) => (
        <Fragment key={`${token}-${index}`}>
          {index > 0 && (
            <span
              aria-hidden="true"
              className="text-[11px] text-muted-foreground"
            >
              +
            </span>
          )}
          <span className="inline-flex h-5 min-w-5 items-center justify-center rounded-sm bg-muted px-1.5 font-mono text-[11px] leading-none text-foreground ring-1 ring-border">
            {token}
          </span>
        </Fragment>
      ))}
    </kbd>
  );
}

/**
 * Keyboard shortcuts quick-reference overlay. Opens on "?" (outside text
 * inputs) or mod+/ anywhere; closes with Esc via the dialog primitive.
 * Content is rendered from the central registry in lib/shortcutRegistry.ts.
 */
export function ShortcutsSheet({ open, onOpenChange }: ShortcutsSheetProps) {
  const { t } = useTranslation();

  useHotkey("?", () => onOpenChange(!open), { allowInEditable: false });
  useHotkey("mod+/", () => onOpenChange(!open));

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t("shortcuts.title")}</DialogTitle>
          <DialogDescription>{t("shortcuts.description")}</DialogDescription>
        </DialogHeader>
        <DialogBody className="space-y-4">
          {SHORTCUT_GROUPS.map((group) => (
            <section key={group.id} aria-label={t(group.titleKey)}>
              <h3 className="text-[0.72rem] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
                {t(group.titleKey)}
              </h3>
              <ul className="mt-1.5">
                {group.entries.map((entry) => (
                  <li
                    key={entry.id}
                    className="flex items-start justify-between gap-4 py-1.5"
                  >
                    <div className="min-w-0">
                      <div className="text-sm text-foreground">
                        {t(entry.descriptionKey)}
                      </div>
                      {entry.scopeKey && (
                        <div className="text-xs text-muted-foreground">
                          {t(entry.scopeKey)}
                        </div>
                      )}
                    </div>
                    <span className="flex shrink-0 items-center gap-1.5 pt-0.5">
                      {entry.combos.map((combo, index) => (
                        <Fragment key={combo}>
                          {index > 0 && (
                            <span className="text-xs text-muted-foreground">
                              {t("shortcuts.or")}
                            </span>
                          )}
                          <KeyCombo combo={combo} />
                        </Fragment>
                      ))}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </DialogBody>
      </DialogContent>
    </Dialog>
  );
}
