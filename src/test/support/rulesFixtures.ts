import type { RuleDetail, RulesSnapshot } from "@/types/rules";
import { useRulesStore } from "@/stores/rulesStore";
import { useTargetStore } from "@/stores/targetStore";
import { mockIpcCommands } from "./ipcMock";

export function ruleFixture(name = "review.md"): RuleDetail {
  return {
    name,
    title: name === "review.md" ? "Review" : "Writing",
    description: "Synthetic acceptance rule",
    body: "# Review\n\nRead affected source files.\n",
    source: "---\nalwaysApply: true\n---\n# Review\n\nRead affected source files.\n",
    bytes: 48,
    revision: `revision-${name}`,
    compatibility: "supported",
    targets: [
      { tool: "claude-code", path: `C:/isolated/.claude/rules/${name}`, state: "linked", fingerprint: "claude-link", errorCode: null },
      { tool: "omp", path: `C:/isolated/.omp/agent/rules/${name}`, state: "native_equivalent", fingerprint: "omp-original", errorCode: null },
    ],
  };
}

export function rulesSnapshotFixture(rules = [ruleFixture(), ruleFixture("writing.md")]): RulesSnapshot {
  return {
    rootPath: "C:/isolated/.skillport/rules",
    rules,
    targets: [
      { tool: "claude-code", path: "C:/isolated/.claude/rules", supported: true, errorCode: null },
      { tool: "omp", path: "C:/isolated/.omp/agent/rules", supported: true, errorCode: null },
    ],
    recoveryOperations: [],
  };
}

export function resetRulesStoreForTest(): void {
  useTargetStore.setState({ activeTarget: { id: "local", kind: "local", label: "Local", isActive: true } });
  useRulesStore.setState({ targetId: null, snapshot: null, selectedName: null, detail: null, drafts: {}, loading: false, detailLoading: false, saving: false, mutating: false, pendingTools: {}, importPreview: null, importResult: null, errorCode: null, requiresReload: false });
}

export function mockRulesLoaders(snapshot = rulesSnapshotFixture()): void {
  mockIpcCommands({
    list_rules: snapshot,
    read_rule: ({ name }: { name: string }) => snapshot.rules.find((rule) => rule.name === name),
  });
}
