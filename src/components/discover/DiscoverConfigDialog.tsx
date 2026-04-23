import { Radar, Loader2, AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { useDiscoverStore } from "@/stores/discoverStore";
import { usePlatformStore } from "@/stores/platformStore";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  filterVisiblePlatformAgents,
} from "@/lib/platformVisibility";
import { ScanRoot } from "@/types";

// ─── DiscoverConfigDialog ────────────────────────────────────────────────────

interface DiscoverConfigDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function DiscoverConfigDialog({ open, onOpenChange }: DiscoverConfigDialogProps) {
  const { t } = useTranslation();

  const scanRoots = useDiscoverStore((s) => s.scanRoots);
  const isLoadingRoots = useDiscoverStore((s) => s.isLoadingRoots);
  const loadScanRoots = useDiscoverStore((s) => s.loadScanRoots);
  const setScanRootEnabled = useDiscoverStore((s) => s.setScanRootEnabled);
  const startScan = useDiscoverStore((s) => s.startScan);

  const agents = usePlatformStore((s) => s.agents);
  const categoryVisibility = usePlatformStore((s) => s.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;

  // Load roots when dialog opens.
  const handleOpenChange = (open: boolean) => {
    if (open) {
      loadScanRoots();
    }
    onOpenChange(open);
  };

  // Get platform skill directory patterns for display.
  const visiblePlatformAgents = filterVisiblePlatformAgents(agents, categoryVisibility);
  const platformPatterns = visiblePlatformAgents
    .map((a) => {
      const rel = a.global_skills_dir.replace(
        /^.*\/(\.[\w-]+\/skills\/?)$/,
        "$1"
      );
      return { name: a.display_name, pattern: rel || a.global_skills_dir };
    });

  const enabledCount = scanRoots.filter((r) => r.enabled && r.exists).length;

  function handleStartScan() {
    // Close the dialog IMMEDIATELY so the user can see the ProgressView
    // with the Stop button. The scan runs asynchronously in the background;
    // errors are captured in the store's error state.
    onOpenChange(false);
    startScan();
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Radar className="size-5" />
            {t("discover.title")}
          </DialogTitle>
          <DialogDescription>{t("discover.desc")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* Scan Roots */}
          <div>
            <h3 className="text-sm font-medium mb-2">{t("discover.scanRoots")}</h3>
            <p className="text-xs text-muted-foreground mb-2">
              {t("discover.scanRootsDesc")}
            </p>

            {isLoadingRoots ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground py-4 justify-center">
                <Loader2 className="size-4 animate-spin" />
                <span>{t("common.loading")}</span>
              </div>
            ) : scanRoots.length === 0 ? (
              <p className="text-sm text-muted-foreground py-2 text-center">
                No candidate directories found.
              </p>
            ) : (
              <div className="space-y-2 max-h-48 overflow-y-auto">
                {scanRoots.map((root) => (
                  <ScanRootRow
                    key={root.path}
                    root={root}
                    onToggle={(enabled) =>
                      setScanRootEnabled(root.path, enabled)
                    }
                  />
                ))}
              </div>
            )}
          </div>

          {/* Platform Patterns */}
          <div>
            <h3 className="text-xs font-medium text-muted-foreground mb-1">
              {t("discover.lookingFor")}
            </h3>
            <div className="flex flex-wrap gap-1.5">
              {platformPatterns.slice(0, 6).map((p) => (
                <span
                  key={p.name}
                  className="text-xs px-2 py-0.5 rounded bg-muted text-muted-foreground font-mono"
                >
                  {p.pattern}
                </span>
              ))}
              {platformPatterns.length > 6 && (
                <span className="text-xs px-2 py-0.5 rounded bg-muted text-muted-foreground">
                  +{platformPatterns.length - 6}
                </span>
              )}
            </div>
          </div>

          {/* Warning if no roots enabled */}
          {enabledCount === 0 && !isLoadingRoots && (
            <div className="flex items-start gap-2 text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950/20 rounded-md p-2.5">
              <AlertTriangle className="size-4 shrink-0 mt-0.5" />
              <span>{t("discover.noRootsEnabled")}</span>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleStartScan}
            disabled={enabledCount === 0}
          >
            <Radar className="size-4 mr-1" />
            {t("discover.startScan")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ─── ScanRootRow ──────────────────────────────────────────────────────────────

function ScanRootRow({
  root,
  onToggle,
}: {
  root: ScanRoot;
  onToggle: (enabled: boolean) => void;
}) {
  return (
    <div className="flex items-center gap-2 py-1.5 px-2 rounded hover:bg-hover-bg/20 cursor-pointer">
      <Checkbox
        checked={root.enabled}
        onCheckedChange={(checked) => onToggle(!!checked)}
        disabled={!root.exists}
        aria-label={root.path}
      />
      <div className="flex-1 min-w-0">
        <span
          className={`text-sm font-mono truncate ${!root.exists ? "text-muted-foreground line-through" : ""}`}
        >
          {root.path}
        </span>
      </div>
      <span className="text-xs text-muted-foreground shrink-0">
        {root.label}
      </span>
    </div>
  );
}
