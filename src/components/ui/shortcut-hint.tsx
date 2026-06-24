import { formatShortcut, type ShortcutPlatform } from "@/lib/keyboardShortcuts";
import { cn } from "@/lib/utils";

interface ShortcutHintProps {
  shortcut: string;
  className?: string;
  platform?: ShortcutPlatform;
}

export function ShortcutHint({ shortcut, className, platform }: ShortcutHintProps) {
  const descriptor = formatShortcut(shortcut, platform);

  return (
    <kbd
      aria-label={descriptor.ariaLabel}
      title={descriptor.ariaLabel}
      className={cn(
        "inline-flex items-center gap-0.5 font-mono",
        className,
      )}
    >
      {descriptor.tokens.map((token) => (
        <span key={token}>{token}</span>
      ))}
    </kbd>
  );
}
