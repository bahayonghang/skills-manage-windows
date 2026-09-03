export type UpdateCheckMode = "regular" | "sync";

export const CENTRAL_UPDATE_CHECK_MODE_SETTING_KEY = "central_update_check_mode_v1";
export const DEFAULT_UPDATE_CHECK_MODE: UpdateCheckMode = "regular";

export function normalizeUpdateCheckMode(value: unknown): UpdateCheckMode {
  return value === "sync" ? "sync" : DEFAULT_UPDATE_CHECK_MODE;
}
