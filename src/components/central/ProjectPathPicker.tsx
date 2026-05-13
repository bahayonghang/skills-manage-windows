import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface ProjectPathPickerProps {
  value: string;
  onChange: (value: string) => void;
  onError: (message: string | null) => void;
  disabled?: boolean;
}

export function ProjectPathPicker({
  value,
  onChange,
  onError,
  disabled = false,
}: ProjectPathPickerProps) {
  const { t } = useTranslation();
  const [isPicking, setIsPicking] = useState(false);

  async function handleBrowseProjectPath() {
    setIsPicking(true);
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        defaultPath: value.trim() || undefined,
        canCreateDirectories: true,
      });

      if (typeof selectedPath === "string") {
        onChange(selectedPath);
        onError(null);
      }
    } catch (err) {
      onError(
        t("central.batchInstallProjectPathPickerError", {
          error: String(err),
        })
      );
    } finally {
      setIsPicking(false);
    }
  }

  return (
    <div className="flex gap-2">
      <Input
        value={value}
        onChange={(event) => {
          onChange(event.target.value);
          onError(null);
        }}
        placeholder={t("central.batchInstallProjectPathPlaceholder")}
      />
      <Button
        type="button"
        variant="outline"
        onClick={handleBrowseProjectPath}
        disabled={disabled || isPicking}
        aria-label={t("central.batchInstallProjectPathBrowseAria")}
      >
        <FolderOpen className="size-3.5" aria-hidden="true" />
        <span className="hidden sm:inline">
          {t("central.batchInstallProjectPathBrowse")}
        </span>
      </Button>
    </div>
  );
}
