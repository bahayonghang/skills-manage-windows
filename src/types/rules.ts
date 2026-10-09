import type { CommandResult } from "@/lib/ipc";

export type RulesSnapshot = CommandResult<"list_rules">;
export type RuleSummary = RulesSnapshot["rules"][number];
export type RuleDetail = CommandResult<"read_rule">;
export type RuleTargetStatus = RuleDetail["targets"][number];
export type RuleTool = RuleTargetStatus["tool"];
export type RulesImportPreview = CommandResult<"preview_rules_import">;
export type RulesImportResult = CommandResult<"import_existing_rules">;
export type RuleRecoveryOperation = RulesSnapshot["recoveryOperations"][number];

export interface RuleDraft {
  body: string;
  baseBody: string;
  revision: string;
}
