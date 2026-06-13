import type { CSSProperties } from "react";

export interface TagColorResult {
  /** 无自定义颜色时返回的 tailwind class 串。 */
  className?: string;
  /** 有 hex 自定义颜色时返回的内联样式（边框/底色用 color-mix 派生）。 */
  style?: CSSProperties;
}

/**
 * 固定色板：每项 = 边框 + 底色 + 文字（含 dark: 变体），保证明暗主题下可读。
 * 不依赖 data-accent，避免与全局 accent 冲突导致全部同色。
 */
const TAG_SCHEMES: readonly string[] = [
  "border-rose-500/30 bg-rose-500/10 text-rose-700 dark:text-rose-300",
  "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
  "border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-300",
  "border-violet-500/30 bg-violet-500/10 text-violet-700 dark:text-violet-300",
  "border-fuchsia-500/30 bg-fuchsia-500/10 text-fuchsia-700 dark:text-fuchsia-300",
  "border-teal-500/30 bg-teal-500/10 text-teal-700 dark:text-teal-300",
  "border-orange-500/30 bg-orange-500/10 text-orange-700 dark:text-orange-300",
  "border-indigo-500/30 bg-indigo-500/10 text-indigo-700 dark:text-indigo-300",
  "border-cyan-500/30 bg-cyan-500/10 text-cyan-700 dark:text-cyan-300",
];

/**
 * repo 色块用的纯色（HSL），与 getTagColor 的 class 配色独立。
 * 同名确定性映射到色相环上的一个固定色相。
 */
const REPO_DOT_HUES: readonly number[] = [
  348, 32, 152, 199, 262, 291, 173, 25, 234, 188,
];

function hashString(input: string): number {
  let hash = 0;
  for (let i = 0; i < input.length; i += 1) {
    hash = (hash << 5) - hash + input.charCodeAt(i);
    hash |= 0; // 转 32 位
  }
  return Math.abs(hash);
}

export function getTagColor(tag: {
  name: string;
  color?: string | null;
}): TagColorResult {
  const hex = tag.color?.trim();
  if (hex) {
    return {
      style: {
        color: hex,
        borderColor: `color-mix(in srgb, ${hex} 35%, transparent)`,
        backgroundColor: `color-mix(in srgb, ${hex} 12%, transparent)`,
      },
    };
  }
  const index = hashString(tag.name) % TAG_SCHEMES.length;
  return { className: TAG_SCHEMES[index] };
}

/**
 * repo 名 → 一个稳定的纯色（HSL 串），用于 footer / sidebar 的 repo 色块头像。
 * 与 getTagColor 共用 hashString，但映射到饱和度/亮度固定的实色，便于直接作 backgroundColor。
 */
export function getRepoDotColor(name: string): string {
  const hue = REPO_DOT_HUES[hashString(name) % REPO_DOT_HUES.length];
  return `hsl(${hue}, 60%, 55%)`;
}
