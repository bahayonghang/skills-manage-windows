export type TargetKind = "local" | "ssh";
export type SshAuthMethod = "key" | "password";
export type TargetCredentialStatus = "stored" | "session" | "missing" | "unreadable";
export type SecretStorageState = "stored" | "session" | "missing" | "unreadable";

export interface TargetSummary {
  id: string;
  kind: TargetKind;
  label: string;
  host?: string | null;
  username?: string | null;
  port?: number | null;
  authMethod?: SshAuthMethod | null;
  keyPath?: string | null;
  remoteHome?: string | null;
  remoteOs?: string | null;
  cacheDbPath?: string | null;
  hasStoredPassword?: boolean | null;
  credentialStatus?: TargetCredentialStatus | null;
  credentialError?: string | null;
  symlinkEnabled?: boolean | null;
  isActive: boolean;
}

export interface CreateSshTargetRequest {
  label: string;
  host: string;
  username: string;
  port?: number | null;
  authMethod?: SshAuthMethod | null;
  keyPath?: string | null;
  password?: string | null;
  passphrase?: string | null;
}

export interface UpdateSshTargetRequest {
  id: string;
  label: string;
  host: string;
  username: string;
  port?: number | null;
  authMethod?: SshAuthMethod | null;
  keyPath?: string | null;
  password?: string | null;
  passphrase?: string | null;
}

export interface TestSshTargetRequest {
  id?: string | null;
  label?: string | null;
  host?: string | null;
  username?: string | null;
  port?: number | null;
  authMethod?: SshAuthMethod | null;
  keyPath?: string | null;
  password?: string | null;
  passphrase?: string | null;
}

export interface SshTargetTestResult {
  ok: boolean;
  remoteHome?: string | null;
  remoteOs?: string | null;
  credentialStatus?: TargetCredentialStatus | null;
  credentialError?: string | null;
  message: string;
}

export interface GitHubPatTestResult {
  configured: boolean;
  ok: boolean;
  status?: number | null;
  messageKey: string;
  message: string;
}

export interface GitHubPatState {
  configured: boolean;
  storageState: SecretStorageState;
  error?: string | null;
}

export interface AiApiKeyState {
  provider?: string | null;
  configured: boolean;
  storageState: SecretStorageState;
  fingerprint?: string | null;
  error?: string | null;
}
