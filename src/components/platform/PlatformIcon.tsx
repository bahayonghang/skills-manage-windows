import { memo } from "react";
import { Terminal } from "lucide-react";
import { cn } from "@/lib/utils";

// Real app icons extracted from /Applications/*.app
import autoclawIcon from "@/assets/autoclaw.png";
import workbuddyIcon from "@/assets/workbuddy.png";
import cursorIcon from "@/assets/cursor.png";
import windsurfIcon from "@/assets/windsurf.png";
import traeIcon from "@/assets/trae.png";
import qclawIcon from "@/assets/qclaw.png";
import codebuddyIcon from "@/assets/codebuddy.png";
import kiroIcon from "@/assets/kiro.png";
import qoderIcon from "@/assets/qoder.png";
import factoryDroidIcon from "@/assets/factory-droid.png";
import codexIcon from "@/assets/codex.png";
import easyclawIcon from "@/assets/easyclaw.png";
import openclawIcon from "@/assets/openclaw.png";
import hermesIcon from "@/assets/hermes.png";

// Lobehub real product icons (Mono variants — use currentColor)
import ClaudeCodeIcon from "@lobehub/icons/es/ClaudeCode/components/Mono";
import GithubCopilotIcon from "@lobehub/icons/es/GithubCopilot/components/Mono";
import GeminiCliIcon from "@lobehub/icons/es/GeminiCLI/components/Mono";
import AntigravityIcon from "@lobehub/icons/es/Antigravity/components/Mono";
import GrokIcon from "@lobehub/icons/es/Grok/components/Mono";
import JunieIcon from "@lobehub/icons/es/Junie/components/Mono";
import QwenIcon from "@lobehub/icons/es/Qwen/components/Mono";
import OpenCodeIcon from "@lobehub/icons/es/OpenCode/components/Mono";
import KiloCodeIcon from "@lobehub/icons/es/KiloCode/components/Mono";
import AmpIcon from "@lobehub/icons/es/Amp/components/Mono";

// ─── Platform Icon ────────────────────────────────────────────────────────────
//
// Uses real app PNGs for platforms with local /Applications/*.app installs.
// Falls back to @lobehub/icons Mono variants, then custom SVGs.

const APP_ICONS: Record<string, { src: string; alt: string }> = {
  "autoclaw": { src: autoclawIcon, alt: "AutoClaw" },
  "workbuddy": { src: workbuddyIcon, alt: "WorkBuddy" },
  "cursor": { src: cursorIcon, alt: "Cursor" },
  "windsurf": { src: windsurfIcon, alt: "Windsurf" },
  "trae": { src: traeIcon, alt: "Trae" },
  "trae-cn": { src: traeIcon, alt: "Trae CN" },
  "qclaw": { src: qclawIcon, alt: "QClaw" },
  "codebuddy": { src: codebuddyIcon, alt: "CodeBuddy" },
  "kiro": { src: kiroIcon, alt: "Kiro" },
  "qoder": { src: qoderIcon, alt: "Qoder" },
  "factory-droid": { src: factoryDroidIcon, alt: "Factory Droid" },
  "codex": { src: codexIcon, alt: "Codex CLI" },
  "easyclaw": { src: easyclawIcon, alt: "EasyClaw" },
  "openclaw": { src: openclawIcon, alt: "OpenClaw" },
  "hermes": { src: hermesIcon, alt: "Hermes" },
};

type LobeIconProps = React.SVGProps<SVGSVGElement> & {
  size?: number | string;
};

const LOBEHUB_ICONS: Record<string, React.ComponentType<LobeIconProps>> = {
  "claude-code": ClaudeCodeIcon,
  "antigravity": AntigravityIcon,
  grok: GrokIcon,
  "copilot": GithubCopilotIcon,
  "gemini-cli": GeminiCliIcon,
  "junie": JunieIcon,
  "qwen": QwenIcon,
  "opencode": OpenCodeIcon,
  "kilocode": KiloCodeIcon,
  "amp": AmpIcon,
};

interface PlatformIconProps {
  agentId: string;
  className?: string;
  /** Icon size in pixels (default: 16). */
  size?: number;
}

function PlatformIconComponent({ agentId, className, size = 16 }: PlatformIconProps) {
  // Use real app icon PNG if available
  const appIcon = APP_ICONS[agentId];
  if (appIcon) {
    return (
      <img
        src={appIcon.src}
        width={size}
        height={size}
        alt={appIcon.alt}
        className={cn("shrink-0 rounded-sm", className)}
        aria-hidden
      />
    );
  }

  // Antigravity CLI — base product mark plus a terminal badge so it is
  // statically distinguishable from the plain "antigravity" icon.
  if (agentId === "antigravity-cli") {
    return (
      <span className={cn("relative inline-flex shrink-0", className)} aria-hidden>
        <AntigravityIcon size={size} className="shrink-0" aria-hidden />
        <Terminal
          className="absolute -bottom-1 -right-1 size-2.5 rounded-sm bg-background p-0.5 text-primary ring-1 ring-primary/40"
          strokeWidth={2.5}
          aria-hidden
        />
      </span>
    );
  }

  // Use lobehub real product icon if available
  const LobeIcon = LOBEHUB_ICONS[agentId];
  if (LobeIcon) {
    return <LobeIcon size={size} className={cn("shrink-0", className)} aria-hidden />;
  }

  // Fall back to custom SVGs for remaining platforms
  const svgProps = {
    width: size,
    height: size,
    viewBox: "0 0 16 16",
    fill: "currentColor",
    className: cn("shrink-0", className),
    "aria-hidden": true as const,
    role: "img" as const,
  };

  switch (agentId) {
    case "augment":
      // Augment — arrow/growth symbol (upward arrow with bar)
      return (
        <svg {...svgProps}>
          <path d="M8 2l4 4H9v5H7V6H4z" />
          <rect x="3" y="12.5" width="10" height="1.5" rx="0.5" />
        </svg>
      );

    case "ob1":
      // OB1 — circle with '1'
      return (
        <svg {...svgProps}>
          <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" strokeWidth="1.5" />
          <path d="M7 5.5h1.5v5H7V7l-.8.5-.4-1z" />
        </svg>
      );

    case "aider":
      // Aider — terminal/command line icon
      return (
        <svg {...svgProps}>
          <rect x="2" y="2.5" width="12" height="11" rx="1.5" fill="currentColor" opacity="0.15" />
          <path d="M2 2.5h12a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5H2a1.5 1.5 0 0 1-1.5-1.5V4A1.5 1.5 0 0 1 2 2.5zm1 2v7h10v-7H3zm1 1.5L6 7.5 4 9V7.5z" fillRule="evenodd" />
          <rect x="7" y="8" width="3.5" height="1" rx="0.3" opacity="0.5" />
        </svg>
      );

    case "zed":
      // Zed — stylized Z mark, kept local to avoid adding another icon asset.
      return (
        <svg {...svgProps}>
          <path d="M3 2.5h10v2H6.8l6.2 7V13.5H3v-2h6.2L3 4.5z" />
          <path d="M5.8 6.1h4.4L8 8.6z" fill="none" stroke="currentColor" strokeWidth="1" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      );

    case "central":
      // Central Skills — 3-D cube / package
      return (
        <svg {...svgProps}>
          <path d="M8 1.5 14 4.8v6.4L8 14.5 2 11.2V4.8zm0 1.9L4.3 5.4 8 7.8l3.7-2.5zM3.5 6.7v3.7L7.2 12V8.2zm5.3 5.3L12.5 10.4V6.7L8.8 8.2z" />
        </svg>
      );

    case "universal-agents":
      return (
        <svg {...svgProps}>
          <circle cx="8" cy="8" r="6.25" fill="none" stroke="currentColor" strokeWidth="1.35" />
          <path d="M8 1.8c1.9 1.5 2.9 3.6 2.9 6.2S9.9 12.7 8 14.2C6.1 12.7 5.1 10.6 5.1 8S6.1 3.3 8 1.8z" fill="none" stroke="currentColor" strokeWidth="1.2" />
          <path d="M2.7 6h10.6M2.7 10h10.6" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
        </svg>
      );

    case "reasonix":
      // Reasonix official brand: blue rounded square with white geometric mark
      // Simplified from docs/brand/reasonix-icon.svg (esengine/DeepSeek-Reasonix)
      return (
        <svg {...svgProps} fill="none">
          <rect x="1" y="1" width="14" height="14" rx="3" fill="#0153e5" />
          <path
            d="M4.5 11V5h2.3c1.2 0 2 .3 2.5.9.5.6.7 1.4.5 2.2-.2.7-.6 1.2-1.2 1.5L10.2 11H8.8L7.2 9H5.5v2H4.5zM5.5 8.2h2c.4 0 .8-.1 1-.4.2-.3.3-.7.2-1-.1-.4-.4-.6-.8-.6h-2.4v2z"
            fill="#fff"
          />
          <circle cx="11.5" cy="4.5" r="0.8" fill="#fff" />
        </svg>
      );

    default:
      // Generic terminal / code icon — used for unknown platforms
      return (
        <svg {...svgProps}>
          <rect x="2" y="3" width="12" height="10" rx="1.5" fill="currentColor" opacity="0.15" />
          <path d="M2 3h12a1.5 1.5 0 0 1 1.5 1.5v7A1.5 1.5 0 0 1 14 13H2a1.5 1.5 0 0 1-1.5-1.5v-7A1.5 1.5 0 0 1 2 3zm1 2v6h10V5H3zm1 1.5 2 1.5L4 10v-1.5l.8-.5L4 8V6.5zm3.5 3.5h3v1h-3z" />
        </svg>
      );
  }
}

export const PlatformIcon = memo(PlatformIconComponent);
PlatformIcon.displayName = "PlatformIcon";
