import type { SshAuthMethod, WslDistributionSummary } from "@/types";

export type SshTargetFormState = {
  label: string;
  host: string;
  username: string;
  port: string;
  authMethod: SshAuthMethod;
  keyPath: string;
  password: string;
};

export type WslTargetFormState = {
  label: string;
  distribution: string;
};

export type TargetMessage = { type: "success" | "error"; text: string } | null;

export type WslDistributionOption = WslDistributionSummary;
