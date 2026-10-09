import i18n from "@/i18n";
import { ipcFixtureError, registerIpcFixtures } from "@/lib/ipc";
import type { RuleDetail, RuleTargetStatus, RuleTool, RulesImportPreview, RulesSnapshot } from "@/types/rules";

const rootPath = "C:/fixture-home/.skillport/rules";
const toolPaths: Record<RuleTool, string> = {
  "claude-code": "C:/fixture-home/.claude/rules",
  omp: "C:/fixture-home/.omp/agent/rules",
};
let revision = 0;
let rules: RuleDetail[] = [];

function local(targetId: string) {
  if (targetId !== "local") throw ipcFixtureError("rules.local_only", "Local rules only");
}

function detail(name: string): RuleDetail {
  const rule = rules.find((item) => item.name === name);
  if (!rule) throw ipcFixtureError("rules.not_found", "Rule not found");
  return rule;
}

function buildRule(name: string, description: string, body: string): RuleDetail {
  revision += 1;
  return {
    name,
    title: body.match(/^#\s+(.+)$/m)?.[1] ?? name,
    description,
    body,
    source: `---\nalwaysApply: true\ndescription: ${JSON.stringify(description)}\n---\n${body}`,
    bytes: new TextEncoder().encode(body).length,
    revision: `fixture-revision-${revision}`,
    compatibility: "supported",
    targets: (["claude-code", "omp"] as const).map((tool) => ({ tool, path: `${toolPaths[tool]}/${name}`, state: "absent", fingerprint: null, errorCode: null })),
  };
}

function snapshot(): RulesSnapshot {
  return structuredClone({ rootPath, rules, targets: (["claude-code", "omp"] as const).map((tool) => ({ tool, path: toolPaths[tool], supported: true, errorCode: null })), recoveryOperations: [] });
}

function sourcePreview(): RulesImportPreview {
  const names = ["review-checklist.md", "writing-style.md", "safety-basics.md", "project-context.md", "conditional-example.md", "source-conflict.md"];
  return {
    entries: names.map((name) => ({
      name,
      status: name === "conditional-example.md" ? "unsupported" : name === "source-conflict.md" ? "conflict" : rules.some((rule) => rule.name === name) ? "same" : "new",
      description: i18n.t("rules.fixture.description"),
      sourceFingerprints: (["claude-code", "omp"] as const).map((tool) => ({ tool, fingerprint: `fixture-source-${tool}-${name}` })),
      errorCode: name === "conditional-example.md" ? "rules.unsupported" : name === "source-conflict.md" ? "rules.target_conflict" : null,
    })),
  };
}

export function resetRulesFixturesForTest(): void {
  revision = 0;
  rules = [
    buildRule("review-checklist.md", i18n.t("rules.fixture.description"), i18n.t("rules.fixture.reviewBody")),
    buildRule("writing-style.md", i18n.t("rules.fixture.description"), i18n.t("rules.fixture.writingBody")),
    buildRule("safety-basics.md", i18n.t("rules.fixture.description"), i18n.t("rules.fixture.safetyBody")),
  ];
  rules[0].targets[0].state = "linked";
  rules[0].targets[1].state = "native_equivalent";
  rules[0].targets[1].fingerprint = "fixture-equivalent-omp";
  rules[2].targets[1].state = "conflict";
  rules[2].targets[1].errorCode = "rules.target_conflict";
}

export function registerRulesFixtures(): void {
  if (!rules.length) resetRulesFixturesForTest();
  registerIpcFixtures({
    list_rules: ({ targetId }) => { local(targetId); return snapshot(); },
    read_rule: ({ targetId, name }) => { local(targetId); return structuredClone(detail(name)); },
    preview_rules_import: ({ targetId }) => { local(targetId); return sourcePreview(); },
    import_existing_rules: ({ targetId, entries }) => {
      local(targetId);
      const preview = sourcePreview();
      return {
        operationId: "fixture-import-operation",
        entries: entries.map((entry) => {
          const source = preview.entries.find((item) => item.name === entry.name);
          if (!source || source.status === "unsupported" || source.status === "conflict") return { name: entry.name, status: "conflict" as const, errorCode: "rules.target_conflict" };
          if (JSON.stringify(source.sourceFingerprints) !== JSON.stringify(entry.sourceFingerprints)) return { name: entry.name, status: "failed" as const, errorCode: "rules.revision_conflict" };
          if (rules.some((rule) => rule.name === entry.name)) return { name: entry.name, status: "same" as const, errorCode: null };
          rules.push(buildRule(entry.name, source.description, i18n.t("rules.fixture.contextBody")));
          return { name: entry.name, status: "imported" as const, errorCode: null };
        }),
      };
    },
    create_rule: ({ targetId, name, description, body }) => {
      local(targetId);
      if (!/^[^\\/]+\.md$/i.test(name) || name.includes("..") || /^rules\.md$/i.test(name)) throw ipcFixtureError("rules.invalid_name", "Invalid file name");
      if (rules.some((rule) => rule.name.toLocaleLowerCase() === name.toLocaleLowerCase())) throw ipcFixtureError("rules.target_conflict", "File exists");
      const next = buildRule(name, description, body);
      rules.push(next);
      return structuredClone(next);
    },
    save_rule: ({ targetId, name, body, expectedRevision }) => {
      local(targetId);
      const previous = detail(name);
      if (previous.revision !== expectedRevision) throw ipcFixtureError("rules.revision_conflict", "Revision changed");
      const next = { ...buildRule(name, previous.description, body), targets: previous.targets };
      rules = rules.map((rule) => rule.name === name ? next : rule);
      return structuredClone(next);
    },
    set_rule_target_enabled: ({ targetId, name, tool, enabled, expectedDestinationFingerprint }) => {
      local(targetId);
      const rule = detail(name);
      const target = rule.targets.find((item) => item.tool === tool)!;
      if (target.state === "conflict" || (target.state === "native_equivalent" && target.fingerprint !== expectedDestinationFingerprint)) throw ipcFixtureError("rules.target_conflict", "Target conflict");
      const next: RuleTargetStatus = { ...target, state: enabled ? "linked" : "absent", fingerprint: enabled ? `fixture-link-${tool}-${name}` : null, errorCode: null };
      rule.targets = rule.targets.map((item) => item.tool === tool ? next : item);
      return structuredClone(rule);
    },
    delete_rule: ({ targetId, name, expectedRevision }) => {
      local(targetId);
      const rule = detail(name);
      if (rule.revision !== expectedRevision) throw ipcFixtureError("rules.revision_conflict", "Revision changed");
      if (rule.targets.some((target) => target.state === "conflict")) throw ipcFixtureError("rules.target_conflict", "Target conflict");
      rules = rules.filter((item) => item.name !== name);
      return { name, operationId: "fixture-delete-operation", recoveryRequired: false };
    },
    recover_rule_operation: ({ targetId }) => { local(targetId); throw ipcFixtureError("rules.recovery_blocked", "No fixture operation exists"); },
  });
}
