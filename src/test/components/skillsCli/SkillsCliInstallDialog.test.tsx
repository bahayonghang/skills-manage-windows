import { createRef, type ComponentProps } from "react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { SkillsCliInstallDialog } from "@/components/skillsCli/SkillsCliInstallDialog";
import { cliTargetToPlatformTarget } from "@/pages/skillsCliInstallViewModel";
import type { SkillsCliInstallTarget, SkillsCliSourcePreview } from "@/types";

const { showSkillsCliActionToast } = vi.hoisted(() => ({
  showSkillsCliActionToast: vi.fn(),
}));

vi.mock("@/components/skillsCli/skillsCliActionToast", () => ({
  showSkillsCliActionToast,
  SKILLS_CLI_ACTION_TOAST_ID: "skills-cli-action",
  SKILLS_CLI_ACTION_TOAST_DURATION_MS: 2800,
}));

const ASYNC_UI_TIMEOUT_MS = 5_000;

const targets: SkillsCliInstallTarget[] = [
  {
    id: "cursor",
    displayName: "Cursor",
    iconName: "cursor",
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "claude-code",
    displayName: "Claude Code",
    iconName: null,
    cliAgent: "claude",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "amp",
    displayName: "Amp",
    iconName: "amp",
    cliAgent: "amp",
    isEnabled: false,
    defaultSelected: false,
  },
];

const preview: SkillsCliSourcePreview = {
  source: "owner/repo",
  skills: ["demo-skill", "helper-skill"],
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function renderDialog(
  overrides: Partial<ComponentProps<typeof SkillsCliInstallDialog>> = {},
) {
  const props: ComponentProps<typeof SkillsCliInstallDialog> = {
    open: true,
    onOpenChange: vi.fn(),
    canonicalRoot: "/tmp/agents",
    npmSpec: "skills@1.5.23",
    targets,
    platformTargets: targets.map(cliTargetToPlatformTarget),
    installedNames: new Set(["demo-skill"]),
    platformInstalledCounts: { cursor: 2, "claude-code": 0, amp: 0 },
    recentSources: ["owner/recent"],
    isRecentSourcesLoading: false,
    isPreviewing: false,
    isMutating: false,
    returnFocusRef: createRef<HTMLElement | null>(),
    onPreview: vi.fn().mockResolvedValue(preview),
    onInstall: vi.fn().mockResolvedValue({
      installedSkills: 1,
      targetedPlatforms: 2,
    }),
    ...overrides,
  };
  const view = render(<SkillsCliInstallDialog {...props} />);
  return { ...view, props };
}

async function findWizard() {
  const dialog = await screen.findByRole(
    "dialog",
    { name: "安装技能" },
    { timeout: ASYNC_UI_TIMEOUT_MS },
  );
  return within(dialog);
}

describe("SkillsCliInstallDialog", () => {
  beforeEach(() => {
    showSkillsCliActionToast.mockClear();
  });

  it("opens on source with a three-state stepper and resets after close", async () => {
    const { rerender, props } = renderDialog();
    const wizard = await findWizard();
    expect(wizard.getByTestId("skills-cli-install-step-source")).toHaveAttribute(
      "data-status",
      "current",
    );
    expect(wizard.getByTestId("skills-cli-install-step-skills")).toHaveAttribute(
      "data-status",
      "pending",
    );
    expect(wizard.getByTestId("skills-cli-install-step-platforms")).toHaveAttribute(
      "data-status",
      "pending",
    );
    expect(wizard.getByRole("button", { name: "关闭" })).toBeInTheDocument();

    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    expect(
      await wizard.findByRole("heading", { name: "要安装的技能" }, {
        timeout: ASYNC_UI_TIMEOUT_MS,
      }),
    ).toBeInTheDocument();
    expect(wizard.getByTestId("skills-cli-install-step-source")).toHaveAttribute(
      "data-status",
      "completed",
    );
    expect(wizard.getByTestId("skills-cli-install-step-skills")).toHaveAttribute(
      "data-status",
      "current",
    );

    rerender(<SkillsCliInstallDialog {...props} open={false} />);
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
    rerender(<SkillsCliInstallDialog {...props} open />);
    const again = await findWizard();
    expect(again.getByTestId("skills-cli-install-step-source")).toHaveAttribute(
      "data-status",
      "current",
    );
    expect(again.getByLabelText("技能来源")).toHaveValue("");
    expect(again.queryByRole("heading", { name: "要安装的技能" })).not.toBeInTheDocument();
  });

  it("stays on source while preview is pending and advances only after success", async () => {
    const pending = deferred<SkillsCliSourcePreview>();
    const onPreview = vi.fn().mockReturnValue(pending.promise);
    renderDialog({ onPreview });
    const wizard = await findWizard();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    expect(await wizard.findByRole("button", { name: "正在预览…" })).toBeDisabled();
    expect(wizard.getByTestId("skills-cli-install-step-source")).toHaveAttribute(
      "data-status",
      "current",
    );
    pending.resolve(preview);
    expect(
      await wizard.findByRole("heading", { name: "要安装的技能" }, {
        timeout: ASYNC_UI_TIMEOUT_MS,
      }),
    ).toBeInTheDocument();
  });

  it("stays on source with an inline error when preview returns no skills", async () => {
    const onPreview = vi.fn().mockResolvedValue({
      source: "owner/repo",
      skills: [],
    });
    renderDialog({ onPreview });
    const wizard = await findWizard();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    const alert = await wizard.findByRole("alert", {}, { timeout: ASYNC_UI_TIMEOUT_MS });
    expect(alert).toHaveTextContent("预览没有返回任何技能。");
    expect(wizard.getByTestId("skills-cli-install-step-source")).toHaveAttribute(
      "data-status",
      "current",
    );
    expect(wizard.getByLabelText("技能来源")).toHaveValue("owner/repo");
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "error",
      message: "预览没有返回任何技能。",
    });
  });

  it("shows a recent-load warning and still lets a manual preview advance", async () => {
    renderDialog({
      recentSources: [],
      recentSourcesLoadFailed: true,
    });
    const wizard = await findWizard();
    expect(
      wizard.getByText("最近来源未能加载。你仍可粘贴来源并预览。"),
    ).toBeInTheDocument();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    expect(
      await wizard.findByRole("heading", { name: "要安装的技能" }, {
        timeout: ASYNC_UI_TIMEOUT_MS,
      }),
    ).toBeInTheDocument();
  });

  it("keeps source editable and shows inline error plus toast when preview fails", async () => {
    const onPreview = vi.fn().mockRejectedValue(
      new Error("skills_cli.source_invalid:The skill source is not allowed."),
    );
    renderDialog({ onPreview });
    const wizard = await findWizard();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "bad source" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    const alert = await wizard.findByRole("alert", {}, { timeout: ASYNC_UI_TIMEOUT_MS });
    expect(alert).toHaveTextContent("技能来源不被允许。");
    expect(wizard.getByTestId("skills-cli-install-step-source")).toHaveAttribute(
      "data-status",
      "current",
    );
    expect(wizard.getByLabelText("技能来源")).not.toBeDisabled();
    expect(showSkillsCliActionToast).toHaveBeenCalledWith({
      semantic: "error",
      message: expect.stringContaining("技能来源不被允许。"),
    });
  });

  it("starts the same preview from a recent pill without changing step first", async () => {
    const pending = deferred<SkillsCliSourcePreview>();
    const onPreview = vi.fn().mockReturnValue(pending.promise);
    renderDialog({ onPreview });
    const wizard = await findWizard();
    fireEvent.click(wizard.getByRole("button", { name: "预览 owner/recent" }));
    expect(onPreview).toHaveBeenCalledWith("owner/recent");
    expect(wizard.getByTestId("skills-cli-install-step-source")).toHaveAttribute(
      "data-status",
      "current",
    );
    expect(wizard.getByRole("button", { name: "正在预览…" })).toBeDisabled();
    pending.resolve({ source: "owner/recent", skills: ["helper-skill"] });
    expect(
      await wizard.findByRole("heading", { name: "要安装的技能" }, {
        timeout: ASYNC_UI_TIMEOUT_MS,
      }),
    ).toBeInTheDocument();
    expect(wizard.getByText("owner/recent", { exact: false })).toBeInTheDocument();
    expect(wizard.getByTestId("skills-cli-install-step-skills")).toHaveAttribute(
      "data-status",
      "current",
    );
  });

  it("single-flights preview and ignores a late settle after close and reopen", async () => {
    const first = deferred<SkillsCliSourcePreview>();
    const onPreview = vi.fn().mockReturnValueOnce(first.promise);
    const { rerender, props } = renderDialog({
      onPreview,
      recentSources: ["owner/recent", "owner/other"],
    });
    const wizard = await findWizard();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/a" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    expect(wizard.getByRole("button", { name: "预览 owner/recent" })).toBeDisabled();
    fireEvent.click(wizard.getByRole("button", { name: "预览 owner/other" }));
    expect(onPreview).toHaveBeenCalledTimes(1);

    rerender(<SkillsCliInstallDialog {...props} onPreview={onPreview} open={false} />);
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
    const second = deferred<SkillsCliSourcePreview>();
    onPreview.mockReturnValueOnce(second.promise);
    rerender(<SkillsCliInstallDialog {...props} onPreview={onPreview} open />);
    const again = await findWizard();
    first.resolve({ source: "stale", skills: ["stale-skill"] });
    await waitFor(() => {
      expect(again.getByTestId("skills-cli-install-step-source")).toHaveAttribute(
        "data-status",
        "current",
      );
    });
    expect(again.queryByText("stale-skill")).not.toBeInTheDocument();
    fireEvent.change(again.getByLabelText("技能来源"), {
      target: { value: "owner/b" },
    });
    fireEvent.click(again.getByRole("button", { name: "预览技能" }));
    second.resolve({ source: "owner/b", skills: ["fresh-skill"] });
    expect(
      await again.findByText("fresh-skill", {}, { timeout: ASYNC_UI_TIMEOUT_MS }),
    ).toBeInTheDocument();
  });

  it("defaults to uninstalled skills and supports select all, clear, and empty continue", async () => {
    renderDialog();
    const wizard = await findWizard();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    const helper = await wizard.findByLabelText("安装 helper-skill", {}, {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    const demo = wizard.getByLabelText("安装 demo-skill");
    expect(helper).toHaveAttribute("data-checked");
    expect(demo).not.toHaveAttribute("data-checked");
    expect(wizard.getByText("已安装")).toBeInTheDocument();
    expect(wizard.getByText(/发现 2 个/)).toBeInTheDocument();

    fireEvent.click(wizard.getByRole("button", { name: "清除" }));
    expect(wizard.getByRole("button", { name: "继续" })).toBeDisabled();
    fireEvent.click(wizard.getByRole("button", { name: "全选" }));
    expect(wizard.getByRole("button", { name: "继续" })).toBeEnabled();
  });

  it("uses the shared platform grid, defaultSelected targets, and mapped cliAgents", async () => {
    const onInstall = vi.fn().mockResolvedValue({
      installedSkills: 1,
      targetedPlatforms: 2,
    });
    renderDialog({ onInstall });
    const wizard = await findWizard();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    fireEvent.click(
      await wizard.findByRole("button", { name: "继续" }, {
        timeout: ASYNC_UI_TIMEOUT_MS,
      }),
    );
    const cursor = await wizard.findByLabelText("Cursor", {}, {
      timeout: ASYNC_UI_TIMEOUT_MS,
    });
    await waitFor(() => {
      expect(cursor).toHaveAttribute("data-checked");
    });
    expect(wizard.getByLabelText("Claude Code")).toHaveAttribute("data-checked");
    expect(wizard.getByLabelText("Amp")).not.toHaveAttribute("data-checked");
    expect(wizard.getByText("npx skills@1.5.23 add owner/repo -s helper-skill -g -a cursor -a claude -y")).toBeInTheDocument();
    expect(wizard.queryByText(/--force/)).not.toBeInTheDocument();
    expect(wizard.queryByText(/--keep-links/)).not.toBeInTheDocument();
    fireEvent.click(wizard.getByRole("button", { name: "安装" }));
    await waitFor(() => {
      expect(onInstall).toHaveBeenCalledWith({
        source: "owner/repo",
        skillNames: ["helper-skill"],
        skillportAgentIds: ["cursor", "claude-code"],
      });
    });
  });

  it("refuses close, backdrop, and Escape while install is pending", async () => {
    const pending = deferred<{ installedSkills: number; targetedPlatforms: number }>();
    const onOpenChange = vi.fn();
    const onInstall = vi.fn().mockReturnValue(pending.promise);
    renderDialog({ onInstall, onOpenChange });
    const wizard = await findWizard();
    fireEvent.change(wizard.getByLabelText("技能来源"), {
      target: { value: "owner/repo" },
    });
    fireEvent.click(wizard.getByRole("button", { name: "预览技能" }));
    fireEvent.click(
      await wizard.findByRole("button", { name: "继续" }, {
        timeout: ASYNC_UI_TIMEOUT_MS,
      }),
    );
    const install = await wizard.findByRole(
      "button",
      { name: "安装" },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
    await waitFor(() => {
      expect(install).toBeEnabled();
    });
    fireEvent.click(install);
    expect(await wizard.findByRole("button", { name: "正在安装…" })).toBeDisabled();
    fireEvent.click(wizard.getByRole("button", { name: "关闭" }));
    fireEvent.click(wizard.getByRole("button", { name: "返回" }));
    const overlay = document.querySelector("[data-slot='dialog-overlay']");
    if (overlay) {
      fireEvent.click(overlay);
    }
    fireEvent.keyDown(screen.getByRole("dialog", { name: "安装技能" }), {
      key: "Escape",
      code: "Escape",
    });
    await userEvent.keyboard("{Escape}");
    expect(onOpenChange).not.toHaveBeenCalledWith(false);
    expect(screen.getByRole("dialog", { name: "安装技能" })).toBeInTheDocument();
    expect(onInstall).toHaveBeenCalledTimes(1);
    pending.resolve({ installedSkills: 1, targetedPlatforms: 2 });
    await waitFor(() => {
      expect(wizard.queryByRole("button", { name: "正在安装…" })).not.toBeInTheDocument();
    });
  });

  it("expands the close control to a 40px hit area and wraps at a narrow width", async () => {
    renderDialog();
    const wizard = await findWizard();
    expect(wizard.getByRole("button", { name: "关闭" }).className).toContain(
      "after:size-10",
    );
    const dialog = screen.getByRole("dialog", { name: "安装技能" });
    expect(dialog.className).toMatch(/min-w-0/);
    expect(dialog.className).toMatch(/overflow-hidden/);
  });

  it("lets Enter preview and Escape close when install is not pending", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    renderDialog({ onOpenChange });
    const wizard = await findWizard();
    const input = wizard.getByLabelText("技能来源");
    await user.click(input);
    await user.type(input, "owner/repo");
    await user.keyboard("{Tab}");
    expect(wizard.getByRole("button", { name: "预览技能" })).toHaveFocus();
    await user.keyboard("{Shift>}{Tab}{/Shift}");
    expect(input).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(
      await wizard.findByRole("heading", { name: "要安装的技能" }, {
        timeout: ASYNC_UI_TIMEOUT_MS,
      }),
    ).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
