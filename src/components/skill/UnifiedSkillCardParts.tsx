import type { ReactNode } from "react";

import { useTextTruncation } from "@/hooks/useTextTruncation";
import { cn } from "@/lib/utils";

export function CardActionButton({
  onClick,
  disabled,
  title,
  ariaLabel,
  icon,
  testId,
  danger,
}: {
  onClick: () => void;
  disabled?: boolean;
  title: string;
  ariaLabel: string;
  icon: ReactNode;
  testId?: string;
  danger?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={ariaLabel}
      data-testid={testId}
      className={cn(
        "focus-ring inline-flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground transition-[scale,background-color,color] active:not-disabled:scale-[0.96] disabled:cursor-default disabled:opacity-50",
        danger
          ? "hover:bg-destructive/10 hover:text-destructive-text"
          : "hover:bg-accent/40 hover:text-primary",
      )}
    >
      {icon}
    </button>
  );
}

export function SkillCardSummary({
  text,
  label,
  lineClamp = 2,
}: {
  text: string;
  label?: string;
  lineClamp?: 2 | 3;
}) {
  const { ref, isTruncated } = useTextTruncation<HTMLParagraphElement>(text);
  return (
    <div className="relative">
      {label && (
        <span className="mr-1.5 inline-flex align-baseline rounded-full border border-primary/15 bg-primary/8 px-1.5 py-0.5 text-xs font-medium leading-none text-primary-text">
          {label}
        </span>
      )}
      <p
        ref={ref}
        data-truncated={isTruncated ? "true" : "false"}
        title={text}
        className={cn(
          "text-pretty break-words text-xs leading-relaxed text-muted-foreground",
          lineClamp === 3 ? "line-clamp-3" : "line-clamp-2",
          label && "inline",
        )}
      >
        {text}
      </p>
      <span
        aria-hidden
        data-truncated={isTruncated ? "true" : "false"}
        className={cn(
          "pointer-events-none absolute inset-x-0 bottom-0 h-5",
          "bg-gradient-to-t from-card to-transparent",
          "opacity-0 transition-opacity duration-150",
          "data-[truncated=true]:opacity-100",
        )}
      />
    </div>
  );
}
