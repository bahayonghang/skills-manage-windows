import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { createRef, useState, type ComponentProps } from "react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import { SkillsCliDetailDrawer } from "@/components/skillsCli/SkillsCliDetailDrawer";
import type { SkillsCliDocState } from "@/pages/skillsCliDetailModel";
import type {
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliPlacement,
  SkillsCliPlacementState,
} from "@/types";

const { showSkillsCliActionToast } = vi.hoisted(() => ({
  showSkillsCliActionToast: vi.fn(),
}));

vi.mock("@/components/skillsCli/skillsCliActionToast", () => ({
  showSkillsCliActionToast,
  SKILLS_CLI_ACTION_TOAST_ID: "skills-cli-action",
  SKILLS_CLI_ACTION_TOAST_DURATION_MS: 2800,
}));

const ASYNC_UI_TIMEOUT_MS = 5_000;
const NOW_MS = Date.parse("2026-08-26T12:00:00.000Z");
const FRONTMATTER = "---\nname: demo\n---\n# Hello";

const targets: SkillsCliInstallTarget[] = [
  {
    id: "cursor",
    displayName: "Cursor",
    iconName: null,
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "amp",
    displayName: "Amp",
    iconName: null,
    cliAgent: "amp",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "codex",
    displayName: "Codex",
    iconName: null,
    cliAgent: "codex",
    isEnabled: false,
    defaultSelected: false,
  },
];

function placement(
  agentId: string,
  state: SkillsCliPlacementState,
  displayName: string,
): SkillsCliPlacement {
  return {
    agentId,
    displayName,
    targetPath: `/tmp/${agentId}/skills/demo`,
    state,
    managedLinkKind: state === "managed_link" ? "windows_junction" : null,
    reasonCode:
      state === "conflict"
        ? "skills_cli.placement_conflict"
        : state === "unavailable"
          ? "skills_cli.placement_unavailable"
          : state === "direct_copy"
            ? "skills_cli.direct_copy_not_toggleable"
            : null,
    installOrigin: null,
  };
}

function makeSkill(
  overrides: Partial<SkillsCliGlobalSkill> = {},
): SkillsCliGlobalSkill {
  return {
    name: "demo-skill",
    path: "/tmp/demo-skill",
    installKind: "canonical",
    scope: "global",
    agents: ["Cursor"],
    source: "owner/repo",
    sourceUrl: "https://github.com/owner/repo",
    sourceType: "github",
    sourceTypeBucket: "github",
    canonicalPath: "/canonical/demo-skill",
    folderHash: "abcdef1234567890",
    installedAt: "2026-08-26T11:00:00.000Z",
    updatedAt: null,
    placements: [
      placement("cursor", "managed_link", "Cursor"),
      placement("amp", "missing", "Amp"),
      placement("codex", "direct_copy", "Codex"),
    ],
    ...overrides,
  };
}

const readyDoc: SkillsCliDocState = {
  status: "ready",
  skillName: "demo-skill",
  content: FRONTMATTER,
  byteSize: FRONTMATTER.length,
};

function renderDrawer(
  overrides: Partial<ComponentProps<typeof SkillsCliDetailDrawer>> = {},
) {
  const props = {
    skill: makeSkill(),
    targets,
    contentWidth: 720,
    docState: readyDoc as SkillsCliDocState,
    updateAvailable: false,
    focusSection: null as "links" | null,
    onClose: vi.fn(),
    onToggleLink: vi.fn(),
    onLinkAll: vi.fn(),
    onUnlinkAll: vi.fn(),
    onRetryDoc: vi.fn(),
    onRevealFolder: vi.fn(),
    onUninstall: vi.fn(),
    nowMs: NOW_MS,
    ...overrides,
  };
  const view = render(<SkillsCliDetailDrawer {...props} />);
  return { ...view, props };
}

describe("SkillsCliDetailDrawer", () => {
  beforeEach(() => {
    showSkillsCliActionToast.mockClear();
  });
  it("does not invoke, import sonner, listen for Escape, or use viewport md width", () => {
    const src = readFileSync(
      resolve(process.cwd(), "src/components/skillsCli/SkillsCliDetailDrawer.tsx"),
      "utf8",
    );
    expect(src).not.toMatch(/from ["']@\/lib\/ipc["']/);
    expect(src).not.toMatch(/from ["']sonner["']/);
    expect(src).not.toMatch(/window\.addEventListener/);
    expect(src).not.toMatch(/document\.addEventListener/);
    expect(src).not.toMatch(/\bmd:w-/);
    expect(src).not.toMatch(/open_in_file_manager/);
    expect(src).toContain("showSkillsCliActionToast");
  });

  it("renders header pills, 720px width, and omits empty metadata", async () => {
    const first = renderDrawer();
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(drawer).toHaveAttribute("data-width-mode", "fixed460");
    expect(drawer).toHaveStyle({ width: "460px" });
    expect(within(drawer).getByText("owner/repo")).toBeInTheDocument();
    const hash = within(drawer).getByText("abcdef1");
    expect(hash).toHaveAttribute(
      "title",
      "技能文件夹的本地内容哈希，不是 git commit。",
    );
    expect(hash).not.toHaveTextContent(/^commit$/i);
    expect(
      within(drawer).getByTitle("2026-08-26T11:00:00.000Z"),
    ).toBeInTheDocument();
    expect(within(drawer).getByTitle("/canonical/demo-skill")).toBeInTheDocument();
    first.unmount();

    renderDrawer({
      skill: makeSkill({
        source: null,
        folderHash: null,
        installedAt: null,
        updatedAt: null,
      }),
      contentWidth: 719,
    });
    const narrow = await screen.findByTestId("skills-cli-detail-drawer");
    expect(narrow).toHaveAttribute("data-width-mode", "fullWidth");
    expect(narrow).toHaveStyle({ width: "719px" });
    expect(within(narrow).getByTestId("skills-cli-detail-pills").textContent).toBe(
      "",
    );
  });

  it("enables managed_link and missing switches and disables copy/conflict/unavailable", async () => {
    renderDrawer({
      skill: makeSkill({
        placements: [
          placement("cursor", "managed_link", "Cursor"),
          placement("amp", "missing", "Amp"),
          placement("codex", "conflict", "Codex"),
        ],
      }),
    });
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const surface = within(drawer);
    expect(
      surface.getByRole("switch", { name: "取消 demo-skill 与 Cursor 的链接" }),
    ).toBeEnabled();
    expect(
      surface.getByRole("switch", { name: "将 demo-skill 链接到 Amp" }),
    ).toBeEnabled();
    expect(
      surface.getByRole("switch", { name: /Codex/ }),
    ).toHaveAttribute("aria-disabled", "true");
    expect(surface.getByTestId("skills-cli-placement-codex")).toHaveTextContent(
      "该平台路径与受管技能冲突。",
    );
  });

  it("shows loading, empty, ready frontmatter, and retryable doc error", async () => {
    const loadingView = renderDrawer({
      docState: { status: "loading", skillName: "demo-skill", requestId: "r1" },
    });
    const loading = await screen.findByTestId(
      "skills-cli-detail-doc-skeleton",
      {},
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(loading).toHaveAttribute("aria-busy", "true");
    loadingView.unmount();

    const emptyView = renderDrawer({
      docState: { status: "empty", skillName: "demo-skill", byteSize: 0 },
    });
    expect(
      await screen.findByTestId("skills-cli-detail-doc-empty"),
    ).toHaveTextContent("SKILL.md 为空。");
    emptyView.unmount();

    const readyView = renderDrawer({ docState: readyDoc });
    const pre = await screen.findByTestId("skills-cli-detail-doc-pre");
    expect(pre).toHaveTextContent("---");
    expect(pre).toHaveTextContent("name: demo");
    expect(pre).toHaveTextContent("# Hello");
    expect(screen.getByText(`${FRONTMATTER.length} 字节`)).toBeInTheDocument();
    readyView.unmount();

    const onRetryDoc = vi.fn();
    renderDrawer({
      docState: {
        status: "error",
        skillName: "demo-skill",
        errorCode: "skills_cli.skill_doc_missing",
      },
      onRetryDoc,
    });
    const alert = await screen.findByTestId("skills-cli-detail-doc-error");
    expect(alert).toHaveTextContent("受管技能文件夹中找不到 SKILL.md。");
    fireEvent.click(within(alert).getByRole("button", { name: "重试" }));
    expect(onRetryDoc).toHaveBeenCalledTimes(1);
  });

  it("omits Update unless both available and wired, and wires Reveal/Uninstall", async () => {
    const onRevealFolder = vi.fn();
    const onUninstall = vi.fn();
    const onUpdate = vi.fn();
    const { unmount } = renderDrawer({
      updateAvailable: true,
      onRevealFolder,
      onUninstall,
    });
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(
      within(drawer).queryByTestId("skills-cli-detail-update"),
    ).not.toBeInTheDocument();
    fireEvent.click(within(drawer).getByTestId("skills-cli-detail-reveal"));
    expect(onRevealFolder).toHaveBeenCalledTimes(1);
    fireEvent.click(within(drawer).getByTestId("skills-cli-detail-uninstall"));
    expect(onUninstall).toHaveBeenCalledTimes(1);
    unmount();

    renderDrawer({ updateAvailable: true, onUpdate });
    expect(
      await screen.findByTestId("skills-cli-detail-update"),
    ).toBeInTheDocument();
  });

  it("scrolls links once for Manage Links focus and keeps 40px close hit area", async () => {
    const scrollIntoView = vi.spyOn(Element.prototype, "scrollIntoView");
    const onFocusConsumed = vi.fn();
    renderDrawer({ focusSection: "links", onFocusConsumed });
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    await waitFor(
      () => {
        expect(scrollIntoView).toHaveBeenCalled();
        expect(onFocusConsumed).toHaveBeenCalledTimes(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(
      within(drawer).getByTestId("skills-cli-detail-close").className,
    ).toContain("after:size-10");
    scrollIntoView.mockRestore();
  });

  it("hides another skill's doc behind a loading skeleton", async () => {
    renderDrawer({
      skill: makeSkill({ name: "beta-skill" }),
      docState: {
        status: "ready",
        skillName: "alpha-skill",
        content: "stale-alpha-body",
        byteSize: 16,
      },
    });
    const drawer = await screen.findByRole(
      "dialog",
      { name: "beta-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(within(drawer).getByTestId("skills-cli-detail-doc-skeleton")).toBeInTheDocument();
    expect(within(drawer).queryByTestId("skills-cli-detail-doc-pre")).not.toBeInTheDocument();
    expect(within(drawer).queryByText("stale-alpha-body")).not.toBeInTheDocument();
  });

  it("links only missing rows from Link all and never mutates copy/conflict/unavailable", async () => {
    const onToggleLink = vi.fn();
    const onLinkAll = vi.fn();
    const onUnlinkAll = vi.fn();
    renderDrawer({
      skill: makeSkill({
        placements: [
          placement("cursor", "missing", "Cursor"),
          placement("amp", "direct_copy", "Amp"),
          placement("codex", "conflict", "Codex"),
        ],
      }),
      onToggleLink,
      onLinkAll,
      onUnlinkAll,
    });
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const surface = within(drawer);
    fireEvent.click(surface.getByRole("switch", { name: /Amp/ }));
    fireEvent.click(surface.getByRole("switch", { name: /Codex/ }));
    expect(onToggleLink).not.toHaveBeenCalled();
    fireEvent.click(surface.getByTestId("skills-cli-detail-link-all"));
    expect(onLinkAll).toHaveBeenCalledTimes(1);
    expect(onUnlinkAll).not.toHaveBeenCalled();
    expect(surface.queryByTestId("skills-cli-detail-unlink-all")).not.toBeInTheDocument();
  });

  it("offers Force unlink on wrong_link_target conflict rows", async () => {
    const onForceUnlink = vi.fn();
    renderDrawer({
      skill: makeSkill({
        placements: [
          {
            ...placement("codex", "conflict", "Codex"),
            reasonCode: "wrong_link_target",
          },
        ],
      }),
      onForceUnlink,
    });
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    const surface = within(drawer);
    const reason = surface.getByTestId("skills-cli-placement-codex");
    expect(reason.textContent).not.toContain("wrong_link_target:");
    fireEvent.click(surface.getByTestId("skills-cli-force-unlink-codex"));
    await waitFor(() => expect(onForceUnlink).toHaveBeenCalledWith("codex"), {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
  });

  it("closes from overlay and close button without a window listener", async () => {
    const onClose = vi.fn();
    const first = renderDrawer({ onClose });
    await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    fireEvent.click(screen.getByTestId("skills-cli-detail-overlay"));
    await waitFor(() => expect(onClose).toHaveBeenCalled(), {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    first.unmount();
    onClose.mockClear();

    renderDrawer({ onClose });
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    fireEvent.click(screen.getByTestId("skills-cli-detail-close"));
    await waitFor(() => expect(onClose).toHaveBeenCalled(), {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    expect(drawer.getAttribute("data-testid")).toBe("skills-cli-detail-drawer");
  });

  it("toasts a formatted link failure without raw path details", async () => {
    const onToggleLink = vi.fn().mockRejectedValue(
      Object.assign(new Error("skills_cli.canonical_missing:The owned skill folder is missing."), {
        code: "skills_cli.canonical_missing",
      }),
    );
    renderDrawer({ onToggleLink });
    const drawer = await screen.findByRole(
      "dialog",
      { name: "demo-skill" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    fireEvent.click(
      within(drawer).getByRole("switch", { name: "将 demo-skill 链接到 Amp" }),
    );
    await waitFor(
      () => {
        expect(screen.getByTestId("skills-cli-detail-mutation-error")).toBeInTheDocument();
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    expect(screen.getByTestId("skills-cli-detail-mutation-error")).toHaveTextContent(
      "受管技能文件夹已缺失。",
    );
    expect(screen.getByTestId("skills-cli-detail-mutation-error")).not.toHaveTextContent(
      "/tmp/",
    );
    expect(showSkillsCliActionToast).toHaveBeenCalledWith(
      expect.objectContaining({ semantic: "error" }),
    );
  });

  it("restores focus to the return target on close", async () => {
    const triggerRef = createRef<HTMLButtonElement>();
    function Harness() {
      const [open, setOpen] = useState(true);
      return (
        <>
          <button ref={triggerRef} type="button">
            open drawer
          </button>
          <SkillsCliDetailDrawer
            open={open}
            skill={makeSkill()}
            targets={targets}
            contentWidth={720}
            docState={readyDoc}
            updateAvailable={false}
            focusSection={null}
            returnFocusRef={triggerRef}
            nowMs={NOW_MS}
            onClose={() => setOpen(false)}
            onToggleLink={() => undefined}
            onLinkAll={() => undefined}
            onUnlinkAll={() => undefined}
            onRetryDoc={() => undefined}
            onRevealFolder={() => undefined}
            onUninstall={() => undefined}
          />
        </>
      );
    }
    render(<Harness />);
    triggerRef.current?.focus();
    fireEvent.click(await screen.findByTestId("skills-cli-detail-close"));
    await waitFor(
      () => {
        expect(screen.getByRole("button", { name: "open drawer" })).toHaveFocus();
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
  });
});
