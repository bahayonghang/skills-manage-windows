import * as React from "react"
import { Switch as SwitchPrimitive } from "@base-ui/react/switch"

import { cn } from "@/lib/utils"

// ─── Switch ───────────────────────────────────────────────────────────────────

interface SwitchProps extends Omit<React.ComponentProps<typeof SwitchPrimitive.Root>, "onCheckedChange"> {
  onCheckedChange?: (checked: boolean) => void;
}

function Switch({ className, onCheckedChange, ...props }: SwitchProps) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        "group/switch relative inline-flex h-6 w-10 shrink-0 cursor-pointer rounded-full border-2 border-transparent md:h-5 md:w-9",
        "transition-colors duration-200",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
        "disabled:cursor-not-allowed disabled:opacity-50",
        "bg-input data-[checked]:bg-primary",
        className
      )}
      onCheckedChange={
        onCheckedChange
          ? (checked) => onCheckedChange(checked)
          : undefined
      }
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className={cn(
          "pointer-events-none block size-5 rounded-full bg-background shadow-lg ring-0 md:size-4",
          "transition-transform duration-200",
          "translate-x-0 group-data-[checked]/switch:translate-x-4"
        )}
      />
    </SwitchPrimitive.Root>
  )
}

export { Switch }
