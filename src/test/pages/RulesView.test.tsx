import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RulesView } from "@/pages/RulesView";
import { ipcFixtureError } from "@/lib/ipc";
import { useRulesStore } from "@/stores/rulesStore";
import { useTargetStore } from "@/stores/targetStore";
import { ipcInvokeCalls, mockIpcCommand } from "@/test/support/ipcMock";
import { mockRulesLoaders, resetRulesStoreForTest, ruleFixture, rulesSnapshotFixture } from "@/test/support/rulesFixtures";

const { errorToast } = vi.hoisted(() => ({ errorToast: vi.fn() }));
vi.mock("sonner", () => ({ toast: { error: errorToast } }));
const ASYNC_UI_TIMEOUT_MS = 5_000;

async function renderReady() {
  const view = render(<RulesView />);
  await screen.findByRole("textbox", { name: "规则正文" }, { timeout: ASYNC_UI_TIMEOUT_MS });
  return view;
}

describe("RulesView", () => {
  beforeEach(() => { resetRulesStoreForTest(); mockRulesLoaders(); errorToast.mockClear(); });

  it("searches flat file rows and previews body without a skill label", async () => {
    await renderReady();
    fireEvent.change(screen.getByRole("textbox", { name: "搜索规则" }), { target: { value: "writing" } });
    expect(screen.queryByRole("button", { name: "选择 review.md" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "选择 writing.md" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "预览" }));
    expect(screen.getByRole("heading", { name: "Review" })).toBeInTheDocument();
    expect(screen.queryByText("SKILL.md")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "源文件" }));
    expect(screen.getByLabelText("完整源文件")).toHaveTextContent("alwaysApply: true");
  });

  it("requires a dirty-switch decision and keeps failed save text", async () => {
    await renderReady();
    fireEvent.change(screen.getByRole("textbox", { name: "规则正文" }), { target: { value: "unsaved body" } });
    fireEvent.click(screen.getByRole("button", { name: "选择 writing.md" }));
    const dialog = await screen.findByRole("dialog", { name: "正文尚未保存" }, { timeout: ASYNC_UI_TIMEOUT_MS });
    mockIpcCommand("save_rule", () => { throw ipcFixtureError("rules.revision_conflict", "SECRET details"); });
    fireEvent.click(within(dialog).getByRole("button", { name: "保存并切换" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent("草稿已保留");
    expect(screen.queryByText(/SECRET/)).not.toBeInTheDocument();
    expect(useRulesStore.getState().drafts["review.md"].body).toBe("unsaved body");
    expect(errorToast).toHaveBeenCalled();
    fireEvent.click(within(dialog).getByRole("button", { name: "放弃并切换" }));
    await waitFor(() => expect(useRulesStore.getState().selectedName).toBe("writing.md"));
    expect(useRulesStore.getState().drafts["review.md"]).toBeUndefined();
  });

  it("enables tools independently and confirms equivalent-file takeover", async () => {
    await renderReady();
    mockIpcCommand("set_rule_target_enabled", ({ tool, enabled }: { tool: string; enabled: boolean }) => ({ ...ruleFixture(), targets: ruleFixture().targets.map((item) => item.tool === tool ? { ...item, state: enabled ? "linked" : "absent" } : item) }));
    fireEvent.click(screen.getByRole("button", { name: "停用" }));
    await waitFor(() => expect(ipcInvokeCalls("set_rule_target_enabled")).toHaveLength(1));
    expect(ipcInvokeCalls("set_rule_target_enabled")[0].args).toMatchObject({ tool: "claude-code", enabled: false });
    fireEvent.click(screen.getByRole("button", { name: "接管为链接" }));
    const dialog = await screen.findByRole("dialog", { name: "将同文文件接管为链接" }, { timeout: ASYNC_UI_TIMEOUT_MS });
    expect(within(dialog).getByText("C:/isolated/.omp/agent/rules/review.md")).toBeInTheDocument();
    expect(within(dialog).getByText("C:/isolated/.skillport/rules/.backups")).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "接管为链接" }));
    await waitFor(() => expect(ipcInvokeCalls("set_rule_target_enabled")).toHaveLength(2));
    expect(ipcInvokeCalls("set_rule_target_enabled")[1].args).toMatchObject({ tool: "omp", enabled: true, expectedDestinationFingerprint: "omp-original" });
  });

  it("imports selected supported entries without enabling either tool", async () => {
    await renderReady();
    mockIpcCommand("preview_rules_import", { entries: [
      { name: "ready.md", status: "new", sourceFingerprints: [{ tool: "claude-code", fingerprint: "source" }], description: "", errorCode: null },
      { name: "conflict.md", status: "conflict", sourceFingerprints: [], description: "", errorCode: "rules.target_conflict" },
    ] });
    mockIpcCommand("import_existing_rules", { operationId: "import", entries: [{ name: "ready.md", status: "imported", errorCode: null }] });
    fireEvent.click(screen.getByRole("button", { name: "导入现有规则" }));
    const dialog = await screen.findByRole("dialog", { name: "导入现有规则" }, { timeout: ASYNC_UI_TIMEOUT_MS });
    await within(dialog).findByText("ready.md");
    expect(within(dialog).getByRole("checkbox", { name: /conflict.md/ })).toBeDisabled();
    fireEvent.click(within(dialog).getByRole("button", { name: "导入所选（1）" }));
    await within(dialog).findByText("导入完成");
    expect(ipcInvokeCalls("set_rule_target_enabled")).toHaveLength(0);
  });

  it("retains drafts across route unmount and disables remote target access", async () => {
    const view = await renderReady();
    fireEvent.change(screen.getByRole("textbox", { name: "规则正文" }), { target: { value: "route draft" } });
    view.unmount();
    await renderReady();
    expect(screen.getByRole("textbox", { name: "规则正文" })).toHaveValue("route draft");
    const calls = ipcInvokeCalls("list_rules").length;
    act(() => useTargetStore.setState({ activeTarget: { id: "remote", kind: "ssh", label: "Remote", isActive: true } }));
    expect(screen.getByRole("button", { name: "新建规则" })).toBeDisabled();
    expect(screen.getByText(/切换到 Local 目标/)).toBeInTheDocument();
    expect(ipcInvokeCalls("list_rules")).toHaveLength(calls);
    expect(useRulesStore.getState().drafts["review.md"].body).toBe("route draft");
  });

  it("shows blocked recovery in its confirmation", async () => {
    const snapshot = rulesSnapshotFixture();
    snapshot.recoveryOperations = [{ operationId: "recover", name: "review.md", kind: "enable", phase: "prepared", tool: "omp", backupPath: "C:/isolated/backup/review.md" }];
    mockRulesLoaders(snapshot);
    mockIpcCommand("recover_rule_operation", () => { throw ipcFixtureError("rules.recovery_blocked", "External change"); });
    await renderReady();
    fireEvent.click(screen.getByRole("button", { name: "查看恢复" }));
    const dialog = await screen.findByRole("dialog", { name: "恢复中断操作" }, { timeout: ASYNC_UI_TIMEOUT_MS });
    fireEvent.click(within(dialog).getByRole("button", { name: "恢复" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent("备份和操作记录已保留");
    expect(within(dialog).getByText("C:/isolated/backup/review.md")).toBeInTheDocument();
  });

  it("creates rules and confirms deletion with both tool impacts", async () => {
    await renderReady();
    mockIpcCommand("create_rule", ruleFixture("new.md"));
    fireEvent.click(screen.getByRole("button", { name: "新建规则" }));
    const createDialog = await screen.findByRole("dialog", { name: "新建规则" }, { timeout: ASYNC_UI_TIMEOUT_MS });
    fireEvent.change(within(createDialog).getByRole("textbox", { name: "文件名" }), { target: { value: "new.md" } });
    fireEvent.change(within(createDialog).getByRole("textbox", { name: "规则正文" }), { target: { value: "new body" } });
    fireEvent.click(within(createDialog).getByRole("button", { name: "创建" }));
    await waitFor(() => expect(ipcInvokeCalls("create_rule")).toHaveLength(1));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    const deleteDialog = await screen.findByRole("dialog", { name: "删除规则" }, { timeout: ASYNC_UI_TIMEOUT_MS });
    expect(within(deleteDialog).getByText(/Claude Code:/)).toBeInTheDocument();
    expect(within(deleteDialog).getByText(/OMP:/)).toBeInTheDocument();
    mockIpcCommand("delete_rule", { name: "review.md", operationId: "delete", recoveryRequired: false });
    fireEvent.click(within(deleteDialog).getByRole("button", { name: "删除" }));
    await waitFor(() => expect(ipcInvokeCalls("delete_rule")).toHaveLength(1));
  });
});
