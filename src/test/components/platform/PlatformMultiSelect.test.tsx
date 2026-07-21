import { describe, expect, it, vi } from "vitest";
import {
  act,
  fireEvent,
  render,
  renderHook,
  screen,
} from "@testing-library/react";

import {
  InstallFailureList,
  PlatformMultiSelectGrid,
  usePlatformTargetSelection,
  type UsePlatformTargetSelectionOptions,
} from "@/components/platform/PlatformMultiSelect";
import {
  getPlatformTargetGroups,
  type PlatformTarget,
  type PlatformTargetGroup,
} from "@/lib/platformTargetGroups";
import type { AgentWithStatus } from "@/types";

const baseAgent = {
  category: "coding",
  is_detected: true,
  is_builtin: true,
  is_enabled: true,
} satisfies Pick<
  AgentWithStatus,
  "category" | "is_detected" | "is_builtin" | "is_enabled"
>;

function agent(
  id: string,
  displayName: string,
  path: string,
  detected = true,
): AgentWithStatus {
  return {
    ...baseAgent,
    id,
    display_name: displayName,
    global_skills_dir: path,
    project_skills_dir: undefined,
    is_detected: detected,
  };
}

/** [universal-agents (codex, cursor), claude-code, kiro (undetected)] */
function buildTargets(): PlatformTarget[] {
  const agents = [
    agent("codex", "Codex CLI", "~/.agents/skills"),
    agent("cursor", "Cursor", "~/.agents/skills"),
    agent("claude-code", "Claude Code", "~/.claude/skills"),
    agent("kiro", "Kiro", "~/.kiro/skills", false),
    agent("central", "Central Skills", "~/.skillsmanage/skills"),
  ];

  return getPlatformTargetGroups(agents, { coding: true, lobster: true });
}

function buildUniversalGroup(): PlatformTargetGroup {
  return {
    ...agent("codex", "Codex CLI", "~/.agents/skills"),
    id: "universal-agents",
    display_name: "Universal",
    icon_name: "universal-agents",
    is_virtual_group: true,
    member_agents: [
      agent("codex", "Codex CLI", "~/.agents/skills"),
      agent("cursor", "Cursor", "~/.agents/skills"),
    ],
    install_agent_id: "codex",
  };
}

describe("usePlatformTargetSelection", () => {
  it("reset selects all targets when no default predicate is given", () => {
    const targets = buildTargets();
    const { result } = renderHook(() =>
      usePlatformTargetSelection({ targets }),
    );

    expect(result.current.selectedIds.size).toBe(0);

    act(() => result.current.reset());

    expect(result.current.selectedIds).toEqual(
      new Set(["universal-agents", "claude-code", "kiro"]),
    );
  });

  it("reset skips disabled targets with the default semantics", () => {
    const targets = buildTargets();
    const { result } = renderHook(() =>
      usePlatformTargetSelection({
        targets,
        isTargetDisabled: (target) => target.id === "kiro",
      }),
    );

    act(() => result.current.reset());

    expect(result.current.selectedIds).toEqual(
      new Set(["universal-agents", "claude-code"]),
    );
  });

  it("reset honors a custom isTargetDefaultSelected", () => {
    const targets = buildTargets();
    const { result } = renderHook(() =>
      usePlatformTargetSelection({
        targets,
        isTargetDefaultSelected: (target) => target.is_detected,
      }),
    );

    act(() => result.current.reset());

    expect(result.current.selectedIds).toEqual(
      new Set(["universal-agents", "claude-code"]),
    );
  });

  it("toggle updates selection but is a no-op for disabled targets", () => {
    const targets = buildTargets();
    const { result } = renderHook(() =>
      usePlatformTargetSelection({
        targets,
        isTargetDisabled: (target) => target.id === "universal-agents",
        isTargetDefaultSelected: () => false,
      }),
    );

    act(() => result.current.toggle("claude-code", true));
    expect(result.current.isSelected(targets[1])).toBe(true);

    act(() => result.current.toggle("claude-code", false));
    expect(result.current.isSelected(targets[1])).toBe(false);

    act(() => result.current.toggle("universal-agents", true));
    expect(result.current.selectedIds.has("universal-agents")).toBe(false);
  });

  it("derives install agent ids from universal groups, dedupes, and excludes disabled targets", () => {
    // Hand-built list: universal group installs via "codex", which collides
    // with the standalone "codex" target — the derivation must dedupe.
    const targets: PlatformTarget[] = [
      buildUniversalGroup(),
      agent("codex", "Codex CLI", "~/.agents/skills"),
      agent("kiro", "Kiro", "~/.kiro/skills"),
    ];
    const { result } = renderHook(() =>
      usePlatformTargetSelection({
        targets,
        isTargetDisabled: (target) => target.id === "kiro",
        isTargetDefaultSelected: () => true,
      }),
    );

    act(() => result.current.reset());

    // kiro is selected but disabled, so it never reaches the install list.
    expect(result.current.selectedIds.has("kiro")).toBe(true);
    expect(result.current.selectedInstallAgentIds()).toEqual(["codex"]);
  });

  it("recomputes the derivation with the predicate passed on the current render", () => {
    const targets = buildTargets();
    const initialProps: UsePlatformTargetSelectionOptions = { targets };
    const { result, rerender } = renderHook(
      (props: UsePlatformTargetSelectionOptions) =>
        usePlatformTargetSelection(props),
      { initialProps },
    );

    act(() => result.current.reset());
    expect(result.current.selectedInstallAgentIds()).toEqual([
      "codex",
      "claude-code",
      "kiro",
    ]);

    // Same selection, new disabled predicate (e.g. InstallDialog switching
    // targetMode) — no reset/toggle in between.
    rerender({ targets, isTargetDisabled: (target) => target.id === "kiro" });

    expect(result.current.selectedInstallAgentIds()).toEqual([
      "codex",
      "claude-code",
    ]);
  });

  it("keeps toggle and reset referentially stable across rerenders", () => {
    const targets = buildTargets();
    const initialProps: UsePlatformTargetSelectionOptions = { targets };
    const { result, rerender } = renderHook(
      (props: UsePlatformTargetSelectionOptions) =>
        usePlatformTargetSelection(props),
      { initialProps },
    );
    const { toggle, reset } = result.current;

    rerender({ targets, isTargetDisabled: () => false });

    expect(result.current.toggle).toBe(toggle);
    expect(result.current.reset).toBe(reset);
  });
});

describe("PlatformMultiSelectGrid", () => {
  const noop = () => {};

  it("renders universal groups with the i18n label, member subtitle, and title hint", () => {
    const targets = buildTargets();
    render(
      <PlatformMultiSelectGrid
        targets={targets}
        isSelected={() => false}
        onToggle={noop}
        emptyMessage="empty"
        ariaLabel="platforms"
      />,
    );

    expect(
      screen.getByRole("group", { name: "platforms" }),
    ).toBeInTheDocument();
    const universalLabel = screen.getByText("Universal (.agents/skills)");
    expect(universalLabel).toBeInTheDocument();
    expect(screen.getByText("Codex CLI, Cursor")).toBeInTheDocument();
    expect(universalLabel.parentElement).toHaveAttribute(
      "title",
      "Codex CLI, Cursor",
    );
    expect(screen.getByText("Claude Code").parentElement).toHaveAttribute(
      "title",
      "~/.claude/skills",
    );
  });

  it("renders the compact universal label when labelVariant is short", () => {
    const targets = buildTargets();
    render(
      <PlatformMultiSelectGrid
        targets={targets}
        isSelected={() => false}
        onToggle={noop}
        labelVariant="short"
        emptyMessage="empty"
        ariaLabel="platforms"
      />,
    );

    expect(screen.getByText("Universal")).toBeInTheDocument();
    expect(
      screen.queryByText("Universal (.agents/skills)"),
    ).not.toBeInTheDocument();
    // Non-universal targets keep their display name in both variants.
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
  });

  it("renders the empty message when there are no targets", () => {
    render(
      <PlatformMultiSelectGrid
        targets={[]}
        isSelected={() => false}
        onToggle={noop}
        emptyMessage="no platforms"
        ariaLabel="platforms"
      />,
    );

    expect(screen.getByText("no platforms")).toBeInTheDocument();
  });

  it("toggles via the name block and reflects the current checked state", () => {
    const targets = buildTargets();
    const onToggle = vi.fn();
    const selected = new Set(["kiro"]);
    render(
      <PlatformMultiSelectGrid
        targets={targets}
        isSelected={(target) => selected.has(target.id)}
        onToggle={onToggle}
        emptyMessage="empty"
        ariaLabel="platforms"
      />,
    );

    fireEvent.click(screen.getByText("Claude Code"));
    expect(onToggle).toHaveBeenCalledWith("claude-code", true);

    fireEvent.click(screen.getByText("Kiro"));
    expect(onToggle).toHaveBeenCalledWith("kiro", false);
  });

  it("does not toggle disabled rows and disables their checkbox", () => {
    const targets = buildTargets();
    const onToggle = vi.fn();
    render(
      <PlatformMultiSelectGrid
        targets={targets}
        isSelected={() => false}
        isDisabled={(target) => target.id === "kiro"}
        onToggle={onToggle}
        emptyMessage="empty"
        ariaLabel="platforms"
      />,
    );

    fireEvent.click(screen.getByText("Kiro"));
    expect(onToggle).not.toHaveBeenCalled();
    expect(screen.getByRole("checkbox", { name: "Kiro" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  });

  it("renders caller badges at the end of each row", () => {
    const targets = buildTargets();
    render(
      <PlatformMultiSelectGrid
        targets={targets}
        isSelected={() => false}
        onToggle={noop}
        renderBadges={(target) =>
          target.is_detected ? null : <span>not detected</span>
        }
        emptyMessage="empty"
        ariaLabel="platforms"
      />,
    );

    expect(screen.getAllByText("not detected")).toHaveLength(1);
  });

  it("renders a platform icon before the name when showIcon is set", () => {
    render(
      <PlatformMultiSelectGrid
        targets={[agent("cursor", "Cursor", "~/.agents/skills")]}
        isSelected={() => false}
        onToggle={noop}
        showIcon
        emptyMessage="empty"
        ariaLabel="platforms"
      />,
    );

    expect(screen.getByAltText("Cursor")).toBeInTheDocument();
  });
});

describe("InstallFailureList", () => {
  it("renders one destructive line per failure", () => {
    render(
      <InstallFailureList
        failures={[
          { key: "claude-code", label: "claude-code: boom" },
          { key: "kiro", label: "kiro: nope" },
        ]}
      />,
    );

    expect(screen.getByText("claude-code: boom")).toBeInTheDocument();
    expect(screen.getByText("kiro: nope")).toBeInTheDocument();
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
  });
});
