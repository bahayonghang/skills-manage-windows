import { beforeEach, describe, expect, it } from "vitest";
import { ipcFixtureError } from "@/lib/ipc";
import { useRulesStore } from "@/stores/rulesStore";
import { useTargetStore } from "@/stores/targetStore";
import { ipcInvokeCalls, mockIpcCommand } from "@/test/support/ipcMock";
import { mockRulesLoaders, resetRulesStoreForTest, ruleFixture, rulesSnapshotFixture } from "@/test/support/rulesFixtures";
import type { RuleDetail, RulesSnapshot } from "@/types/rules";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("rulesStore", () => {
  beforeEach(() => { resetRulesStoreForTest(); mockRulesLoaders(); });

  it("saves the captured revision and keeps a failed draft", async () => {
    await useRulesStore.getState().load();
    useRulesStore.getState().editBody("draft body");
    mockIpcCommand("save_rule", () => { throw ipcFixtureError("rules.revision_conflict", "Changed"); });
    await expect(useRulesStore.getState().save()).rejects.toMatchObject({ code: "rules.revision_conflict" });
    expect(ipcInvokeCalls("save_rule")[0].args).toEqual({ targetId: "local", name: "review.md", body: "draft body", expectedRevision: "revision-review.md" });
    expect(useRulesStore.getState().drafts["review.md"].body).toBe("draft body");
    expect(useRulesStore.getState().errorCode).toBe("rules.revision_conflict");
    expect(useRulesStore.getState().saving).toBe(false);
  });

  it("keeps text entered while an earlier save is pending", async () => {
    await useRulesStore.getState().load();
    useRulesStore.getState().editBody("first draft");
    const pending = deferred<RuleDetail>();
    mockIpcCommand("save_rule", pending.promise);
    const saving = useRulesStore.getState().save();
    useRulesStore.getState().editBody("newer draft");
    pending.resolve({ ...ruleFixture(), body: "first draft", revision: "saved-revision" });
    await saving;
    expect(useRulesStore.getState().drafts["review.md"]).toEqual({ body: "newer draft", baseBody: "first draft", revision: "saved-revision" });
  });

  it("retains draft and original revision after leaving Local and returning", async () => {
    await useRulesStore.getState().load();
    useRulesStore.getState().editBody("session draft");
    useTargetStore.setState({ activeTarget: { id: "remote", kind: "ssh", label: "Remote", isActive: true } });
    await useRulesStore.getState().load();
    expect(ipcInvokeCalls("list_rules")).toHaveLength(1);
    await expect(useRulesStore.getState().save()).rejects.toThrow("rules.local_only");
    useTargetStore.setState({ activeTarget: { id: "local", kind: "local", label: "Local", isActive: true } });
    mockRulesLoaders(rulesSnapshotFixture([{ ...ruleFixture(), revision: "external-revision" }]));
    await useRulesStore.getState().load();
    expect(useRulesStore.getState().drafts["review.md"]).toMatchObject({ body: "session draft", revision: "revision-review.md" });
    expect(useRulesStore.getState().detail?.revision).toBe("external-revision");
  });

  it("ignores a stale list after target changes", async () => {
    const pending = deferred<RulesSnapshot>();
    mockIpcCommand("list_rules", pending.promise);
    const loading = useRulesStore.getState().load();
    useTargetStore.setState({ activeTarget: { id: "remote", kind: "ssh", label: "Remote", isActive: true } });
    await useRulesStore.getState().load();
    pending.resolve(rulesSnapshotFixture());
    await loading;
    expect(useRulesStore.getState().snapshot).toBeNull();
    expect(useRulesStore.getState().targetId).toBe("remote");
    expect(ipcInvokeCalls("read_rule")).toHaveLength(0);
  });

  it("ignores an older detail response after a new selection", async () => {
    const pending = deferred<RuleDetail>();
    mockIpcCommand("read_rule", ({ name }: { name: string }) => name === "review.md" ? pending.promise : ruleFixture(name));
    const first = useRulesStore.getState().select("review.md");
    await useRulesStore.getState().select("writing.md");
    pending.resolve(ruleFixture());
    await first;
    expect(useRulesStore.getState().detail?.name).toBe("writing.md");
    expect(useRulesStore.getState().detailLoading).toBe(false);
  });

  it("sends a confirmed target fingerprint and clears per-tool pending on failure", async () => {
    await useRulesStore.getState().load();
    mockIpcCommand("set_rule_target_enabled", () => { throw ipcFixtureError("rules.target_conflict", "Changed"); });
    await expect(useRulesStore.getState().setTargetEnabled("omp", true, "omp-original")).rejects.toMatchObject({ code: "rules.target_conflict" });
    expect(ipcInvokeCalls("set_rule_target_enabled")[0].args).toEqual({ targetId: "local", name: "review.md", tool: "omp", enabled: true, expectedDestinationFingerprint: "omp-original" });
    expect(useRulesStore.getState().pendingTools.omp).toBe(false);
    expect(useRulesStore.getState().detail?.targets[1].state).toBe("native_equivalent");
  });

  it("does not let an older read overwrite a completed target mutation", async () => {
    await useRulesStore.getState().load();
    const mutation = deferred<RuleDetail>();
    const read = deferred<RuleDetail>();
    mockIpcCommand("set_rule_target_enabled", mutation.promise);
    const toggling = useRulesStore.getState().setTargetEnabled("omp", true, "omp-original");
    mockIpcCommand("read_rule", read.promise);
    const reading = useRulesStore.getState().select("review.md");
    const updated = ruleFixture();
    updated.targets[1].state = "linked";
    mutation.resolve(updated);
    await toggling;
    read.resolve(ruleFixture());
    await reading;
    expect(useRulesStore.getState().detail?.targets[1].state).toBe("linked");
    expect(useRulesStore.getState().detailLoading).toBe(false);
  });

  it("ignores a saved response after a target round trip without a mounted Rules page", async () => {
    await useRulesStore.getState().load();
    useRulesStore.getState().editBody("session draft");
    const pending = deferred<RuleDetail>();
    mockIpcCommand("save_rule", pending.promise);
    const saving = useRulesStore.getState().save();
    useTargetStore.setState({ activeTarget: { id: "remote", kind: "ssh", label: "Remote", isActive: true } });
    useTargetStore.setState({ activeTarget: { id: "local", kind: "local", label: "Local", isActive: true } });
    const fresh = { ...ruleFixture(), body: "external body", revision: "fresh-revision" };
    mockRulesLoaders(rulesSnapshotFixture([fresh]));
    await useRulesStore.getState().load();
    pending.resolve({ ...ruleFixture(), body: "session draft", revision: "old-save-revision" });
    await saving;
    expect(useRulesStore.getState().detail?.revision).toBe("fresh-revision");
    expect(useRulesStore.getState().drafts["review.md"]).toMatchObject({ body: "session draft", revision: "revision-review.md" });
    expect(useRulesStore.getState().saving).toBe(false);
  });

  it("keeps a new tool operation pending when an older target context completes", async () => {
    await useRulesStore.getState().load();
    const old = deferred<RuleDetail>();
    mockIpcCommand("set_rule_target_enabled", old.promise);
    const first = useRulesStore.getState().setTargetEnabled("omp", true, "omp-original");
    useTargetStore.setState({ activeTarget: { id: "remote", kind: "ssh", label: "Remote", isActive: true } });
    useTargetStore.setState({ activeTarget: { id: "local", kind: "local", label: "Local", isActive: true } });
    await useRulesStore.getState().load();
    const current = deferred<RuleDetail>();
    mockIpcCommand("set_rule_target_enabled", current.promise);
    const second = useRulesStore.getState().setTargetEnabled("omp", true, "omp-original");
    const stale = ruleFixture();
    stale.targets[1].state = "linked";
    old.resolve(stale);
    await first;
    expect(useRulesStore.getState().pendingTools.omp).toBe(true);
    expect(useRulesStore.getState().detail?.targets[1].state).toBe("native_equivalent");
    current.resolve(stale);
    await second;
    expect(useRulesStore.getState().pendingTools.omp).toBe(false);
    expect(useRulesStore.getState().detail?.targets[1].state).toBe("linked");
  });

  it("does not submit conflicting import entries", async () => {
    mockIpcCommand("preview_rules_import", { entries: [
      { name: "ready.md", status: "new", sourceFingerprints: [{ tool: "claude-code", fingerprint: "source" }], description: "", errorCode: null },
      { name: "conflict.md", status: "conflict", sourceFingerprints: [], description: "", errorCode: "rules.target_conflict" },
    ] });
    mockIpcCommand("import_existing_rules", { operationId: "import", entries: [] });
    await useRulesStore.getState().previewImport();
    await useRulesStore.getState().importRules(["ready.md", "conflict.md"]);
    expect(ipcInvokeCalls("import_existing_rules")[0].args).toEqual({ targetId: "local", entries: [{ name: "ready.md", sourceFingerprints: [{ tool: "claude-code", fingerprint: "source" }] }] });
  });

  it("marks committed mutation with failed refresh as reload-required", async () => {
    mockIpcCommand("create_rule", ruleFixture("new.md"));
    mockIpcCommand("list_rules", () => { throw ipcFixtureError("rules.io", "Read failed"); });
    await expect(useRulesStore.getState().createRule("new.md", "", "body")).rejects.toMatchObject({ code: "rules.io" });
    expect(useRulesStore.getState().detail?.name).toBe("new.md");
    expect(useRulesStore.getState().requiresReload).toBe(true);
  });

  it("preserves recovery records after blocked recovery", async () => {
    const snapshot = rulesSnapshotFixture();
    snapshot.recoveryOperations = [{ operationId: "recover", name: "review.md", kind: "enable", phase: "prepared", tool: "omp", backupPath: "C:/isolated/backup/review.md" }];
    mockRulesLoaders(snapshot);
    await useRulesStore.getState().load();
    mockIpcCommand("recover_rule_operation", () => { throw ipcFixtureError("rules.recovery_blocked", "External change"); });
    await expect(useRulesStore.getState().recover("recover")).rejects.toMatchObject({ code: "rules.recovery_blocked" });
    expect(useRulesStore.getState().snapshot?.recoveryOperations).toHaveLength(1);
    expect(useRulesStore.getState().mutating).toBe(false);
  });
});
