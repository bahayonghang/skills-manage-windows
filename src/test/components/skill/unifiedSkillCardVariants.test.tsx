import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import {
  UnifiedSkillCard,
  type UnifiedSkillCardProps,
} from "@/components/skill/UnifiedSkillCard";

const noop = () => {};

/** 每个场景一个合法最小 props：正例（可构造、可渲染）。 */
const positiveCases: UnifiedSkillCardProps[] = [
  {
    variant: "central",
    name: "case-central",
    checkbox: { checked: false, onChange: noop },
    onDetail: noop,
    onInstallTo: noop,
    onUninstallFromPlatforms: noop,
    onUpdateCentral: noop,
    onDeleteFromCentral: noop,
  },
  {
    variant: "platform",
    name: "case-platform",
    sourceType: "symlink",
    originKind: null,
    isReadOnly: false,
    onDetail: noop,
    uninstallFromLabel: "卸载 case-platform",
  },
  {
    variant: "project",
    name: "case-project",
    originBadge: { kind: "central", label: "中央" },
    platformBadge: { id: "claude-code", name: "Claude Code" },
    onUninstallFromPlatform: noop,
    uninstallFromLabel: "卸载 case-project",
  },
  {
    variant: "import",
    name: "case-import",
    isCentral: false,
    platformBadge: { id: "claude-code", name: "Claude Code" },
    onDetail: noop,
    onInstallToCentral: noop,
    onInstallToPlatform: noop,
  },
  {
    variant: "marketplace",
    name: "case-marketplace",
    onDetail: noop,
  },
  {
    variant: "collection",
    name: "case-collection",
    onDetail: noop,
    onInstallTo: noop,
    onRemove: noop,
  },
  {
    variant: "skillsCli",
    name: "case-skills-cli",
    agents: ["cursor"],
    onUninstall: noop,
  },
];

/**
 * 编译期互斥负例：跨场景 props 必须被 typecheck 拒绝。
 * 每条 `@ts-expect-error` 由 `pnpm typecheck` 双向强制——
 * 若互斥失效（不再报错），tsc 会以 Unused directive 报错。
 */
const negativeCases: UnifiedSkillCardProps[] = [
  {
    variant: "collection",
    name: "bad-collection",
    onDetail: noop,
    onInstallTo: noop,
    onRemove: noop,
    // @ts-expect-error 场景互斥：collection 不接受 central 专属的 editableTags
    editableTags: { tags: [], allTags: [], onAdd: noop, onCreate: noop, onRemove: noop },
  },
  {
    variant: "marketplace",
    name: "bad-marketplace",
    onDetail: noop,
    // @ts-expect-error 场景互斥：marketplace 不接受 central 专属的 platformIcons
    platformIcons: {
      agents: [],
      linkedAgents: [],
      skillId: "x",
      onToggle: noop,
      togglingAgentId: null,
    },
  },
  {
    variant: "platform",
    name: "bad-platform",
    sourceType: "copy",
    originKind: null,
    isReadOnly: false,
    onDetail: noop,
    uninstallFromLabel: "卸载",
    // @ts-expect-error 场景互斥：platform 不接受 collection 专属的 onRemove
    onRemove: noop,
  },
  {
    variant: "central",
    name: "bad-central",
    checkbox: { checked: false, onChange: noop },
    onDetail: noop,
    onInstallTo: noop,
    onUninstallFromPlatforms: noop,
    onUpdateCentral: noop,
    onDeleteFromCentral: noop,
    // @ts-expect-error 场景互斥：central 不接受 marketplace 专属的 onInstall
    onInstall: noop,
  },
  {
    variant: "central",
    name: "bad-central-lifetime",
    checkbox: { checked: false, onChange: noop },
    onDetail: noop,
    onInstallTo: noop,
    onUninstallFromPlatforms: noop,
    onUpdateCentral: noop,
    onDeleteFromCentral: noop,
    // @ts-expect-error 场景互斥：central 不接受 platform 专属的 lifetimeUsage
    lifetimeUsage: { rank: 1, count: 4 },
  },
  {
    variant: "project",
    name: "bad-project",
    originBadge: { kind: "central", label: "中央" },
    platformBadge: { id: "claude-code", name: "Claude Code" },
    onUninstallFromPlatform: noop,
    uninstallFromLabel: "卸载",
    // @ts-expect-error 场景互斥：project 不接受 import 专属的 onInstallToCentral
    onInstallToCentral: noop,
  },
  {
    variant: "skillsCli",
    name: "bad-skills-cli",
    agents: ["cursor"],
    onUninstall: noop,
    // @ts-expect-error 场景互斥：skillsCli 不接受 collection 专属的 onRemove
    onRemove: noop,
  },
];

// JSX 形态同样被拒绝（单行元素，directive 覆盖整行）
const jsxNegativeCase = [
  // @ts-expect-error 场景互斥（JSX 形态）：collection 不接受 central 专属的 statusChipLabel
  <UnifiedSkillCard key="jsx-bad" variant="collection" name="jsx-bad" onDetail={noop} onInstallTo={noop} onRemove={noop} statusChipLabel="可更新" />,
  // @ts-expect-error 场景互斥（JSX 形态）：skillsCli 不接受 marketplace 专属的 onInstall
  <UnifiedSkillCard key="jsx-skills-cli" variant="skillsCli" name="jsx-cli" agents={[]} onUninstall={noop} onInstall={noop} />,
];

describe("UnifiedSkillCard 场景 interface", () => {
  it("七个场景的最小合法 props 均可渲染", () => {
    for (const props of positiveCases) {
      const { unmount } = render(<UnifiedSkillCard {...props} />);
      expect(screen.getByText(props.name)).toBeInTheDocument();
      unmount();
    }
  });

  it("互斥负例仅存在于编译期（运行时对象可构造）", () => {
    expect(negativeCases).toHaveLength(7);
    expect(jsxNegativeCase).toHaveLength(2);
  });
});
