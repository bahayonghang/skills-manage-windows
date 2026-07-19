import { registerIpcFixtures } from "@/lib/ipc";
import type { TargetSummary, WslDistributionSummary } from "@/types";

const LOCAL_TARGET: TargetSummary = {
  id: "local",
  kind: "local",
  label: "Local",
  isActive: true,
};

const LONG_PATH_SSH_TARGET: TargetSummary = {
  id: "ssh-browser-long-path",
  kind: "ssh",
  label: "Windows typography validation workstation with a long label",
  host: "typography-validation.internal.example",
  username: "fixture-user",
  port: 22,
  authMethod: "key",
  remoteHome:
    "C:\\Users\\fixture-user\\Documents\\SkillPort Validation Workspace\\deeply-nested-project\\.agents\\skills",
  remoteOs: "Windows 11",
  credentialStatus: "stored",
  isActive: false,
};

const BROWSER_FIXTURE_WSL_DISTRIBUTIONS: WslDistributionSummary[] = [
  {
    name: "Ubuntu-24.04",
    isDefault: true,
    state: "Stopped",
    version: "2",
  },
];

const SSH_ONLY_ERROR = "Remote targets are available only in the Tauri app.";
const WSL_ONLY_ERROR = "WSL targets are available only in the Tauri app.";

export function registerTargetFixtures(): void {
  registerIpcFixtures({
    list_targets: () => [LOCAL_TARGET, LONG_PATH_SSH_TARGET],
    list_wsl_distributions: () => BROWSER_FIXTURE_WSL_DISTRIBUTIONS,
    // 建远程目标依赖真实 SSH/WSL，浏览器演示态保持「仅桌面可用」语义
    create_ssh_target: () => Promise.reject(SSH_ONLY_ERROR),
    update_ssh_target: () => Promise.reject(SSH_ONLY_ERROR),
    create_wsl_target: () => Promise.reject(WSL_ONLY_ERROR),
    update_wsl_target: () => Promise.reject(WSL_ONLY_ERROR),
    test_ssh_target: () => ({
      ok: true,
      remoteHome: "/home/fixture",
      remoteOs: "Linux",
      message: "Browser fixture target is reachable.",
    }),
    test_wsl_target: () => ({
      ok: true,
      remoteHome: "/home/fixture",
      remoteOs: "Linux",
      message: "Browser fixture WSL target is reachable.",
    }),
    update_ssh_target_password: () => ({
      ok: true,
      remoteHome: "/home/fixture",
      remoteOs: "Linux",
      message: "Browser fixture target password is saved.",
    }),
    delete_target: () => undefined,
    set_active_target: () => LOCAL_TARGET,
  });
}
