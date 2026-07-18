import { FolderOpen, Loader2, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { Switch } from "@/components/ui/switch";
import { formatPathForDisplay } from "@/lib/path";
import type { ScanDirectory } from "@/types";

interface ScanDirectoriesSettingsSectionProps {
  isLoadingScanDirs: boolean;
  removingDir: string | null;
  scanDirError: string | null;
  scanDirectories: ScanDirectory[];
  showBuiltinDirs: boolean;
  onAddDirectory: () => void;
  onRemoveDirectory: (path: string) => void;
  onToggleBuiltinDirs: () => void;
  onToggleDirectory: (path: string, active: boolean) => void;
}

export function ScanDirectoriesSettingsSection({
  isLoadingScanDirs,
  removingDir,
  scanDirError,
  scanDirectories,
  showBuiltinDirs,
  onAddDirectory,
  onRemoveDirectory,
  onToggleBuiltinDirs,
  onToggleDirectory,
}: ScanDirectoriesSettingsSectionProps) {
  const { t } = useTranslation();
  const customDirs = scanDirectories.filter((dir) => !dir.is_builtin);
  const builtinDirs = scanDirectories.filter((dir) => dir.is_builtin);

  return (
    <SettingsSection
      sectionId="scan-directories"
      title={t("settings.scanDirs")}
      description={t("settings.scanDirsDesc")}
      icon={<FolderOpen className="size-5 shrink-0 text-muted-foreground" />}
      action={
        <Button
          variant="outline"
          size="sm"
          onClick={onAddDirectory}
          aria-label={t("settings.addDirAriaLabel")}
        >
          <Plus className="size-3.5" />
          <span>{t("settings.addDirectory")}</span>
        </Button>
      }
    >
        {scanDirError && (
          <p className="text-xs text-destructive-text mb-3" role="alert">
            {scanDirError}
          </p>
        )}
        {isLoadingScanDirs ? (
          <div className="flex items-center gap-2 py-4 text-muted-foreground text-sm justify-center">
            <Loader2 className="size-4 animate-spin" />
            <span>{t("settings.loading")}</span>
          </div>
        ) : (
          <div className="space-y-3">
            {customDirs.length > 0 && (
              <div className="rounded-lg border border-border overflow-hidden">
                {customDirs.map((dir) => (
                  <ScanDirectoryRow
                    key={dir.id}
                    dir={dir}
                    onRemove={() => onRemoveDirectory(dir.path)}
                    onToggle={(active) => onToggleDirectory(dir.path, active)}
                    isRemoving={removingDir === dir.path}
                  />
                ))}
              </div>
            )}
            {customDirs.length === 0 && (
              <p className="text-xs text-muted-foreground text-center py-2">
                {t("settings.noDirs")}
              </p>
            )}
            {builtinDirs.length > 0 && (
              <div>
                <button
                  onClick={onToggleBuiltinDirs}
                  className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                >
                  <span>{showBuiltinDirs ? "▾" : "▸"}</span>
                  <span>{t("settings.builtinDir")} ({builtinDirs.length})</span>
                </button>
                {showBuiltinDirs && (
                  <div className="grid grid-cols-2 gap-1.5 mt-2">
                    {builtinDirs.map((dir) => (
                      <div
                        key={dir.id}
                        className="flex items-center gap-2 px-2.5 py-1.5 rounded-md bg-muted/30 text-xs text-muted-foreground truncate"
                      >
                        <FolderOpen className="size-3 shrink-0" />
                        <span className="truncate">{formatPathForDisplay(dir.path)}</span>
                        {dir.label && <span className="shrink-0 opacity-60">· {dir.label}</span>}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}
    </SettingsSection>
  );
}

interface ScanDirectoryRowProps {
  dir: ScanDirectory;
  onRemove: () => void;
  onToggle: (active: boolean) => void;
  isRemoving: boolean;
}

function ScanDirectoryRow({
  dir,
  onRemove,
  onToggle,
  isRemoving,
}: ScanDirectoryRowProps) {
  const { t } = useTranslation();
  const action = dir.is_active ? t("settings.enabled") : t("settings.disabled");

  return (
    <div className="flex items-center gap-3 py-2.5 px-4 border-b border-border/50 last:border-0">
      <FolderOpen className="size-4 text-muted-foreground shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium truncate">{formatPathForDisplay(dir.path)}</div>
        {dir.label && (
          <div className="text-xs text-muted-foreground mt-0.5">{dir.label}</div>
        )}
        {dir.is_builtin && (
          <div className="text-xs text-muted-foreground mt-0.5">{t("settings.builtinDir")}</div>
        )}
      </div>
      <div className="flex items-center gap-2 shrink-0">
        {!dir.is_builtin && (
          <div className="flex items-center gap-1.5">
            <span className="text-xs text-muted-foreground">
              {action}
            </span>
            <Switch
              checked={dir.is_active}
              onCheckedChange={onToggle}
              aria-label={t("settings.enableDirLabel", { action, path: dir.path })}
            />
          </div>
        )}
        {!dir.is_builtin && (
          <InlineConfirmAction
            onConfirm={onRemove}
            isLoading={isRemoving}
            idleAriaLabel={t("settings.removeDirLabel", { path: dir.path })}
            idleTitle={t("settings.removeDirLabel", { path: dir.path })}
            confirmLabel={t("common.confirmDelete")}
            icon={<Trash2 className="size-3.5" />}
          />
        )}
      </div>
    </div>
  );
}
